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
use sop_core::run::{
    render_record, Attachment, RecordStep, RunEvent, RunState, SensorIdentity, StepState,
};
use sop_core::Document;
use thiserror::Error;

use std::collections::BTreeMap;

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
    #[error("'{0}' is not a well-formed run id, so it does not name a run")]
    BadRunId(String),
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

/// The facts about the instrument and the conditions that are not in the checklist.
///
/// Everything here is optional. A run started without them gets the same record as
/// before this existed, minus the empty blocks; the fields are additive in the event
/// log, so a log written by an older build still replays (`SPEC-COMPAT.md`).
#[derive(Debug, Clone, Default)]
pub struct RunMeta {
    pub sensor: Option<SensorIdentity>,
    pub hardware: Vec<String>,
    pub conditions: BTreeMap<String, String>,
}

impl RunMeta {
    /// True when there is nothing worth writing, so a caller can skip empty blocks.
    pub fn is_empty(&self) -> bool {
        self.sensor.as_ref().is_none_or(SensorIdentity::is_empty)
            && self.hardware.is_empty()
            && self.conditions.is_empty()
    }
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

/// The key a run is filed under: the checklist's own `sop_id`.
///
/// A run can be named by a checklist id (`ground-walk-survey`) or by the path to the
/// checklist file, which is how the app runs an external "Open file" checklist. A path's
/// file stem is not the checklist's id - the id lives in the file's front matter - so the
/// run directory, the record file, and the History view's grouping all have to come from
/// this one answer. Deriving them differently put the in-progress record in
/// `runs/<stem>/` while the run lived in `runs/<sop_id>/`, and History listed the run
/// twice.
fn run_key(repo: &Repo, sop_id: &str) -> String {
    let path = Path::new(sop_id);
    if path.is_file() {
        if let Some(id) = repo
            .load(path)
            .ok()
            .and_then(|loaded| loaded.doc.front.str("sop_id").flatten().map(str::to_owned))
        {
            return id;
        }
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| sop_id.to_owned())
    } else {
        sop_id.to_owned()
    }
}

fn run_dir(repo: &Repo, sop_id: &str, run_id: &str) -> PathBuf {
    let key = run_key(repo, sop_id);
    repo.resolve(&format!("runs/{key}/{run_id}"))
}

/// The committed record file `runs/<sop_id>/<run_id>.md`, next to the run directory.
fn committed_record_file(repo: &Repo, sop_id: &str, run_id: &str) -> PathBuf {
    let key = run_key(repo, sop_id);
    repo.resolve(&format!("runs/{key}/{run_id}.md"))
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
    let record_file = committed_record_file(repo, sop_id, run_id);
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
    start_with_meta(repo, sop_id, run_id, operator, site, override_reason, &RunMeta::default())
}

