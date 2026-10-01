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

use std::fs::{self, OpenOptions};
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sop_core::Document;
use sop_core::run::{
    Attachment, AttachmentKind, ClockInfo, RecordStep, RunEvent, RunState, SensorIdentity,
    StepState, render_notes, render_record,
};
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
    /// The test plan and case this run came from, when started from `testplan/`.
    pub plan: Option<String>,
    pub case: Option<String>,
    /// The instrument's clock, when the operator recorded it at run start.
    pub clock: Option<ClockInfo>,
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
    /// Files this step is declared to produce (e.g. `mag_raw.csv`).
    pub outputs: Vec<String>,
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
    repo.resolve(&format!("runs/{}/{}", run_subdir(repo, &key), run_id))
}

/// The committed record file `runs/<sop_id>/<run_id>.md`, next to the run directory.
fn committed_record_file(repo: &Repo, sop_id: &str, run_id: &str) -> PathBuf {
    let key = run_key(repo, sop_id);
    repo.resolve(&format!("runs/{}/{}.md", run_subdir(repo, &key), run_id))
}

/// The `runs/...` sub-path a checklist is filed under: `<project_id>/<sop_id>` when the
/// repository declares a project id, or just `<sop_id>` when it does not (so a repo with
/// no project keeps the old flat layout).
pub(crate) fn run_subdir(repo: &Repo, key: &str) -> String {
    let project = crate::project::id(repo);
    if project.is_empty() {
        key.to_owned()
    } else {
        format!("{project}/{key}")
    }
}

/// Freeze a resolved checklist into `snapshot.md` text and the step shapes for replay.
fn snapshot_of(resolved: &[sop_core::Step]) -> (String, Vec<RecordStep>) {
    let mut text = String::from("---\nkind: snapshot\n---\n\n");
    let mut record_steps = Vec::new();
    for step in resolved {
        let title = step
            .title
            .clone()
            .unwrap_or_else(|| step.id_or_placeholder().to_owned());
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
            .filter(|line| {
                line.trim_start().starts_with("- [") || line.trim_start().starts_with("* [")
            })
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
            .filter(|line| {
                line.trim_start().starts_with("- [") || line.trim_start().starts_with("* [")
            })
            .count();
        shapes.push(RecordStep {
            id: step.id.clone().unwrap_or_default(),
            title: step
                .title
                .clone()
                .unwrap_or_else(|| step.id_or_placeholder().to_owned()),
            body: step.prose.clone(),
            checkbox_count,
        });
        let captures = step
            .captures
            .iter()
            .map(|capture| {
                let raw =
                    crate::manifest::json_safe(&sop_core::Value::Mapping(capture.raw.clone()));
                CaptureDef {
                    key: capture.key.clone().unwrap_or_default(),
                    label: capture.label.clone(),
                    capture_type: capture
                        .capture_type
                        .as_ref()
                        .map(|kind| kind.as_str().to_owned()),
                    unit: capture.unit.clone(),
                    required: capture.required.unwrap_or(false),
                    options: capture.options.clone(),
                    expected: raw.get("expected").cloned(),
                }
            })
            .collect();
        defs.push(ResolvedStepDef {
            id: step.id.clone().unwrap_or_default(),
            title: step
                .title
                .clone()
                .unwrap_or_else(|| step.id_or_placeholder().to_owned()),
            prose: step.prose.clone(),
            severity: step.severity.clone(),
            kind: step.key.as_ref().map(|key| key.as_str().to_owned()),
            captures,
            outputs: step.outputs.clone(),
        });
    }
    Ok((shapes, defs))
}

