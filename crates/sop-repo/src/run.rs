//! Executing a run: start it, append events, recover it, attach logs.
//!
//! A run lives in `runs/<sop_id>/<run_id>/` with three files and a logs directory:
//!
//! - `events.jsonl` - the append-only event log, the source of truth (`DESIGN.md` 6.2)
//! - `snapshot.md`  - the checklist frozen at start, so the record survives content edits
//! - `record.md`    - the rendered human review, regenerated after every event
//! - `logs/`        - attachments, copied in and hashed
//!
//! The human-reviewable record file `runs/<sop_id>/<run_id>.md` is written from
//! `record.md` at the end of the run; the run directory holds the working log.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sop_core::run::{render_record, RunEvent, RunState, RecordStep, StepState};
use sop_core::Document;
use thiserror::Error;

use crate::atomic;
use crate::git;
use crate::{Repo, resolve_checklist};

#[derive(Debug, Error)]
pub enum RunError {
    #[error("{0}")]
    Core(#[from] sop_core::RunError),
    #[error("{0}")]
    Repo(#[from] crate::RepoError),
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("the checklist '{0}' does not exist; checked checklists/ for a file named {0}.md")]
    NoChecklist(String),
    #[error("checklist '{0}' is a draft and cannot start a run without an override reason")]
    Draft(String),
    #[error("a run named '{0}' already exists for checklist '{1}'")]
    RunExists(String, String),
    #[error("the run '{1}' for checklist '{0}' does not exist")]
    NoRun(String, String),
    #[error("snapshot.md for run '{1}' could not be parsed: {0}")]
    BadSnapshot(String, String),
    #[error("event {line} in the run log is not a well-formed event")]
    BadEvent { line: usize },
}

/// The events and the state they produce, for one run.
#[derive(Debug)]
pub struct LoadedRun {
    pub state: RunState,
    /// The checklist as frozen at start, for rendering the record.
    pub steps: Vec<RecordStep>,
    /// The step definitions as frozen at start, for the app's execution view.
    pub defs: Vec<ResolvedStepDef>,
    pub snapshot_text: String,
    pub dir: PathBuf,
    pub events_path: PathBuf,
    pub record_path: PathBuf,
    /// The human record file `runs/<sop_id>/<run_id>.md`, written on end.
    pub record_file: PathBuf,
}

/// A step as it was at run start: definitions and prose, independent of the run state.
#[derive(Debug, Clone)]
pub struct ResolvedStepDef {
    pub id: String,
    pub title: String,
    pub prose: String,
    pub severity: Option<String>,
    pub kind: Option<String>,
    pub captures: Vec<CaptureDef>,
}

#[derive(Debug, Clone)]
pub struct CaptureDef {
    pub key: String,
    pub label: Option<String>,
    pub capture_type: Option<String>,
    pub unit: Option<String>,
    pub required: bool,
    pub options: Vec<String>,
    pub expected: Option<serde_json::Value>,
}

fn run_dir(repo: &Repo, sop_id: &str, run_id: &str) -> PathBuf {
    repo.resolve(&format!("runs/{sop_id}/{run_id}"))
}

/// Freeze a resolved checklist into `snapshot.md` text and the step shapes for replay.
fn snapshot_of(resolved: &[sop_core::Step]) -> (String, Vec<RecordStep>) {
    let mut text = String::from("---\nkind: snapshot\n---\n\n");
    let mut record_steps = Vec::new();
    for step in resolved {
        let title = step.title.clone().unwrap_or_else(|| step.id_or_placeholder().to_owned());
        text.push_str(&format!("## {title}\n\n"));
        if let Some(id) = &step.id {
            text.push_str("```yaml step\n");
            if let Ok(yaml) = serde_norway::to_string(&step.raw) {
                text.push_str(&yaml);
            }
            text.push_str("```\n\n");
            let _ = id;
        }
        text.push_str(&step.prose);
        text.push_str("\n\n");

        let checkbox_count = step
            .prose
            .lines()
            .filter(|line| line.trim_start().starts_with("- [") || line.trim_start().starts_with("* ["))
            .count();
        record_steps.push(RecordStep {
            id: step.id.clone().unwrap_or_default(),
            title,
            body: step.prose.clone(),
            checkbox_count,
        });
    }
    (text, record_steps)
}

/// Recover the step shapes from a saved snapshot, so replay can bound checkbox indexes.
///
/// Returns both the shapes the record renderer needs and the fuller definitions the
/// app's execution view needs (capture types, units, expected ranges).
fn seed_steps_from_snapshot(
    snapshot_text: &str,
) -> Result<(Vec<RecordStep>, Vec<ResolvedStepDef>), RunError> {
    let doc = Document::parse(snapshot_text)
        .map_err(|error| RunError::BadSnapshot(snapshot_text.to_owned(), error.to_string()))?;
    let mut shapes = Vec::new();
    let mut defs = Vec::new();
    for step in &doc.steps {
        let checkbox_count = step
            .prose
            .lines()
            .filter(|line| line.trim_start().starts_with("- [") || line.trim_start().starts_with("* ["))
            .count();
        shapes.push(RecordStep {
            id: step.id.clone().unwrap_or_default(),
            title: step.title.clone().unwrap_or_else(|| step.id_or_placeholder().to_owned()),
            body: step.prose.clone(),
            checkbox_count,
        });
        let captures = step
            .captures
            .iter()
            .map(|capture| {
                let raw = crate::manifest::json_safe(&sop_core::Value::Mapping(capture.raw.clone()));
                CaptureDef {
                    key: capture.key.clone().unwrap_or_default(),
                    label: capture.label.clone(),
                    capture_type: capture.capture_type.as_ref().map(|kind| kind.as_str().to_owned()),
                    unit: capture.unit.clone(),
                    required: capture.required.unwrap_or(false),
                    options: capture.options.clone(),
                    expected: raw.get("expected").cloned(),
                }
            })
            .collect();
        defs.push(ResolvedStepDef {
            id: step.id.clone().unwrap_or_default(),
            title: step.title.clone().unwrap_or_else(|| step.id_or_placeholder().to_owned()),
            prose: step.prose.clone(),
            severity: step.severity.clone(),
            kind: step.key.as_ref().map(|key| key.as_str().to_owned()),
            captures,
        });
    }
    Ok((shapes, defs))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now() -> String {
    sop_core::now_utc_rfc3339()
}

fn write_events(path: &Path, events: &[RunEvent]) -> Result<(), RunError> {
    let mut text = String::new();
    for event in events {
        let line = serde_json::to_string(event).map_err(|error| {
            RunError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::other(error.to_string()),
            }
        })?;
        text.push_str(&line);
        text.push('\n');
    }
    atomic::write(path, &text).map_err(|error| RunError::Io {
        path: path.to_path_buf(),
        source: error.source,
    })?;
    Ok(())
}

/// Load a run: read the log, seed step shapes from the snapshot, replay to a state.
pub fn load(repo: &Repo, sop_id: &str, run_id: &str) -> Result<LoadedRun, RunError> {
    let dir = run_dir(repo, sop_id, run_id);
    let events_path = dir.join("events.jsonl");
    let snapshot_path = dir.join("snapshot.md");
    let record_path = dir.join("record.md");
    let record_file = repo.resolve(&format!("runs/{sop_id}/{run_id}.md"));
    if !dir.is_dir() {
        return Err(RunError::NoRun(sop_id.to_owned(), run_id.to_owned()));
    }

    let snapshot_text = repo
        .read_text(&snapshot_path)
        .map_err(RunError::Repo)?;
    let (seed, defs) = seed_steps_from_snapshot(&snapshot_text)?;

    let mut state = RunState::default();
    for step in &seed {
        state.steps.entry(step.id.clone()).or_insert_with(|| StepState {
            checkboxes: vec![false; step.checkbox_count],
            ..StepState::default()
        });
    }

    let bytes = fs::read(&events_path).map_err(|source| RunError::Io {
        path: events_path.clone(),
        source,
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let mut events = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<RunEvent>(line) {
            Ok(event) => events.push(event),
            // A torn final line from a crash is ignored; the log up to it is authoritative.
            Err(_) if index + 1 == text.lines().count() => break,
            Err(_) => return Err(RunError::BadEvent { line: index + 1 }),
        }
    }
    for event in &events {
        sop_core::run::apply(&mut state, event).map_err(RunError::Core)?;
    }

    Ok(LoadedRun {
        state,
        steps: seed,
        defs,
        snapshot_text,
        dir,
        events_path,
        record_path,
        record_file,
    })
}

fn write_record(loaded: &LoadedRun) -> Result<(), RunError> {
    let rendered = render_record(&loaded.state, &loaded.steps);
    atomic::write(&loaded.record_path, &rendered).map_err(|error| RunError::Io {
        path: loaded.record_path.clone(),
        source: error.source,
    })?;
    Ok(())
}

/// Start a run from a checklist, freezing a snapshot and opening the event log.
///
/// `override_reason` lets a `draft` checklist start; the reason is recorded as the
/// opening event so it cannot be forgotten (`FEATURES.md` B: "Block starting a draft").
pub fn start(
    repo: &Repo,
    sop_id: &str,
    run_id: &str,
    operator: &str,
    site: &str,
    override_reason: Option<&str>,
) -> Result<LoadedRun, RunError> {
    let checklist_path = repo.resolve(&format!("checklists/{sop_id}.md"));
    if !checklist_path.is_file() {
        return Err(RunError::NoChecklist(sop_id.to_owned()));
    }
    let loaded = repo.load(&checklist_path).map_err(RunError::Repo)?;
    let resolved = resolve_checklist(repo, &loaded);
    let status = loaded.doc.front.str("status").flatten();
    if status == Some("draft") && override_reason.is_none() {
        return Err(RunError::Draft(sop_id.to_owned()));
    }

    let dir = run_dir(repo, sop_id, run_id);
    if dir.is_dir() {
        return Err(RunError::RunExists(run_id.to_owned(), sop_id.to_owned()));
    }

    let (snapshot_text, _record_steps) = snapshot_of(&resolved.steps);
    let snapshot_sha256 = sha256_hex(snapshot_text.as_bytes());
    let sop_version = loaded
        .doc
        .front
        .str("version")
        .flatten()
        .unwrap_or("1");
    let sop_commit = git::current_commit(repo.root()).unwrap_or_else(|| "unknown".to_owned());

    fs::create_dir_all(dir.join("logs")).map_err(|source| RunError::Io {
        path: dir.clone(),
        source,
    })?;
    atomic::write(&dir.join("snapshot.md"), &snapshot_text).map_err(|error| RunError::Io {
        path: dir.join("snapshot.md"),
        source: error.source,
    })?;

    let mut events = vec![RunEvent::RunStarted {
        at: now(),
        sop: sop_id.to_owned(),
        sop_version: sop_version.to_owned(),
        sop_commit,
        snapshot_sha256,
        operator: operator.to_owned(),
        site: site.to_owned(),
    }];
    if let Some(reason) = override_reason {
        events.push(RunEvent::NoteAdded {
            at: now(),
            step: None,
            text: format!("run started over a draft checklist override: {reason}"),
        });
    }
    write_events(&dir.join("events.jsonl"), &events)?;

    let loaded_run = load(repo, sop_id, run_id)?;
    write_record(&loaded_run)?;
    Ok(loaded_run)
}

/// Append one event to the run and regenerate the record.
pub fn record(repo: &Repo, sop_id: &str, run_id: &str, event: &RunEvent) -> Result<LoadedRun, RunError> {
    let loaded = load(repo, sop_id, run_id)?;
    let mut next = loaded.state.clone();
    sop_core::run::apply(&mut next, event).map_err(RunError::Core)?;
    let mut events = Vec::new();
    {
        let bytes = fs::read(&loaded.events_path).map_err(|source| RunError::Io {
            path: loaded.events_path.clone(),
            source,
        })?;
        for line in String::from_utf8_lossy(&bytes).lines() {
            if let Ok(event) = serde_json::from_str::<RunEvent>(line) {
                events.push(event);
            }
        }
    }
    events.push(event.clone());
    write_events(&loaded.events_path, &events)?;

    let reloaded = load(repo, sop_id, run_id)?;
    write_record(&reloaded)?;
    Ok(reloaded)
}

/// Copy a file into the run's logs, hash it, and record an event for it.
pub fn attach(
    repo: &Repo,
    sop_id: &str,
    run_id: &str,
    step: Option<&str>,
    source: &Path,
) -> Result<sop_core::run::Attachment, RunError> {
    let loaded = load(repo, sop_id, run_id)?;
    let bytes = fs::read(source).map_err(|error| RunError::Io {
        path: source.to_path_buf(),
        source: error,
    })?;
    let digest = sha256_hex(&bytes);
    let name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "attachment".to_owned());
    let logs_dir = loaded.dir.join("logs");
    fs::create_dir_all(&logs_dir).map_err(|source| RunError::Io {
        path: logs_dir.clone(),
        source,
    })?;
    let dest = logs_dir.join(format!("{digest}-{name}"));
    let mut file = fs::File::create(&dest).map_err(|error| RunError::Io {
        path: dest.clone(),
        source: error,
    })?;
    file.write_all(&bytes).map_err(|error| RunError::Io {
        path: dest.clone(),
        source: error,
    })?;

    let attachment = sop_core::run::Attachment {
        path: dest.display().to_string(),
        sha256: digest,
        size: bytes.len() as u64,
    };
    let event = RunEvent::AttachmentAdded {
        at: now(),
        step: step.map(str::to_owned),
        path: attachment.path.clone(),
        sha256: attachment.sha256.clone(),
        size: attachment.size,
    };
    record(repo, sop_id, run_id, &event)?;
    Ok(attachment)
}

/// The front matter for the run record file, so the record is a valid `runs/...` file.
fn run_file_front(state: &RunState, run_id: &str) -> String {
    let mut out = String::from("---\nkind: run\n");
    out.push_str(&format!("run_id: {run_id}\n"));
    if let Some(value) = &state.sop { out.push_str(&format!("sop: {value}\n")); }
    if let Some(value) = &state.sop_version { out.push_str(&format!("sop_version: {value}\n")); }
    if let Some(value) = &state.sop_commit { out.push_str(&format!("sop_commit: {value}\n")); }
    if let Some(value) = &state.operator { out.push_str(&format!("operator: {value}\n")); }
    if let Some(value) = &state.site { out.push_str(&format!("site: {value}\n")); }
    if let Some(value) = &state.started { out.push_str(&format!("started: {value}\n")); }
    if let Some(value) = &state.ended { out.push_str(&format!("ended: {value}\n")); }
    if let Some(value) = &state.run_status { out.push_str(&format!("status: {value}\n")); }
    out.push_str(&format!("deviations_count: {}\n", state.deviations()));
    out.push_str("---\n\n");
    out
}

/// The run record file: front matter plus the rendered record. This is the file the
/// validator and the manifest read, so it has to be a well-formed `runs/...` record.
fn run_record_text(state: &RunState, steps: &[RecordStep], run_id: &str) -> String {
    let mut out = run_file_front(state, run_id);
    out.push_str(&render_record(state, steps));
    out
}

/// End the run, writing the human record file next to the run directory.
pub fn end(repo: &Repo, sop_id: &str, run_id: &str, status: &str) -> Result<LoadedRun, RunError> {
    let event = RunEvent::RunEnded {
        at: now(),
        status: status.to_owned(),
    };
    let loaded = record(repo, sop_id, run_id, &event)?;
    let text = run_record_text(&loaded.state, &loaded.steps, run_id);
    atomic::write(&loaded.record_file, &text).map_err(|error| RunError::Io {
        path: loaded.record_file.clone(),
        source: error.source,
    })?;
    Ok(loaded)
}