/// [`start`] with the instrument and conditions the operator recorded up front.
pub fn start_with_meta(
    repo: &Repo,
    sop_id: &str,
    run_id: &str,
    operator: &str,
    site: &str,
    override_reason: Option<&str>,
    meta: &RunMeta,
) -> Result<LoadedRun, RunError> {
    let checklist_path = if Path::new(sop_id).is_file() {
        PathBuf::from(sop_id)
    } else {
        repo.resolve(&format!("checklists/{sop_id}.md"))
    };
    if !checklist_path.is_file() {
        return Err(RunError::NoChecklist(sop_id.to_owned()));
    }
    let loaded = repo.load(&checklist_path).map_err(RunError::Repo)?;
    let resolved = resolve_checklist(repo, &loaded);
    let status = loaded.doc.front.str("status").flatten();
    if status == Some("draft") && override_reason.is_none() {
        return Err(RunError::Draft(sop_id.to_owned()));
    }

    // The recorded `sop` is the checklist's id, not the path it was loaded from. An
    // external file loaded via "Open file" names the checklist in its front matter; the
    // run directory and record use that id so History and later run commands can find it.
    let display_sop = run_key(repo, sop_id);

    let dir = run_dir(repo, &display_sop, run_id);
    if dir.is_dir() {
        return Err(RunError::RunExists(run_id.to_owned(), display_sop.clone()));
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
        sop: display_sop.clone(),
        sop_version: sop_version.to_owned(),
        sop_commit,
        snapshot_sha256,
        operator: operator.to_owned(),
        site: site.to_owned(),
        // Only carry a sensor block that has something in it; an all-blank entry is
        // noise in the log and would render an empty `sensor:` block in the record.
        sensor: meta.sensor.clone().filter(|sensor| !sensor.is_empty()),
        hardware: meta.hardware.clone(),
        conditions: meta.conditions.clone(),
    }];
    if let Some(reason) = override_reason {
        events.push(RunEvent::NoteAdded {
            at: now(),
            step: None,
            text: format!("run started over a draft checklist override: {reason}"),
        });
    }
    write_events(&dir.join("events.jsonl"), &events)?;

    let loaded_run = load(repo, &display_sop, run_id)?;
    write_record(&loaded_run)?;
    // Publish the run file as soon as the run starts, not only at `end`, so a run in
    // progress is already visible to discovery and to the History view. An in-progress
    // record carries no `status`, so none of the `complete`-only rules apply to it.
    //
    // A committed record that already exists is left alone: it is a legacy record for
    // this run_id written before runs had a directory, and it is the authoritative file
    // until `end` re-commits the finished run.
    let record_file = loaded_run.record_file.clone();
    if !record_file.exists() {
        let text = run_record_text(&loaded_run.state, &loaded_run.steps, run_id);
        atomic::write(&record_file, &text).map_err(|error| RunError::Io {
            path: record_file,
            source: error.source,
        })?;
    }
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
///
/// The copy keeps the file's own name (`logs/mag_raw.csv`), so the run directory reads
/// like the folder the data came from; the recorded `sha256` is the identity. A second
/// file with the same name gets `-2`, `-3` and so on, and re-attaching a file the run
/// already holds is a no-op rather than a second copy.
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
    // Attaching the same data to the same step twice is one attachment, not two records.
    let held = match step {
        Some(id) => loaded
            .state
            .step(id)
            .map(|state| state.attachments.as_slice())
            .unwrap_or(&[]),
        None => loaded.state.run_attachments.as_slice(),
    };
    if let Some(existing) = held.iter().find(|attachment| attachment.sha256 == digest) {
        return Ok(existing.clone());
    }
    let name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "attachment".to_owned());
    let logs_dir = loaded.dir.join("logs");
    fs::create_dir_all(&logs_dir).map_err(|source| RunError::Io {
        path: logs_dir.clone(),
        source,
    })?;
    let dest = attachment_dest(&logs_dir, &name, &bytes);
    if !dest.is_file() {
        let mut file = fs::File::create(&dest).map_err(|error| RunError::Io {
            path: dest.clone(),
            source: error,
        })?;
        file.write_all(&bytes).map_err(|error| RunError::Io {
            path: dest.clone(),
            source: error,
        })?;
    }

    let attachment = sop_core::run::Attachment {
        // Repository-relative, so the record reads and resolves the same in any clone.
        path: repo.relpath(&dest),
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

/// Where an attached file is saved: `logs/<its own name>`, with a `-2`, `-3` ... suffix
/// only when a *different* file already holds that name. Identical bytes reuse the file
/// that is already there.
fn attachment_dest(logs_dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let taken = |path: &Path| path.is_file() && !same_bytes(path, bytes);
    let direct = logs_dir.join(name);
    if !taken(&direct) {
        return direct;
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_owned());
    let extension = path
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    for suffix in 2u32.. {
        let candidate = logs_dir.join(format!("{stem}-{suffix}{extension}"));
        if !taken(&candidate) {
            return candidate;
        }
    }
    unreachable!("a free suffixed name always exists")
}

fn same_bytes(path: &Path, bytes: &[u8]) -> bool {
    fs::read(path).is_ok_and(|existing| existing == bytes)
}

/// The front matter for the run record file, so the record is a valid `runs/...` file.
fn run_file_front(state: &RunState, run_id: &str) -> String {
    let mut out = String::from("---\nkind: run\n");
    out.push_str(&format!("run_id: {}\n", scalar(run_id)));
    if let Some(value) = &state.sop { out.push_str(&format!("sop: {value}\n")); }
    if let Some(value) = &state.sop_version { out.push_str(&format!("sop_version: {value}\n")); }
    if let Some(value) = &state.sop_commit { out.push_str(&format!("sop_commit: {value}\n")); }
    if let Some(value) = &state.operator { out.push_str(&format!("operator: {value}\n")); }
    if let Some(value) = &state.site { out.push_str(&format!("site: {value}\n")); }
    if let Some(value) = &state.started { out.push_str(&format!("started: {value}\n")); }
    if let Some(value) = &state.ended { out.push_str(&format!("ended: {value}\n")); }
    if let Some(value) = &state.run_status { out.push_str(&format!("status: {value}\n")); }
    if let Some(sensor) = &state.sensor
        && !sensor.is_empty()
    {
        out.push_str("sensor:\n");
        if !sensor.model.is_empty() {
            out.push_str(&format!("  model: {}\n", scalar(&sensor.model)));
        }
        if !sensor.serial.is_empty() {
            out.push_str(&format!("  serial: {}\n", scalar(&sensor.serial)));
        }
        if !sensor.firmware.is_empty() {
            out.push_str(&format!("  firmware: {}\n", scalar(&sensor.firmware)));
        }
    }
    if !state.hardware.is_empty() {
        out.push_str("hardware:\n");
        for item in &state.hardware {
            out.push_str(&format!("  - {}\n", scalar(item)));
        }
    }
    if !state.conditions.is_empty() {
        out.push_str("conditions:\n");
        for (key, value) in &state.conditions {
            out.push_str(&format!("  {key}: {}\n", scalar(value)));
        }
    }
    // The data files, as a machine-readable list (`SPEC.md` section 9). The body restates
    // them for a reader; this is the copy the validator re-checks, so an edited or missing
    // file is caught rather than trusted.
    let attachments: Vec<&Attachment> = state
        .steps
        .values()
        .flat_map(|step| step.attachments.iter())
        .chain(state.run_attachments.iter())
        .collect();
    if !attachments.is_empty() {
        out.push_str("logs:\n");
        for attachment in attachments {
            out.push_str(&format!("  - path: {}\n", scalar(&attachment.path)));
            out.push_str(&format!("    sha256: {}\n", scalar(&attachment.sha256)));
            out.push_str(&format!("    size: {}\n", attachment.size));
        }
    }
    out.push_str(&format!("deviations_count: {}\n", state.deviations()));
    out.push_str("---\n\n");
    out
}

/// A front-matter scalar: quoted when a bare word would read as something else.
///
/// `sensor.serial` and `conditions.temp_c` are the reason this exists. A serial written
/// bare as `4451233` is an integer to every YAML reader, and a `str()` read of it then
/// returns nothing, silently losing the identifier.
fn scalar(value: &str) -> String {
    sop_core::front::format_scalar(value)
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

/// The record document for a run, exactly as [`end`] commits it.
///
/// This is the single self-contained experiment record: front matter, then one section
/// per step carrying its instructions, its checkbox items as the operator left them, and
/// what was recorded against it. `end` writes it to the repository; export prints it.
pub fn record_text(repo: &Repo, sop_id: &str, run_id: &str) -> Result<String, RunError> {
    let loaded = load(repo, sop_id, run_id)?;
    Ok(run_record_text(&loaded.state, &loaded.steps, run_id))
}

/// The experiment record for a run, from wherever it can still be read.
///
/// A run the app recorded has its own directory, and the record is re-rendered from the
/// event log so it carries the checkbox state the operator left it in. A record written
/// before runs carried a directory of their own, or written by hand, has only the file -
/// and that file *is* the record, so it is returned as it stands.
pub fn record_document(repo: &Repo, sop_id: &str, run_id: &str) -> Result<String, RunError> {
    match record_text(repo, sop_id, run_id) {
        Ok(text) => Ok(text),
        Err(RunError::NoRun(..)) => {
            let path = committed_record_file(repo, sop_id, run_id);
            if !path.is_file() {
                return Err(RunError::NoRun(sop_id.to_owned(), run_id.to_owned()));
            }
            repo.read_text(&path).map_err(RunError::Repo)
        }
        Err(error) => Err(error),
    }
}

/// The three places a run's material can live.
fn run_paths(repo: &Repo, sop_id: &str, run_id: &str) -> [PathBuf; 3] {
    [
        committed_record_file(repo, sop_id, run_id),
        run_dir(repo, sop_id, run_id),
        repo.resolve(&format!("logs/{run_id}")),
    ]
}

/// The paths that actually exist for a run, in the order they would be removed.
///
/// This is what a dry run shows the operator, and `delete` removes exactly this list.
pub fn existing_paths(repo: &Repo, sop_id: &str, run_id: &str) -> Vec<PathBuf> {
    run_paths(repo, sop_id, run_id)
        .into_iter()
        .filter(|path| path.exists())
        .collect()
}

/// Remove a run: its committed record, its run directory, and any log directory.
///
/// A record is evidence, so this is deliberately narrow. It removes the three paths a run
/// can occupy and nothing else. It refuses an id that is not well-formed - which is also
/// what keeps `..` out of the paths it builds - and refuses a run that is not there.
/// Asking the operator first is the caller's job; [`existing_paths`] is what to show them.
pub fn delete(repo: &Repo, sop_id: &str, run_id: &str) -> Result<Vec<PathBuf>, RunError> {
    if !sop_core::vocab::is_valid_id(run_id) {
        return Err(RunError::BadRunId(run_id.to_owned()));
    }
    let present = existing_paths(repo, sop_id, run_id);
    if present.is_empty() {
        return Err(RunError::NoRun(sop_id.to_owned(), run_id.to_owned()));
    }
    for path in &present {
        let result = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
        result.map_err(|source| RunError::Io {
            path: path.clone(),
            source,
        })?;
    }
    Ok(present)
}

/// Every run record in the repository, as `(checklist id, run id)`.
///
/// A run's checklist id is the directory it is filed under, so this reads the tree rather
/// than the records. `runs/_inbox/` is skipped: an inbox entry is an observation, not a
/// run, and has no run directory.
pub fn all_runs(repo: &Repo) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Ok(dirs) = fs::read_dir(repo.root().join("runs")) else {
        return out;
    };
    for dir in dirs.flatten() {
        let dir = dir.path();
        if !dir.is_dir() {
            continue;
        }
        let Some(sop) = dir.file_name().and_then(|name| name.to_str()) else { continue };
        if sop == "_inbox" {
            continue;
        }
        let Ok(files) = fs::read_dir(&dir) else { continue };
        for file in files.flatten() {
            let file = file.path();
            if file.extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            let Some(run_id) = file.file_stem().and_then(|stem| stem.to_str()) else { continue };
            out.push((sop.to_owned(), run_id.to_owned()));
        }
    }
    out.sort();
    out
}

/// Remove every run: each record file, run directory, and log directory, in one pass.
///
/// This is [`delete`] for the whole history - what a "delete all runs" button needs.
/// `runs/_inbox/` is left alone, because it holds observations rather than run history.
pub fn delete_all(repo: &Repo) -> Result<Vec<PathBuf>, RunError> {
    let mut removed = Vec::new();
    for (sop, run_id) in all_runs(repo) {
        removed.extend(delete(repo, &sop, &run_id)?);
    }
    Ok(removed)
}