/// Fold steps added mid-run into the runtime step list, each after the step it named.
///
/// The snapshot is frozen at start, so an added step has no definition there; this makes
/// one up (its title, no captures or prose) so the execution view shows it and the record
/// can cite it. Inserting after the anchor step keeps the on-screen order matching the
/// order the operator added them, and an `after` of `None` (or a missing anchor) appends.
fn merge_added_steps(defs: &mut Vec<ResolvedStepDef>, added: &[sop_core::run::AddedStep]) {
    for step in added {
        let anchor = step
            .after
            .as_deref()
            .and_then(|target| defs.iter().position(|existing| existing.id == target));
        let at = anchor.map_or(defs.len(), |index| index + 1);
        defs.insert(
            at,
            ResolvedStepDef {
                id: step.id.clone(),
                title: step.title.clone(),
                prose: String::new(),
                severity: None,
                kind: None,
                captures: Vec::new(),
                outputs: Vec::new(),
            },
        );
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A time cell in a data file, normalised to a whole number of UTC seconds since the epoch.
///
/// Accepts a plain integer (epoch seconds) or an ISO-8601 timestamp with an optional
/// fractional second and an optional `Z` / `±HH:MM` offset. The tool's own timestamps are
/// `YYYY-MM-DDTHH:MM:SSZ`; anything else is reduced to that shape first so the same parser
/// places instrument data and notebook events on the same line.
fn parse_time_cell(cell: &str) -> Option<i64> {
    let cell = cell.trim();
    if cell.is_empty() {
        return None;
    }
    if let Ok(epoch) = cell.parse::<i64>() {
        return Some(epoch);
    }
    let mut s = cell.to_ascii_uppercase().replace(' ', "T");
    // Drop a fractional second: `16:03:00.250Z` -> `16:03:00Z`.
    if let Some(dot) = s.find('.') {
        let len = s.len();
        let end = s[dot + 1..]
            .find(['Z', '+', '-'])
            .map_or(len, |n| dot + 1 + n);
        s.replace_range(dot..end, "");
    }
    // Convert a trailing offset to a signed number of seconds.
    let offset_secs = s
        .find(['+', '-'])
        .filter(|pos| *pos > 10)
        .and_then(|pos| {
            let (core, off) = s.split_at(pos);
            let sign = if off.starts_with('+') { 1i64 } else { -1i64 };
            let body = &off[1..];
            let h: i64 = body.get(..2)?.parse().ok()?;
            let m: i64 = body.get(3..5)?.parse().ok()?;
            s = core.to_owned();
            Some(sign * (h * 3600 + m * 60))
        })
        .unwrap_or(0);
    if s.ends_with('Z') {
        s.pop();
    }
    if !s.ends_with('Z') {
        s.push('Z');
    }
    let base = sop_core::timestamp::epoch_seconds(&s)?;
    Some(base - offset_secs)
}

/// The first and last timestamp in a CSV, and how many rows carry one, when a column
/// looks like a time column (`timestamp*` / `time*` / `utc*` / `gps_time*`) and every
/// value in it parses. `None` when the file does not carry such a column, or any value
/// in it fails - the tool does not guess, it just skips the time information.
fn csv_time_range(bytes: &[u8]) -> Option<(String, String, u64)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    let header = lines.next()?.trim();
    let columns: Vec<&str> = header.split(',').map(str::trim).collect();
    let index = columns.iter().position(|name| {
        let lower = name.to_ascii_lowercase();
        lower.starts_with("timestamp")
            || lower.starts_with("time")
            || lower.starts_with("utc")
            || lower.starts_with("gps_time")
    })?;
    let mut first: Option<i64> = None;
    let mut last: Option<i64> = None;
    let mut rows = 0u64;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cell = trimmed.split(',').nth(index)?.trim();
        let epoch = parse_time_cell(cell)?;
        first = Some(first.map_or(epoch, |seen| seen.min(epoch)));
        last = Some(epoch);
        rows += 1;
    }
    let (f, l) = (first?, last?);
    Some((
        sop_core::timestamp::rfc3339_from_epoch(f),
        sop_core::timestamp::rfc3339_from_epoch(l),
        rows,
    ))
}

fn now() -> String {
    sop_core::now_utc_rfc3339()
}

fn write_events(path: &Path, events: &[RunEvent]) -> Result<(), RunError> {
    let mut text = String::new();
    for event in events {
        let line = serde_json::to_string(event).map_err(|error| RunError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::other(error.to_string()),
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

/// Append one event to the log as a single line, holding the run's lock so a concurrent
/// writer cannot interleave with it.
///
/// The log is the source of truth, so it is the one file that must never be rewritten:
/// a whole-file rewrite can lose every earlier event if it is interrupted. `O_APPEND`
/// makes the offset-and-write one step, the advisory lock serialises the app and the CLI
/// (and two windows), and `sync_data` means a crash can only ever cost the event being
/// written - the events before it are already on the medium.
fn append_event(path: &Path, event: &RunEvent) -> Result<(), RunError> {
    let line = serde_json::to_string(event).map_err(|error| RunError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(error.to_string()),
    })?;
    let mut log = OpenOptions::new()
        .read(true)
        .append(true)
        .open(path)
        .map_err(|source| RunError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    // Released when `log` drops, so an error below cannot leave the run locked.
    log.lock().map_err(|source| RunError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    log.write_all(line.as_bytes())
        .and_then(|()| log.write_all(b"\n"))
        .and_then(|()| log.sync_data())
        .map_err(|source| RunError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    log.unlock().map_err(|source| RunError::Io {
        path: path.to_path_buf(),
        source,
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

    let snapshot_text = repo.read_text(&snapshot_path).map_err(RunError::Repo)?;
    let (seed, mut defs) = seed_steps_from_snapshot(&snapshot_text)?;

    let mut state = RunState::default();
    for step in &seed {
        state
            .steps
            .entry(step.id.clone())
            .or_insert_with(|| StepState {
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

    // Steps added mid-run are not in the snapshot; fold them into the runtime step list so
    // the execution view shows them and the record's coverage can accept them.
    merge_added_steps(&mut defs, &state.added_steps);

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
    // `notes.md` is a generated convenience: every note for the run in one place. The
    // event log stays the source of truth, so it is rebuilt here, never edited.
    let notes = render_notes(&loaded.state);
    atomic::write(&loaded.dir.join("notes.md"), &notes).map_err(|error| RunError::Io {
        path: loaded.dir.join("notes.md"),
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
    start_with_meta(
        repo,
        sop_id,
        run_id,
        operator,
        site,
        override_reason,
        &RunMeta::default(),
    )
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
    } else if let Some(path) = repo.checklist_path(sop_id) {
        // A run can be started from a `checklists/` file or a `testplan/` case; both are
        // found by their `sop_id` here.
        path
    } else {
        return Err(RunError::NoChecklist(sop_id.to_owned()));
    };
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
    let sop_version = loaded.doc.front.str("version").flatten().unwrap_or("1");
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
        plan: meta.plan.clone(),
        case: meta.case.clone(),
        // Only carry a sensor block that has something in it; an all-blank entry is
        // noise in the log and would render an empty `sensor:` block in the record.
        sensor: meta.sensor.clone().filter(|sensor| !sensor.is_empty()),
        hardware: meta.hardware.clone(),
        conditions: meta.conditions.clone(),
        clock: meta.clock.clone().filter(|clock| !clock.is_empty()),
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
        let text = run_record_text(&loaded_run.state, &loaded_run.steps, run_id, None);
        atomic::write(&record_file, &text).map_err(|error| RunError::Io {
            path: record_file,
            source: error.source,
        })?;
    }
    Ok(loaded_run)
}

/// Append one event to the run and regenerate the record.
///
/// The event is appended to `events.jsonl` (see [`append_event`]); the rendered
/// `record.md` is a convenience, rebuilt from the log afterwards and never the truth.
pub fn record(
    repo: &Repo,
    sop_id: &str,
    run_id: &str,
    event: &RunEvent,
) -> Result<LoadedRun, RunError> {
    record_events(repo, sop_id, run_id, std::slice::from_ref(event))
}

/// Drop a tagged field marker on the run: a timestamped, labelled moment for aligning a
/// log to a physical feature (`item 3`).
pub fn marker(repo: &Repo, sop_id: &str, run_id: &str, label: &str) -> Result<LoadedRun, RunError> {
    let label = label.trim();
    if label.is_empty() {
        return Err(RunError::Core(sop_core::RunError::ReasonRequired("marker")));
    }
    record(
        repo,
        sop_id,
        run_id,
        &RunEvent::FieldMarker {
            at: now(),
            label: label.to_owned(),
        },
    )
}

/// Append one or more events as a batch: validate them all against the current state,
/// append them all to the log, then regenerate the record once.
fn record_events(
    repo: &Repo,
    sop_id: &str,
    run_id: &str,
    events: &[RunEvent],
) -> Result<LoadedRun, RunError> {
    let loaded = load(repo, sop_id, run_id)?;
    let mut next = loaded.state.clone();
    // The tool's clock is the authority: the renderer sends no timestamp, and any it
    // might send is overwritten here so the run's time line is one consistent clock.
    let mut stamped: Vec<RunEvent> = events.to_vec();
    for event in &mut stamped {
        event.stamp(now());
        sop_core::run::apply(&mut next, event).map_err(RunError::Core)?;
    }

    // Whether the run was already ended before this call, and how long its log was then.
    // A late addition (a log, photo, note) is applied to an ended run; `end` itself is
    // applied to a run that is still open, so only the former refreshes the committed
    // record - `end` writes that record with the seal itself.
    let was_ended = loaded.state.is_ended();
    let pre_len = fs::metadata(&loaded.events_path)
        .map(|meta| meta.len())
        .unwrap_or(0);

    for event in &stamped {
        append_event(&loaded.events_path, event)?;
    }

    let reloaded = load(repo, sop_id, run_id)?;
    write_record(&reloaded)?;
    if was_ended {
        refresh_committed_record(&reloaded, run_id, pre_len)?;
    }
    Ok(reloaded)
}

/// Re-write the committed record `runs/<sop>/<run_id>.md` after a post-run addition.
///
/// The seal (the sha256 of the event log as it was when the run ended) is kept from the
/// record that is already there, so the events added afterwards do not invalidate it. A
/// record written before prefix sealing only carries `events_sha256`; the length it
/// covered is the log length just before this addition, which is the length to seal.
fn refresh_committed_record(
    reloaded: &LoadedRun,
    run_id: &str,
    pre_len: u64,
) -> Result<(), RunError> {
    let seal = match read_seal_fields(&reloaded.record_file) {
        // A record already carrying a prefix seal keeps it verbatim.
        Some((Some(bytes), sha256)) => Seal { bytes, sha256 },
        // A legacy record sealed the whole file, whose length was `pre_len`.
        Some((None, sha256)) => Seal {
            bytes: pre_len,
            sha256,
        },
        // No seal at all: seal the log as it stood before this addition.
        None => Seal {
            bytes: pre_len,
            sha256: hash_first_n(&reloaded.events_path, pre_len).unwrap_or_default(),
        },
    };
    let text = run_record_text(&reloaded.state, &reloaded.steps, run_id, Some(&seal));
    atomic::write(&reloaded.record_file, &text).map_err(|error| RunError::Io {
        path: reloaded.record_file.clone(),
        source: error.source,
    })?;
    Ok(())
}

/// A suggested name for an attached data file, from the run's own metadata
/// (`LOGS.md`: `<site>_<date>_<sensor>_<stream>.csv`), so the file name and the record
/// agree without the operator inventing one. Best-effort: blank parts are skipped.
pub fn suggest_log_name(loaded: &LoadedRun, stream: &str) -> String {
    let state = &loaded.state;
    let site = state.site.as_deref().unwrap_or("");
    let date = state
        .started
        .as_deref()
        .and_then(|started| started.get(..10))
        .unwrap_or("");
    let sensor = state
        .sensor
        .as_ref()
        .map(|sensor| sensor.model.as_str())
        .unwrap_or("");
    let stream = stream.trim().replace([' ', '/'], "_");
    let parts: Vec<&str> = vec![site, date, sensor]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect();
    if stream.is_empty() {
        format!("{}.csv", parts.join("_"))
    } else {
        format!("{}_{stream}.csv", parts.join("_"))
    }
}

/// Copy a file into the run's logs, hash it, and record an event for it.
///
/// The default kind: a log. [`attach_kind`] is the same operation for a photo or any
/// other file, which is what the History view uploads after a run has ended.
pub fn attach(
    repo: &Repo,
    sop_id: &str,
    run_id: &str,
    step: Option<&str>,
    source: &Path,
) -> Result<sop_core::run::Attachment, RunError> {
    attach_kind(repo, sop_id, run_id, step, AttachmentKind::Log, source)
}

/// Copy a file of a given kind into the run's subdirectory, hash it, and record it.
///
/// The copy keeps the file's own name (`logs/mag_raw.csv`, `photos/plot.jpg`), so the
/// run directory reads like the folder the data came from; the recorded `sha256` is the
/// identity. A second file with the same name gets `-2`, `-3` and so on, and
/// re-attaching the same bytes of the same kind is a no-op rather than a second copy.
/// Only a `log` is scanned for a time column.
pub fn attach_kind(
    repo: &Repo,
    sop_id: &str,
    run_id: &str,
    step: Option<&str>,
    kind: AttachmentKind,
    source: &Path,
) -> Result<sop_core::run::Attachment, RunError> {
    let loaded = load(repo, sop_id, run_id)?;
    let bytes = fs::read(source).map_err(|error| RunError::Io {
        path: source.to_path_buf(),
        source: error,
    })?;
    let digest = sha256_hex(&bytes);
    // Re-attaching the same bytes of the same kind to the same step is one attachment.
    // The kind is part of the identity: a log and a photo can share a hash (an empty
    // file) without one replacing the other.
    let held = match step {
        Some(id) => loaded
            .state
            .step(id)
            .map(|state| state.attachments.as_slice())
            .unwrap_or(&[]),
        None => loaded.state.run_attachments.as_slice(),
    };
    if let Some(existing) = held
        .iter()
        .find(|attachment| attachment.sha256 == digest && attachment.kind == kind)
    {
        return Ok(existing.clone());
    }
    let name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "attachment".to_owned());
    let dir = loaded.dir.join(kind.dir());
    fs::create_dir_all(&dir).map_err(|source| RunError::Io {
        path: dir.clone(),
        source,
    })?;
    let dest = attachment_dest(&dir, &name, &bytes);
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

    // The data's own time span, when the file carries a parseable time column. Only a log
    // is scanned; a photo or a PDF has no such column. Best-effort either way: a file with
    // no such column, or one value that does not parse, simply omits the fields.
    let (t_min, t_max, row_count) = if kind == AttachmentKind::Log {
        match csv_time_range(&bytes) {
            Some((min, max, rows)) => (Some(min), Some(max), Some(rows)),
            None => (None, None, None),
        }
    } else {
        (None, None, None)
    };

    // Repository-relative, so the record reads and resolves the same in any clone.
    let path = repo.relpath(&dest);
    let event = RunEvent::AttachmentAdded {
        at: now(),
        step: step.map(str::to_owned),
        // A log is the default, so its `kind` is left off, keeping a new log's event
        // identical to one written before the field existed.
        kind: (kind != AttachmentKind::Log).then_some(kind),
        path,
        sha256: digest.clone(),
        size: bytes.len() as u64,
        t_min,
        t_max,
        row_count,
    };
    // The event's timestamp is stamped by `record`; read the attachment back from the
    // reloaded state so the returned value agrees with what was written.
    let reloaded = record(repo, sop_id, run_id, &event)?;
    let found = match step {
        Some(id) => reloaded
            .state
            .step(id)
            .map(|state| state.attachments.as_slice()),
        None => Some(reloaded.state.run_attachments.as_slice()),
    };
    found
        .and_then(|held| held.iter().find(|a| a.sha256 == digest && a.kind == kind))
        .cloned()
        .ok_or(RunError::Core(sop_core::RunError::MissingStep))
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
fn run_file_front(state: &RunState, run_id: &str, seal: Option<&Seal>) -> String {
    let mut out = String::from("---\nkind: run\n");
    out.push_str(&format!("run_id: {}\n", scalar(run_id)));
    if let Some(value) = &state.sop {
        out.push_str(&format!("sop: {value}\n"));
    }
    if let Some(value) = &state.sop_version {
        out.push_str(&format!("sop_version: {value}\n"));
    }
    if let Some(value) = &state.sop_commit {
        out.push_str(&format!("sop_commit: {value}\n"));
    }
    if let Some(value) = &state.operator {
        out.push_str(&format!("operator: {value}\n"));
    }
    if let Some(value) = &state.site {
        out.push_str(&format!("site: {value}\n"));
    }
    if let Some(value) = &state.plan {
        out.push_str(&format!("plan: {}\n", scalar(value)));
    }
    if let Some(value) = &state.case {
        out.push_str(&format!("case: {}\n", scalar(value)));
    }
    if let Some(value) = &state.started {
        out.push_str(&format!("started: {value}\n"));
    }
    if let Some(value) = &state.ended {
        out.push_str(&format!("ended: {value}\n"));
    }
    if let Some(value) = &state.run_status {
        out.push_str(&format!("status: {value}\n"));
    }
    if let Some(value) = &state.conclusion {
        out.push_str(&format!("conclusion: {value}\n"));
    }
    if let Some(value) = &state.conclusion_note {
        out.push_str(&format!("conclusion_note: {}\n", scalar(value)));
    }
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
    if let Some(clock) = &state.clock
        && !clock.is_empty()
    {
        out.push_str("clock:\n");
        out.push_str(&format!("  basis: {}\n", scalar(&clock.basis)));
        out.push_str(&format!(
            "  instrument_time: {}\n",
            scalar(&clock.instrument_time)
        ));
        out.push_str(&format!("  offset_secs: {}\n", clock.offset_secs));
    }
    if !state.markers.is_empty() {
        out.push_str("markers:\n");
        for marker in &state.markers {
            out.push_str(&format!("  - at: {}\n", scalar(&marker.at)));
            out.push_str(&format!("    label: {}\n", scalar(&marker.label)));
        }
    }
    // The data files, as a machine-readable list (`SPEC.md` section 9). The body restates
    // them for a reader; this is the copy the validator re-checks, so an edited or missing
    // file is caught rather than trusted. Each entry names the step it was attached to,
    // when it was, so the validator can check the data overlaps the step's time window.
    let mut attachments: Vec<(&str, &Attachment)> = Vec::new();
    for (step_id, step) in &state.steps {
        for attachment in &step.attachments {
            attachments.push((step_id.as_str(), attachment));
        }
    }
    for attachment in &state.run_attachments {
        attachments.push(("", attachment));
    }
    if !attachments.is_empty() {
        out.push_str("logs:\n");
        for (step_id, attachment) in attachments {
            out.push_str(&format!("  - path: {}\n", scalar(&attachment.path)));
            // A photo or any other file names its kind so a reader (and the validator)
            // knows which subdirectory it belongs in without parsing the path. A log is
            // the default, so its kind is left out and an old record reads unchanged.
            if attachment.kind != AttachmentKind::Log {
                out.push_str(&format!("    kind: {}\n", attachment.kind.as_str()));
            }
            if !step_id.is_empty() {
                out.push_str(&format!("    step: {}\n", scalar(step_id)));
            }
            out.push_str(&format!("    sha256: {}\n", scalar(&attachment.sha256)));
            out.push_str(&format!("    size: {}\n", attachment.size));
            if let Some(min) = &attachment.t_min {
                out.push_str(&format!("    t_min: {}\n", scalar(min)));
            }
            if let Some(max) = &attachment.t_max {
                out.push_str(&format!("    t_max: {}\n", scalar(max)));
            }
            if let Some(rows) = attachment.row_count {
                out.push_str(&format!("    row_count: {rows}\n"));
            }
        }
    }
    if !state.added_steps.is_empty() {
        out.push_str(&format!("added_steps: {}\n", state.added_steps.len()));
    }
    // The seal: the length of the event log when the run ended, and the sha256 of exactly
    // that prefix. Events appended after the end (a late log, photo, or note) sit beyond
    // it and do not change it (`SPEC.md` section 9, `DECISIONS.md` D29).
    if let Some(seal) = seal {
        out.push_str(&format!("events_sealed_bytes: {}\n", seal.bytes));
        out.push_str(&format!("events_sha256: {}\n", scalar(&seal.sha256)));
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
fn run_record_text(
    state: &RunState,
    steps: &[RecordStep],
    run_id: &str,
    seal: Option<&Seal>,
) -> String {
    let mut out = run_file_front(state, run_id, seal);
    out.push_str(&render_record(state, steps));
    out
}

/// End the run, writing the human record file next to the run directory.
///
/// `conclusion` is the operator's own verdict on the run - one of `pass`, `fail`,
/// `inconclusive` - and `conclusion_note` the one-sentence summary they write with it.
/// Both are optional and additive: ending a run without them records the same `RunEnded`
/// event as before this field existed, so an older caller still ends a run the old way.
pub fn end(
    repo: &Repo,
    sop_id: &str,
    run_id: &str,
    status: &str,
    conclusion: Option<(String, String)>,
) -> Result<LoadedRun, RunError> {
    let mut events = vec![RunEvent::RunEnded {
        at: now(),
        status: status.to_owned(),
    }];
    if let Some((outcome, summary)) = conclusion {
        events.push(RunEvent::RunConcluded {
            at: now(),
            conclusion: outcome,
            summary,
        });
    }
    let loaded = record_events(repo, sop_id, run_id, &events)?;
    // The whole log as it stands now is the sealed prefix: everything the run was ended
    // with. Late additions will be appended beyond it.
    let bytes = fs::metadata(&loaded.events_path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let seal = Seal {
        bytes,
        sha256: hash_first_n(&loaded.events_path, bytes).unwrap_or_default(),
    };
    let text = run_record_text(&loaded.state, &loaded.steps, run_id, Some(&seal));
    atomic::write(&loaded.record_file, &text).map_err(|error| RunError::Io {
        path: loaded.record_file.clone(),
        source: error.source,
    })?;
    Ok(loaded)
}

/// The sealed prefix of a run's event log: the byte length sealed at end, and the sha256
/// of exactly those bytes. Everything past `bytes` is a post-run addition.
#[derive(Debug, Clone)]
pub struct Seal {
    pub bytes: u64,
    pub sha256: String,
}

/// The sha256 of the first `bytes` bytes of a file, streamed so a large log is not read
/// into memory whole. `None` when the file cannot be opened.
fn hash_first_n(path: &Path, bytes: u64) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut reader = file.take(bytes);
    let mut hasher = Sha256::new();
    std::io::copy(&mut reader, &mut hasher).ok()?;
    let digest = hasher.finalize();
    Some(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// The seal fields read back from a committed record: `(events_sealed_bytes, events_sha256)`.
/// `events_sealed_bytes` is `None` for a record written before prefix sealing.
fn read_seal_fields(record_file: &Path) -> Option<(Option<u64>, String)> {
    let text = fs::read_to_string(record_file).ok()?;
    let doc = Document::parse(&text).ok()?;
    let sha256 = doc.front.str("events_sha256").flatten()?.to_owned();
    let bytes = doc
        .front
        .get("events_sealed_bytes")
        .and_then(|value| value.as_i64())
        .map(|value| value.max(0) as u64);
    Some((bytes, sha256))
}

/// The seal a run's record should carry when it is re-rendered: the one it already has,
/// or - for a run that never sealed - the whole log as it stands.
fn seal_for_record(loaded: &LoadedRun) -> Option<Seal> {
    match read_seal_fields(&loaded.record_file) {
        Some((Some(bytes), sha256)) => Some(Seal { bytes, sha256 }),
        Some((None, sha256)) => {
            let bytes = fs::metadata(&loaded.events_path)
                .map(|meta| meta.len())
                .unwrap_or(0);
            Some(Seal { bytes, sha256 })
        }
        None => {
            let bytes = fs::metadata(&loaded.events_path)
                .map(|meta| meta.len())
                .unwrap_or(0);
            hash_first_n(&loaded.events_path, bytes).map(|sha256| Seal { bytes, sha256 })
        }
    }
}

/// The record document for a run, exactly as [`end`] commits it.
///
/// This is the single self-contained experiment record: front matter, then one section
/// per step carrying its instructions, its checkbox items as the operator left them, and
/// what was recorded against it. `end` writes it to the repository; export prints it.
pub fn record_text(repo: &Repo, sop_id: &str, run_id: &str) -> Result<String, RunError> {
    let loaded = load(repo, sop_id, run_id)?;
    let seal = seal_for_record(&loaded);
    Ok(run_record_text(
        &loaded.state,
        &loaded.steps,
        run_id,
        seal.as_ref(),
    ))
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

/// Whether the checklist a run was started from has changed since.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drift {
    pub started_version: Option<String>,
    pub started_commit: Option<String>,
    pub snapshot_sha256: Option<String>,
    /// The hash of the checklist as it resolves today, or `None` when it cannot be found.
    pub current_sha256: Option<String>,
    pub current_version: Option<String>,
    /// True only when the current checklist resolves and differs from the snapshot.
    pub drifted: bool,
}

/// Compare a run's frozen snapshot with the checklist as it resolves now.
///
/// The comparison is on the resolved body, the same bytes that were hashed at start, so
/// an edit to any included procedure counts. A checklist that cannot be found - removed,
/// or loaded from outside the repository - yields `drifted: false` rather than a guess.
pub fn drift(repo: &Repo, sop_id: &str, run_id: &str) -> Result<Drift, RunError> {
    let loaded = load(repo, sop_id, run_id)?;
    let state = &loaded.state;
    let key = state.sop.clone().unwrap_or_else(|| run_key(repo, sop_id));
    let current = find_checklist(repo, &key).and_then(|path| repo.load(&path).ok());
    let (current_sha256, current_version) = match &current {
        Some(loaded) => {
            let resolved = resolve_checklist(repo, loaded);
            let (text, _) = snapshot_of(&resolved.steps);
            (
                Some(sha256_hex(text.as_bytes())),
                loaded.doc.front.str("version").flatten().map(str::to_owned),
            )
        }
        None => (None, None),
    };
    Ok(Drift {
        started_version: state.sop_version.clone(),
        started_commit: state.sop_commit.clone(),
        snapshot_sha256: state.snapshot_sha256.clone(),
        drifted: current_sha256.is_some() && current_sha256 != state.snapshot_sha256,
        current_sha256,
        current_version,
    })
}

/// The checklist file a run's `sop` names today: a path, `checklists/<id>.md`, or a test
/// case under `testplan/` whose front matter carries that `sop_id`.
fn find_checklist(repo: &Repo, key: &str) -> Option<PathBuf> {
    let as_path = Path::new(key);
    if as_path.is_file() {
        return Some(as_path.to_path_buf());
    }
    repo.checklist_path(key)
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
/// True when a directory holds run records directly (`.md` files with a run id), which is
/// what a `runs/<sop_id>/` folder looks like in the flat layout.
pub(crate) fn dir_has_run_records(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|entry| {
                entry.path().extension().is_some_and(|ext| ext == "md")
            })
        })
        .unwrap_or(false)
}

/// Collect every `*.md` record file under `dir`, pushing `(sop_id, run_id)` pairs.
fn collect_run_records(dir: &Path, sop: &str, out: &mut Vec<(String, String)>) {
    let Ok(files) = fs::read_dir(dir) else {
        return;
    };
    for file in files.flatten() {
        let file = file.path();
        if file.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let Some(run_id) = file.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        out.push((sop.to_owned(), run_id.to_owned()));
    }
}

/// Every run record in the repository, as `(checklist id, run id)`.
///
/// A run's checklist id is the directory it is filed under, so this reads the tree rather
/// than the records. Both layouts are read: the current `runs/<project>/<sop>/` nesting
/// and the older flat `runs/<sop>/`. `runs/_inbox/` is skipped: an inbox entry is an
/// observation, not a run, and has no run directory.
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
        let Some(name) = dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name == "_inbox" {
            continue;
        }
        if dir_has_run_records(&dir) {
            // Flat layout: this directory is a checklist.
            collect_run_records(&dir, name, &mut out);
        } else {
            // Project layout: this directory is a project; its subdirs are checklists.
            let Ok(subs) = fs::read_dir(&dir) else {
                continue;
            };
            for sub in subs.flatten() {
                let sub = sub.path();
                if !sub.is_dir() {
                    continue;
                }
                let Some(sop) = sub.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if sop == "_inbox" {
                    continue;
                }
                collect_run_records(&sub, sop, &mut out);
            }
        }
    }
    out.sort();
    out
}

/// Move any flat-layout `runs/<sop_id>/` folders under `runs/<project_id>/` now that the
/// top level is the project. Called whenever the manifest is built, so an old copy is
/// upgraded on the next open. Returns how many folders were moved.
pub fn migrate_runs(repo: &Repo) -> usize {
    let project = crate::project::id(repo);
    if project.is_empty() {
        return 0;
    }
    let root = repo.root().join("runs");
    let target = root.join(&project);
    let Ok(dirs) = fs::read_dir(&root) else {
        return 0;
    };
    let mut moved = 0;
    for dir in dirs.flatten() {
        let dir = dir.path();
        if !dir.is_dir() {
            continue;
        }
        let Some(name) = dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name == "_inbox" || name == project {
            continue;
        }
        // Only move a folder that looks like a checklist in the flat layout, not a project
        // folder that already nests its checklists.
        if !dir_has_run_records(&dir) {
            continue;
        }
        let dst = target.join(name);
        if let Err(_) = fs::create_dir_all(&target) {
            continue;
        }
        if dst.exists() {
            // Merge: move the folder's contents into the existing target, then drop it.
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let from = entry.path();
                    let to = dst.join(from.file_name().unwrap_or_default());
                    if to.exists() {
                        let _ = fs::remove_dir_all(&to);
                    }
                    let _ = fs::rename(&from, &to);
                }
            }
            let _ = fs::remove_dir_all(&dir);
        } else {
            let _ = fs::rename(&dir, &dst);
        }
        moved += 1;
    }
    moved
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
