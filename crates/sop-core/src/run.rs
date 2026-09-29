//! A run and its event log.
//!
//! A run is one execution of a checklist. Its source of truth is an append-only JSONL
//! event log; every change to the run is one typed event. The `RunState` below is a pure
//! function of the events: replay the log and you get the state, so a crash can always be
//! recovered by reading the events again (`DESIGN.md` 6.2, P2).
//!
//! This module holds the event types, the state machine that applies them, and the
//! rendering of a human reviewable `record.md`. It does no I/O; `sop-repo` decides where
//! the log and record live, and checks the events against the content the run was
//! started from.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::timestamp::{epoch_seconds, format_duration};
use crate::vocab::{RESULT_STATUS, RUN_CONCLUSION, RUN_STATUS};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RunError {
    #[error("the first event of a run must be RunStarted")]
    MustStartWithRunStarted,

    #[error("a run can only be started once")]
    AlreadyStarted,

    #[error("the run has ended; no further events are allowed")]
    Ended,

    #[error("an event must name the step it belongs to")]
    MissingStep,

    #[error("a '{0}' event carries no reason; it is required")]
    ReasonRequired(&'static str),

    #[error("checkbox index {index} is out of range (the step has {len} checkboxes)")]
    CheckboxOutOfRange { index: usize, len: usize },

    #[error("'{0}' is not a result status; use one of {1}")]
    BadResultStatus(String, String),

    #[error("'{0}' is not a run status; use one of {1}")]
    BadRunStatus(String, String),

    #[error("'{0}' is not a run conclusion; use one of {1}")]
    BadConclusion(String, String),

    #[error("a run conclusion needs a one-sentence note")]
    ConclusionNoteRequired,

    #[error("a run conclusion can only be recorded once the run has ended")]
    ConclusionBeforeEnd,
}

/// The instrument a run was recorded with.
///
/// All three parts are optional so an operator can record a serial without a model, and
/// so a field left blank is absent rather than an empty string in the record. Serial and
/// firmware are strings because a serial is an identifier, not a number: it may carry a
/// leading zero, and a YAML reader that turned `0001234` into an integer would corrupt it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorIdentity {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub serial: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub firmware: String,
}

impl SensorIdentity {
    pub fn is_empty(&self) -> bool {
        self.model.is_empty() && self.serial.is_empty() && self.firmware.is_empty()
    }
}

/// The instrument's clock as the operator recorded it at run start.
///
/// Notebook events carry the tool's clock; the instrument's data carries the instrument's
/// clock. Recording both lets a later analysis align the two (`item 3`): `instrument_time`
/// is what the instrument displayed, `offset_secs` is instrument minus tool clock, and
/// `basis` names the instrument's time base (`UTC` / `GPS` / `local`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockInfo {
    pub basis: String,
    pub instrument_time: String,
    pub offset_secs: i64,
}

impl ClockInfo {
    pub fn is_empty(&self) -> bool {
        self.basis.is_empty() && self.instrument_time.is_empty()
    }
}

/// One line of the run's event log. The `type` discriminates the variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum RunEvent {
    RunStarted {
        at: String,
        sop: String,
        sop_version: String,
        sop_commit: String,
        snapshot_sha256: String,
        operator: String,
        site: String,
        /// The test plan this run belongs to, when it was started from `testplan/`.
        /// Absent for a checklist opened directly, so a reader can tell a plan run
        /// from an ad-hoc one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        plan: Option<String>,
        /// The test case within `plan` this run was started from. Meaningful only
        /// together with `plan`; both are absent on a run started from a checklist.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        case: Option<String>,
        /// The instrument, when the operator recorded one. Absent in logs written
        /// before this field existed, which is why every reader treats it as optional.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sensor: Option<SensorIdentity>,
        /// Software and equipment used, free text, one item each.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        hardware: Vec<String>,
        /// Environment at the start of the run: weather, temperature, and anything
        /// else the checklist asks for. Free-form so a new dimension needs no schema.
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        conditions: BTreeMap<String, String>,
        /// The instrument's clock, when the operator recorded it at run start.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        clock: Option<ClockInfo>,
    },
    StepOpened {
        at: String,
        step: String,
    },
    /// The operator added a step mid-run, so the record can carry a measurement the
    /// checklist did not foresee. The id is generated by the tool (`adhoc-NNN`), never by
    /// the operator (`DESIGN.md` 6.6). The snapshot stays frozen - it records the template
    /// the run was started from - so the added step lives only in the event log and is
    /// merged back into the runtime step list at replay.
    StepAdded {
        at: String,
        id: String,
        title: String,
        /// The step id to insert after; `None` appends at the end.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<String>,
    },
    /// A tagged moment the operator dropped in the field, so a later analysis can align a
    /// log to a physical feature ("start of the third line") when the instrument clock is
    /// not the notebook clock. One hotkey + a label, nothing more (`item 3`).
    FieldMarker {
        at: String,
        label: String,
    },
    CheckboxToggled {
        at: String,
        step: String,
        index: usize,
        checked: bool,
    },
    CaptureRecorded {
        at: String,
        step: String,
        key: String,
        value: String,
        unit: Option<String>,
    },
    CaptureCleared {
        at: String,
        step: String,
        key: String,
        reason: String,
    },
    /// The operator saw a capture whose value is outside the expected range and accepted
    /// it. The tool never judges; this is the recorded judgement, so a red line becomes a
    /// decision that survives review.
    CaptureAcknowledged {
        at: String,
        step: String,
        key: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    NoteAdded {
        at: String,
        step: Option<String>,
        text: String,
    },
    AttachmentAdded {
        at: String,
        step: Option<String>,
        path: String,
        sha256: String,
        size: u64,
        /// The first and last timestamp in the attached data, when the file carries a
        /// parseable time column. Absent for a file the tool could not place in time.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        t_min: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        t_max: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        row_count: Option<u64>,
    },
    StepStatusChanged {
        at: String,
        step: String,
        status: String,
        reason: Option<String>,
    },
    RunEnded {
        at: String,
        status: String,
    },
    /// The operator's own verdict on the run, recorded after it ends. The tool never
    /// derives one from the other: `status` (complete / partial / aborted) means the run
    /// was executed; `conclusion` is the human judgement and `summary` is the one sentence
    /// that says why (`DECISIONS.md` D4). Additive in the log, so a log written before this
    /// event existed still replays.
    RunConcluded {
        at: String,
        conclusion: String,
        summary: String,
    },
}

/// The result status of one step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum StepStatus {
    #[default]
    Open,
    Done,
    Skipped,
    Deviated,
}

impl StepStatus {
    fn parse(value: &str) -> Result<Self, RunError> {
        match value {
            // `open` is not a recorded result status; it is how a mis-tapped step is
            // reopened. A compensating event carries it so the log stays append-only.
            "open" => Ok(StepStatus::Open),
            "done" => Ok(StepStatus::Done),
            "skipped" => Ok(StepStatus::Skipped),
            "deviated" => Ok(StepStatus::Deviated),
            other => Err(RunError::BadResultStatus(other.to_owned(), RESULT_STATUS.join(", "))),
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            StepStatus::Open => "open",
            StepStatus::Done => "done",
            StepStatus::Skipped => "skipped",
            StepStatus::Deviated => "deviated",
        }
    }
}

/// A recorded capture value on a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureValue {
    pub value: String,
    pub unit: Option<String>,
}

/// A log attached to a step or the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    /// The data's own first and last timestamps, and how many rows it spans. These come
    /// from the attached file when it carries a parseable time column, and let the
    /// validator check the log overlaps the run it was attached to.
    pub t_min: Option<String>,
    pub t_max: Option<String>,
    pub row_count: Option<u64>,
}

/// A step added to a run after it started. It is recorded in the event log, not the
/// snapshot, and merged into the runtime step list at replay.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddedStep {
    pub id: String,
    pub title: String,
    /// The step id this was inserted after; `None` means appended at the end.
    pub after: Option<String>,
}

/// A tagged moment the operator dropped in the field, with the tool's clock at that moment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldMarker {
    pub at: String,
    pub label: String,
}

/// The state of one step during a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepState {
    pub status: StepStatus,
    pub reason: Option<String>,
    /// When the operator last opened the step, in the run's event stream. Together with
    /// `ended_at` it forms the step's time window, which the timeline renders.
    pub opened_at: Option<String>,
    /// When the step was closed (done / skipped / deviated), or `None` while it is open.
    pub ended_at: Option<String>,
    pub captures: BTreeMap<String, CaptureValue>,
    /// Capture keys the operator acknowledged as outside the expected range.
    pub acknowledged: BTreeSet<String>,
    pub checkboxes: Vec<bool>,
    pub notes: Vec<String>,
    pub attachments: Vec<Attachment>,
}

/// The state of the whole run, produced by replaying its events.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunState {
    pub sop: Option<String>,
    pub sop_version: Option<String>,
    pub sop_commit: Option<String>,
    pub snapshot_sha256: Option<String>,
    pub operator: Option<String>,
    pub site: Option<String>,
    pub plan: Option<String>,
    pub case: Option<String>,
    pub started: Option<String>,
    pub ended: Option<String>,
    pub run_status: Option<String>,
    pub conclusion: Option<String>,
    pub conclusion_note: Option<String>,
    pub sensor: Option<SensorIdentity>,
    pub hardware: Vec<String>,
    pub conditions: BTreeMap<String, String>,
    pub clock: Option<ClockInfo>,
    pub steps: BTreeMap<String, StepState>,
    /// Steps added mid-run, in the order they were added.
    pub added_steps: Vec<AddedStep>,
    /// Tagged moments the operator dropped, in the order they were dropped.
    pub markers: Vec<FieldMarker>,
    pub run_notes: Vec<String>,
    pub run_attachments: Vec<Attachment>,
}

impl RunState {
    pub fn step(&self, id: &str) -> Option<&StepState> {
        self.steps.get(id)
    }

    pub fn is_ended(&self) -> bool {
        self.ended.is_some()
    }

    /// The count of steps that ended as `deviated`, for `deviations_count`.
    ///
    /// A step added mid-run is not part of the checklist, so deviating one is not counted
    /// against the checklist; `added_steps` is the separate, smaller signal.
    pub fn deviations(&self) -> usize {
        self.steps
            .iter()
            .filter(|(id, state)| {
                state.status == StepStatus::Deviated && !crate::vocab::is_added_step_id(id)
            })
            .count()
    }

    /// How many steps were added to the run after it started.
    pub fn added_steps_count(&self) -> usize {
        self.added_steps.len()
    }
}

/// Apply one event to the state. The rules here are the ones that do not need the
/// content; checks that need the resolved checklist (a step id exists, a capture key is
/// declared, a checkbox index is in range) live in `sop-repo`'s validator.
pub fn apply(state: &mut RunState, event: &RunEvent) -> Result<(), RunError> {
    if state.started.is_none() {
        let RunEvent::RunStarted {
            at,
            sop,
            sop_version,
            sop_commit,
            snapshot_sha256,
            operator,
            site,
            plan,
            case,
            sensor,
            hardware,
            conditions,
            clock,
        } = event
        else {
            return Err(RunError::MustStartWithRunStarted);
        };
        state.started = Some(at.clone());
        state.sop = Some(sop.clone());
        state.sop_version = Some(sop_version.clone());
        state.sop_commit = Some(sop_commit.clone());
        state.snapshot_sha256 = Some(snapshot_sha256.clone());
        state.operator = Some(operator.clone());
        state.site = Some(site.clone());
        state.plan = plan.clone();
        state.case = case.clone();
        state.sensor = sensor.clone();
        state.hardware = hardware.clone();
        state.conditions = conditions.clone();
        state.clock = clock.clone();
        return Ok(());
    }

    if let RunEvent::RunStarted { .. } = event {
        return Err(RunError::AlreadyStarted);
    }
    if state.is_ended() && !matches!(event, RunEvent::RunConcluded { .. }) {
        return Err(RunError::Ended);
    }

    match event {
        RunEvent::StepOpened { step, at, .. } => {
            let step_state = state.steps.entry(step.clone()).or_default();
            // Reopening a step moves its start forward: the window is the last time the
            // operator was actually working on it.
            step_state.opened_at = Some(at.clone());
        }
        RunEvent::StepAdded { id, title, after, .. } => {
            state.steps.entry(id.clone()).or_default();
            state.added_steps.push(AddedStep {
                id: id.clone(),
                title: title.clone(),
                after: after.clone(),
            });
        }
        RunEvent::FieldMarker { label, .. } => {
            state.markers.push(FieldMarker {
                at: event.at(),
                label: label.clone(),
            });
        }
        RunEvent::CheckboxToggled {
            step, index, checked, ..
        } => {
            let step_state = state.steps.entry(step.clone()).or_default();
            if *index >= step_state.checkboxes.len() {
                return Err(RunError::CheckboxOutOfRange {
                    index: *index,
                    len: step_state.checkboxes.len(),
                });
            }
            step_state.checkboxes[*index] = *checked;
        }
        RunEvent::CaptureRecorded {
            step,
            key,
            value,
            unit,
            ..
        } => {
            let step_state = state.steps.entry(step.clone()).or_default();
            step_state.captures.insert(
                key.clone(),
                CaptureValue {
                    value: value.clone(),
                    unit: unit.clone(),
                },
            );
        }
        RunEvent::CaptureCleared { step, key, .. } => {
            let step_state = state.steps.entry(step.clone()).or_default();
            step_state.captures.remove(key);
            step_state.acknowledged.remove(key);
        }
        RunEvent::CaptureAcknowledged { step, key, reason, .. } => {
            let step_state = state.steps.entry(step.clone()).or_default();
            if reason.as_deref().is_some_and(|r| r.trim().is_empty()) {
                return Err(RunError::ReasonRequired("acknowledged"));
            }
            step_state.acknowledged.insert(key.clone());
        }
        RunEvent::NoteAdded { step, text, .. } => {
            match step {
                Some(id) => state.steps.entry(id.clone()).or_default().notes.push(text.clone()),
                None => state.run_notes.push(text.clone()),
            }
        }
        RunEvent::AttachmentAdded {
            step,
            path,
            sha256,
            size,
            t_min,
            t_max,
            row_count,
            ..
        } => {
            let attachment = Attachment {
                path: path.clone(),
                sha256: sha256.clone(),
                size: *size,
                t_min: t_min.clone(),
                t_max: t_max.clone(),
                row_count: *row_count,
            };
            match step {
                Some(id) => state.steps.entry(id.clone()).or_default().attachments.push(attachment),
                None => state.run_attachments.push(attachment),
            }
        }
        RunEvent::StepStatusChanged {
            step,
            status,
            reason,
            ..
        } => {
            let parsed = StepStatus::parse(status)?;
            if (parsed == StepStatus::Skipped || parsed == StepStatus::Deviated)
                && reason.as_deref().is_none_or(str::is_empty)
            {
                return Err(RunError::ReasonRequired("skipped/deviated"));
            }
            let step_state = state.steps.entry(step.clone()).or_default();
            step_state.status = parsed.clone();
            step_state.reason = reason.clone();
            // done / skipped / deviated close the step's time window; reopening clears it.
            if parsed == StepStatus::Open {
                step_state.ended_at = None;
            } else {
                step_state.ended_at = Some(event.at());
            }
        }
        RunEvent::RunEnded { status, .. } => {
            if !RUN_STATUS.contains(&status.as_str()) {
                return Err(RunError::BadRunStatus(status.clone(), RUN_STATUS.join(", ")));
            }
            state.run_status = Some(status.clone());
            state.ended = Some(event.at());
        }
        RunEvent::RunConcluded { conclusion, summary, .. } => {
            if state.ended.is_none() {
                return Err(RunError::ConclusionBeforeEnd);
            }
            if !RUN_CONCLUSION.contains(&conclusion.as_str()) {
                return Err(RunError::BadConclusion(conclusion.clone(), RUN_CONCLUSION.join(", ")));
            }
            if summary.trim().is_empty() {
                return Err(RunError::ConclusionNoteRequired);
            }
            state.conclusion = Some(conclusion.clone());
            state.conclusion_note = Some(summary.trim().to_owned());
        }
        RunEvent::RunStarted { .. } => unreachable!("handled above"),
    }
    Ok(())
}

impl RunEvent {
    fn at(&self) -> String {
        match self {
            RunEvent::RunStarted { at, .. }
            | RunEvent::StepOpened { at, .. }
            | RunEvent::StepAdded { at, .. }
            | RunEvent::FieldMarker { at, .. }
            | RunEvent::CheckboxToggled { at, .. }
            | RunEvent::CaptureRecorded { at, .. }
            | RunEvent::CaptureCleared { at, .. }
            | RunEvent::CaptureAcknowledged { at, .. }
            | RunEvent::NoteAdded { at, .. }
            | RunEvent::AttachmentAdded { at, .. }
            | RunEvent::StepStatusChanged { at, .. }
            | RunEvent::RunEnded { at, .. }
            | RunEvent::RunConcluded { at, .. } => at.clone(),
        }
    }

    /// Replace the event's timestamp with the one the tool believes in.
    ///
    /// The renderer sends no timestamp at all; even if it did, the tool's own clock is the
    /// authority, because the run's time line has to be one consistent clock (D4).
    pub fn stamp(&mut self, at: String) {
        match self {
            RunEvent::RunStarted { at: slot, .. }
            | RunEvent::StepOpened { at: slot, .. }
            | RunEvent::StepAdded { at: slot, .. }
            | RunEvent::FieldMarker { at: slot, .. }
            | RunEvent::CheckboxToggled { at: slot, .. }
            | RunEvent::CaptureRecorded { at: slot, .. }
            | RunEvent::CaptureCleared { at: slot, .. }
            | RunEvent::CaptureAcknowledged { at: slot, .. }
            | RunEvent::NoteAdded { at: slot, .. }
            | RunEvent::AttachmentAdded { at: slot, .. }
            | RunEvent::StepStatusChanged { at: slot, .. }
            | RunEvent::RunEnded { at: slot, .. }
            | RunEvent::RunConcluded { at: slot, .. } => *slot = at,
        }
    }
}

/// Replay a whole log over an empty state.
pub fn replay(events: &[RunEvent]) -> Result<RunState, RunError> {
    let mut state = RunState::default();
    for event in events {
        apply(&mut state, event)?;
    }
    Ok(state)
}

/// The slice of a step the record renderer needs. Filled from the frozen snapshot.
#[derive(Debug, Clone)]
pub struct RecordStep {
    pub id: String,
    pub title: String,
    pub body: String,
    pub checkbox_count: usize,
}

fn yaml_scalar(value: &str) -> String {
    if value.is_empty()
        || value.starts_with(|c: char| c.is_ascii_digit())
        || value.contains([':', '#', '{', '}', '[', ']', ',', '&', '*', '!', '|', '>', '"', '\''])
        || value.trim() != value
    {
        format!("{value:?}")
    } else {
        value.to_owned()
    }
}

/// Set one checkbox item's marker, leaving the bullet, the indent, and the text alone.
///
/// `note` items can contain a `[` of their own, so only the marker after the bullet is
/// touched. [`checkbox_state`](crate::check::checkbox_state) has already established that
/// the line has one.
fn set_checkbox_marker(line: &str, checked: bool) -> String {
    let Some(bracket) = line.find('[') else {
        return line.to_owned();
    };
    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..bracket + 1]);
    out.push(if checked { 'x' } else { ' ' });
    out.push_str(&line[bracket + 2..]);
    out
}

/// The step body with each checkbox marker set from the run's recorded state.
///
/// A step with more items than the record has state for leaves the extra ones unchecked,
/// which is what an item that was never touched should read as.
pub fn body_with_checkbox_state(body: &str, checked: &[bool]) -> String {
    let mut index = 0;
    let lines: Vec<String> = body
        .lines()
        .map(|line| {
            if crate::check::checkbox_state(line).is_none() {
                return line.to_owned();
            }
            let state = checked.get(index).copied().unwrap_or(false);
            index += 1;
            set_checkbox_marker(line, state)
        })
        .collect();
    lines.join("\n")
}

/// The text of every checkbox item in a step body, in order, without its marker.
///
/// The record keeps each item's text next to its marker; the execution view needs the
/// same text on its own, so the operator ticks the words they read rather than a number.
/// The count agrees with [`body_with_checkbox_state`], which walks the same lines.
pub fn checklist_items(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        if crate::check::checkbox_state(line).is_none() {
            continue;
        }
        let Some(bracket) = line.find(']') else {
            continue;
        };
        out.push(line[bracket + 1..].trim().to_owned());
    }
    out
}

/// Every checkbox item in a step body paired with what the run recorded against it.
///
/// The item text comes from the body frozen at run start; the ticks come from the event
/// log. An item the log has no answer for is unchecked, which is what an item nobody
/// touched should read as, so the two lengths need not agree.
pub fn checklist_state(body: &str, checked: &[bool]) -> Vec<(String, bool)> {
    checklist_items(body)
        .into_iter()
        .enumerate()
        .map(|(index, text)| (text, checked.get(index).copied().unwrap_or(false)))
        .collect()
}

/// The step body with its checkbox lines removed.
///
/// The execution view renders those items itself, from [`checklist_items`], so leaving
/// them in the prose would show every item twice: once as words and once as a tick box.
pub fn body_without_checklist(body: &str) -> String {
    body.lines()
        .filter(|line| crate::check::checkbox_state(line).is_none())
        .collect::<Vec<&str>>()
        .join("\n")
}

/// The time-of-day (`HH:MM:SSZ`) part of a timestamp this build writes, for a timeline
/// where the run is a single session. The full date stays in the step's `yaml result`.
fn time_of_day(value: &str) -> String {
    value.get(11..).unwrap_or(value).to_owned()
}

/// The ordered list of steps a run actually executed: the frozen snapshot's steps, with
/// the steps added mid-run inserted after the step they named (or at the end when they
/// named none, or the anchor is gone). This is the same order the app shows, so the
/// timeline and the execution view agree.
pub fn run_step_order(state: &RunState, steps: &[RecordStep]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for step in steps {
        out.push((step.id.clone(), step.title.clone()));
        for added in &state.added_steps {
            if added.after.as_deref() == Some(step.id.as_str()) {
                out.push((added.id.clone(), added.title.clone()));
            }
        }
    }
    // Any added step whose anchor is another added step, is gone, or was `None` lands at
    // the end, in the order it was added.
    let anchored: Vec<String> = steps.iter().map(|s| s.id.clone()).collect();
    for added in &state.added_steps {
        let placed = added.after.as_deref().is_some_and(|target| anchored.contains(&target.to_owned()));
        if !placed {
            out.push((added.id.clone(), added.title.clone()));
        }
    }
    out
}

/// The `## Timeline` table: one row per step with a recorded start or end, so a script
/// can join a data log to the step that collected it by time.
fn render_timeline(state: &RunState, steps: &[RecordStep]) -> String {
    let mut out = String::from("## Timeline\n\n");
    out.push_str("| Step | Title | Start (UTC) | End (UTC) | Status | Duration |\n");
    out.push_str("|------|-------|-------------|-----------|--------|----------|\n");
    for (id, title) in run_step_order(state, steps) {
        let step_state = state.steps.get(&id);
        let opened = step_state.and_then(|s| s.opened_at.clone());
        let ended = step_state.and_then(|s| s.ended_at.clone());
        let status = step_state.map(|s| s.status.as_str()).unwrap_or("open");
        let duration = match (&opened, &ended) {
            (Some(start), Some(end)) => match (epoch_seconds(start), epoch_seconds(end)) {
                (Some(a), Some(b)) if b >= a => format_duration(b - a),
                _ => "—".to_owned(),
            },
            _ => "—".to_owned(),
        };
        let t = |value: &Option<String>| {
            value.as_deref().map(time_of_day).unwrap_or_else(|| "—".to_owned())
        };
        out.push_str(&format!(
            "| {id} | {title} | {} | {} | {status} | {duration} |\n",
            t(&opened),
            t(&ended)
        ));
    }
    out.push('\n');
    out
}

/// The same information as the timeline table, as comma-separated rows for a script.
pub fn timeline_csv(state: &RunState, steps: &[RecordStep]) -> String {
    let mut out = String::from("step,title,start_utc,end_utc,status,duration\n");
    for (id, title) in run_step_order(state, steps) {
        let step_state = state.steps.get(&id);
        let opened = step_state.and_then(|s| s.opened_at.clone()).unwrap_or_default();
        let ended = step_state.and_then(|s| s.ended_at.clone()).unwrap_or_default();
        let status = step_state.map(|s| s.status.as_str()).unwrap_or("open");
        let duration = match (step_state.and_then(|s| s.opened_at.clone()), step_state.and_then(|s| s.ended_at.clone())) {
            (Some(start), Some(end)) => match (epoch_seconds(&start), epoch_seconds(&end)) {
                (Some(a), Some(b)) if b >= a => format_duration(b - a),
                _ => String::new(),
            },
            _ => String::new(),
        };
        let csv = |value: &str| format!("\"{value}\"");
        out.push_str(&format!(
            "{},{},{},{},{},{}\n",
            csv(&id),
            csv(&title),
            csv(&opened),
            csv(&ended),
            csv(status),
            csv(&duration)
        ));
    }
    out
}

/// Render the human reviewable `record.md` for a run.
///
/// `steps` is the checklist as frozen at run start (id, title, prose, and how many
/// checkboxes the step has), so the record is reviewable even after the content changes.
pub fn render_record(state: &RunState, steps: &[RecordStep]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Run {} ({})\n\n",
        state.sop.as_deref().unwrap_or("?"),
        state.started.as_deref().unwrap_or("?")
    ));
    out.push_str(&format!(
        "operator: {}\n\nsite: {}\n\n",
        state.operator.as_deref().unwrap_or(""),
        state.site.as_deref().unwrap_or("")
    ));
    if let Some(plan) = &state.plan {
        out.push_str(&format!("plan: {plan}\n"));
        if let Some(case) = &state.case {
            out.push_str(&format!("case: {case}\n"));
        }
        out.push('\n');
    }
    if let (Some(version), Some(commit)) = (state.sop_version.as_deref(), state.sop_commit.as_deref()) {
        out.push_str(&format!("sop_version: {version}\nsop_commit: {commit}\n"));
    }
    // The head is a restatement of the front matter for a reader who has only the
    // record, so the instrument and the conditions are repeated here too. They are the
    // facts a later re-analysis needs to compare two runs, and front matter is easy to
    // scroll past.
    if let Some(sensor) = &state.sensor
        && !sensor.is_empty()
    {
        let mut parts = Vec::new();
        if !sensor.model.is_empty() {
            parts.push(sensor.model.clone());
        }
        if !sensor.serial.is_empty() {
            parts.push(format!("serial {}", sensor.serial));
        }
        if !sensor.firmware.is_empty() {
            parts.push(format!("firmware {}", sensor.firmware));
        }
        out.push_str(&format!("sensor: {}\n", parts.join(", ")));
    }
    if !state.hardware.is_empty() {
        out.push_str(&format!("hardware: {}\n", state.hardware.join(", ")));
    }
    if !state.conditions.is_empty() {
        let pairs: Vec<String> = state
            .conditions
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        out.push_str(&format!("conditions: {}\n", pairs.join(", ")));
    }
    out.push('\n');

    out.push_str(&render_timeline(state, steps));

    for step in steps {
        let step_state = state.steps.get(&step.id);
        out.push_str(&format!("## {}\n\n", step.title));
        if !step.body.trim().is_empty() {
            // The body is the checklist template, so its items were written `- [ ]`.
            // The record has to say what the operator actually left them as, which is
            // the whole reason the item text and its marker are kept together.
            let checked: &[bool] = step_state.map_or(&[], |state| state.checkboxes.as_slice());
            out.push_str(body_with_checkbox_state(&step.body, checked).trim_end());
            out.push_str("\n\n");
        }
        let Some(state) = step_state else {
            out.push_str("_(not reached)_\n\n");
            continue;
        };
        // A step left open still keeps its notes and attachments below: they are the
        // part of a record that nobody can reconstruct later, so being unfinished is not
        // a reason to drop them. A result block is emitted whenever it has something to
        // say - an outcome, a reason, or captures - and omitting `status` means the step
        // happened but the operator recorded no outcome for it. That is different from
        // `done`, which is a judgement. A block with nothing to say is left out.
        let has_result = state.status != StepStatus::Open
            || state.reason.is_some()
            || !state.captures.is_empty()
            || !state.acknowledged.is_empty();
        if has_result {
            out.push_str("```yaml result\n");
            out.push_str(&format!("step: {}\n", step.id));
            if state.status != StepStatus::Open {
                out.push_str(&format!("status: {}\n", state.status.as_str()));
            }
            if let Some(reason) = &state.reason {
                out.push_str(&format!("reason: {}\n", yaml_scalar(reason)));
            }
            if let Some(opened) = &state.opened_at {
                out.push_str(&format!("opened_at: {}\n", yaml_scalar(opened)));
            }
            if let Some(ended) = &state.ended_at {
                out.push_str(&format!("ended_at: {}\n", yaml_scalar(ended)));
            }
            if !state.captures.is_empty() {
                out.push_str("captures:\n");
                for (key, value) in &state.captures {
                    let rendered = match (&value.unit, value.value.parse::<f64>()) {
                        (Some(unit), _) => {
                            format!("{} {}", yaml_scalar(&value.value), yaml_scalar(unit))
                        }
                        (None, Ok(_)) | (None, Err(_)) => yaml_scalar(&value.value),
                    };
                    out.push_str(&format!("  {key}: {rendered}\n"));
                }
            }
            if !state.acknowledged.is_empty() {
                out.push_str("acknowledged:\n");
                for key in &state.acknowledged {
                    out.push_str(&format!("  - {key}\n"));
                }
            }
            out.push_str("```\n\n");
        }
        let mut tail = false;
        for note in &state.notes {
            // A note may be multi-line Markdown. Prefixing every line keeps the whole
            // note inside one blockquote instead of letting the second line escape into
            // the step's prose.
            for (i, line) in note.split('\n').enumerate() {
                if i == 0 {
                    out.push_str(&format!("> note: {line}\n"));
                } else if line.trim().is_empty() {
                    out.push_str(">\n");
                } else {
                    out.push_str(&format!("> {line}\n"));
                }
            }
            tail = true;
        }
        for attachment in &state.attachments {
            out.push_str(&format!(
                "> attachment: {path} (sha256 {sha256}, {size} bytes)\n",
                path = attachment.path,
                sha256 = &attachment.sha256[..attachment.sha256.len().min(12)],
                size = attachment.size
            ));
            tail = true;
        }
        // Notes and attachments end without a blank line of their own, so one is added
        // here. Without it the next `##` would sit directly under the last note.
        if tail {
            out.push('\n');
        }
    }

    if !state.added_steps.is_empty() {
        out.push_str("## Steps added during the run\n\n");
        out.push_str("| ID | 标题 | 插入位置 | 状态 | 开始 | 结束 |\n");
        out.push_str("|----|------|----------|------|------|------|\n");
        for added in &state.added_steps {
            let step_state = state.steps.get(&added.id);
            let after = added
                .after
                .clone()
                .map(|target| format!("after {target}"))
                .unwrap_or_else(|| "end".to_owned());
            let status = step_state.map(|s| s.status.as_str()).unwrap_or("open");
            let opened = step_state.and_then(|s| s.opened_at.clone());
            let ended = step_state.and_then(|s| s.ended_at.clone());
            let t = |value: &Option<String>| {
                value.as_deref().map(time_of_day).unwrap_or_else(|| "—".to_owned())
            };
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                added.id,
                added.title,
                after,
                status,
                t(&opened),
                t(&ended)
            ));
        }
        out.push('\n');
    }

    if !state.markers.is_empty() {
        out.push_str("## Field markers\n\n");
        for marker in &state.markers {
            out.push_str(&format!("- {label} at {at}\n", label = marker.label, at = marker.at));
        }
        out.push('\n');
    }

    if !state.run_notes.is_empty() {
        out.push_str("## Run notes\n\n");
        for note in &state.run_notes {
            // Continuation lines are indented so they stay inside the same list item.
            for (i, line) in note.split('\n').enumerate() {
                if i == 0 {
                    out.push_str(&format!("- {line}\n"));
                } else if line.trim().is_empty() {
                    out.push('\n');
                } else {
                    out.push_str(&format!("  {line}\n"));
                }
            }
        }
        out.push('\n');
    }
    if !state.run_attachments.is_empty() {
        out.push_str("## Run attachments\n\n");
        for attachment in &state.run_attachments {
            out.push_str(&format!(
                "- {path} (sha256 {sha256})\n",
                path = attachment.path,
                sha256 = &attachment.sha256[..attachment.sha256.len().min(12)]
            ));
        }
        out.push('\n');
    }

    if let Some(status) = &state.run_status {
        out.push_str(&format!("\nstatus: {status}\n"));
    }
    if let Some(conclusion) = &state.conclusion {
        out.push_str(&format!("conclusion: {conclusion}\n"));
        if let Some(note) = &state.conclusion_note {
            out.push_str(&format!("conclusion_note: {note}\n"));
        }
    }
    out.push_str(&format!("deviations_count: {}\n", state.deviations()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started() -> RunEvent {
        RunEvent::RunStarted {
            at: "2026-09-25T00:00:00Z".to_owned(),
            sop: "ground-walk-survey".to_owned(),
            sop_version: "1".to_owned(),
            sop_commit: "abc123".to_owned(),
            snapshot_sha256: "deadbeef".to_owned(),
            operator: "leo".to_owned(),
            site: "Renfrew 395".to_owned(),
            plan: None,
            case: None,
            sensor: None,
            hardware: Vec::new(),
            conditions: BTreeMap::new(),
            clock: None,
        }
    }

    #[test]
    fn replay_builds_state_in_order() {
        let events = vec![
            started(),
            RunEvent::StepOpened { at: "t1".into(), step: "cond-location".into() },
            RunEvent::CaptureRecorded {
                at: "t2".into(),
                step: "cond-location".into(),
                key: "session_location".into(),
                value: "hill top".into(),
                unit: None,
            },
            RunEvent::StepStatusChanged {
                at: "t3".into(),
                step: "cond-location".into(),
                status: "done".into(),
                reason: None,
            },
            RunEvent::RunEnded { at: "t4".into(), status: "complete".into() },
        ];
        let state = replay(&events).unwrap();
        assert_eq!(state.sop.as_deref(), Some("ground-walk-survey"));
        assert_eq!(state.run_status.as_deref(), Some("complete"));
        assert!(state.is_ended());
        let step = state.step("cond-location").unwrap();
        assert_eq!(step.status, StepStatus::Done);
        assert_eq!(step.captures["session_location"].value, "hill top");
        assert_eq!(state.deviations(), 0);
    }

    #[test]
    fn first_event_must_be_run_started() {
        let event = RunEvent::StepOpened { at: "t".into(), step: "s".into() };
        assert_eq!(apply(&mut RunState::default(), &event), Err(RunError::MustStartWithRunStarted));
    }

    #[test]
    fn no_events_after_ended() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(&mut state, &RunEvent::RunEnded { at: "t".into(), status: "complete".into() }).unwrap();
        let late = RunEvent::StepOpened { at: "t2".into(), step: "s".into() };
        assert_eq!(apply(&mut state, &late), Err(RunError::Ended));
    }

    #[test]
    fn skip_requires_reason() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        let skip = RunEvent::StepStatusChanged {
            at: "t".into(),
            step: "s".into(),
            status: "skipped".into(),
            reason: None,
        };
        assert_eq!(apply(&mut state, &skip), Err(RunError::ReasonRequired("skipped/deviated")));
    }

    #[test]
    fn deviated_counts_and_skipped_does_not() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(
            &mut state,
            &RunEvent::StepStatusChanged {
                at: "t".into(),
                step: "a".into(),
                status: "deviated".into(),
                reason: Some("noise".into()),
            },
        )
        .unwrap();
        apply(
            &mut state,
            &RunEvent::StepStatusChanged {
                at: "t".into(),
                step: "b".into(),
                status: "skipped".into(),
                reason: Some("weather".into()),
            },
        )
        .unwrap();
        assert_eq!(state.deviations(), 1);
    }

    #[test]
    fn a_run_conclusion_is_recorded_after_the_run_ends() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        // A conclusion before the run ends is refused: the operator's verdict has no
        // meaning on a run that is still being executed.
        let premature = RunEvent::RunConcluded {
            at: "t".into(),
            conclusion: "pass".into(),
            summary: "all good".into(),
        };
        assert_eq!(apply(&mut state, &premature), Err(RunError::ConclusionBeforeEnd));
        apply(&mut state, &RunEvent::RunEnded { at: "t".into(), status: "complete".into() }).unwrap();
        apply(&mut state, &premature).unwrap();
        assert_eq!(state.conclusion.as_deref(), Some("pass"));
        assert_eq!(state.conclusion_note.as_deref(), Some("all good"));
    }

    #[test]
    fn a_conclusion_must_be_a_vocabulary_value_with_a_note() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(&mut state, &RunEvent::RunEnded { at: "t".into(), status: "complete".into() }).unwrap();
        let bad = RunEvent::RunConcluded {
            at: "t".into(),
            conclusion: "great".into(),
            summary: "it went well".into(),
        };
        assert_eq!(
            apply(&mut state, &bad),
            Err(RunError::BadConclusion("great".into(), RUN_CONCLUSION.join(", ")))
        );
        let no_note = RunEvent::RunConcluded {
            at: "t".into(),
            conclusion: "fail".into(),
            summary: "   ".into(),
        };
        assert_eq!(apply(&mut state, &no_note), Err(RunError::ConclusionNoteRequired));
    }

    #[test]
    fn a_concluded_run_restates_its_verdict_in_the_record() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(&mut state, &RunEvent::RunEnded { at: "t".into(), status: "complete".into() }).unwrap();
        apply(
            &mut state,
            &RunEvent::RunConcluded {
                at: "t".into(),
                conclusion: "inconclusive".into(),
                summary: "mains noise drifted all afternoon".into(),
            },
        )
        .unwrap();

        let rendered = render_record(&state, &[]);

        assert!(rendered.contains("conclusion: inconclusive"), "{rendered}");
        assert!(
            rendered.contains("conclusion_note: mains noise drifted all afternoon"),
            "{rendered}"
        );
    }

    #[test]
    fn checkbox_out_of_range_is_rejected() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(&mut state, &RunEvent::StepOpened { at: "t".into(), step: "s".into() }).unwrap();
        let toggle = RunEvent::CheckboxToggled {
            at: "t".into(),
            step: "s".into(),
            index: 2,
            checked: true,
        };
        assert_eq!(
            apply(&mut state, &toggle),
            Err(RunError::CheckboxOutOfRange { index: 2, len: 0 })
        );
    }

    #[test]
    fn a_records_checkbox_items_carry_what_the_operator_left_them_as() {
        let body = "Read this.\n\n- [ ] first item\n- [ ] second item\n* [X] third item\n\nDone.";
        let rendered = body_with_checkbox_state(body, &[true, false]);
        assert_eq!(
            rendered,
            "Read this.\n\n- [x] first item\n- [ ] second item\n* [ ] third item\n\nDone."
        );
    }

    #[test]
    fn a_checkbox_the_record_has_no_state_for_reads_as_unchecked() {
        let body = "- [ ] one\n- [x] two";
        assert_eq!(
            body_with_checkbox_state(body, &[true]),
            "- [x] one\n- [ ] two"
        );
    }

    #[test]
    fn a_step_left_open_keeps_its_notes_even_though_it_has_no_result() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(
            &mut state,
            &RunEvent::NoteAdded {
                at: "t".into(),
                step: Some("s".into()),
                text: "the stand rocked once".into(),
            },
        )
        .unwrap();
        let steps = vec![RecordStep {
            id: "s".into(),
            title: "Warm-up baseline".into(),
            body: "Prose.".into(),
            checkbox_count: 0,
        }];

        let rendered = render_record(&state, &steps);

        // An open step with nothing recorded gets no result block at all: the absence of
        // an outcome is itself the record, and a placeholder would be one more thing to
        // read past.
        assert!(!rendered.contains("yaml result"), "{rendered}");
        assert!(
            rendered.contains("> note: the stand rocked once"),
            "a note the operator typed must survive an unfinished step: {rendered}"
        );
    }

    #[test]
    fn an_open_step_with_captures_renders_them_without_a_status() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(
            &mut state,
            &RunEvent::CaptureRecorded {
                at: "t".into(),
                step: "s".into(),
                key: "sigma_nt".into(),
                value: "0.08".into(),
                unit: Some("nT".into()),
            },
        )
        .unwrap();
        let steps = vec![RecordStep {
            id: "s".into(),
            title: "Static noise".into(),
            body: String::new(),
            checkbox_count: 0,
        }];

        let rendered = render_record(&state, &steps);

        // The data exists, so it is kept; the operator gave no outcome, so there is no
        // status line. That is the difference between `_` and `done`.
        assert!(rendered.contains("```yaml result"), "{rendered}");
        assert!(rendered.contains("step: s"), "{rendered}");
        assert!(rendered.contains("sigma_nt: \"0.08\" nT"), "{rendered}");
        assert!(!rendered.contains("status:"), "no outcome means no status line: {rendered}");
    }

    #[test]
    fn a_note_between_steps_leaves_exactly_one_blank_line() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(&mut state, &RunEvent::StepOpened { at: "t".into(), step: "a".into() }).unwrap();
        apply(
            &mut state,
            &RunEvent::NoteAdded { at: "t".into(), step: Some("a".into()), text: "note".into() },
        )
        .unwrap();
        apply(&mut state, &RunEvent::StepOpened { at: "t".into(), step: "b".into() }).unwrap();
        let steps = vec![
            RecordStep { id: "a".into(), title: "A".into(), body: String::new(), checkbox_count: 0 },
            RecordStep { id: "b".into(), title: "B".into(), body: String::new(), checkbox_count: 0 },
        ];

        let rendered = render_record(&state, &steps);

        assert!(rendered.contains("> note: note\n\n## B"), "one blank line before the next step: {rendered}");
        assert!(!rendered.contains("\n\n\n"), "no doubled blank lines: {rendered}");
    }

    #[test]
    fn a_multi_line_step_note_stays_in_one_blockquote() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(
            &mut state,
            &RunEvent::NoteAdded {
                at: "t".into(),
                step: Some("s".into()),
                text: "first line\n\n- a list item\nsecond line".into(),
            },
        )
        .unwrap();
        let steps = vec![RecordStep {
            id: "s".into(),
            title: "Warm-up baseline".into(),
            body: "Prose.".into(),
            checkbox_count: 0,
        }];

        let rendered = render_record(&state, &steps);

        assert!(
            rendered.contains("> note: first line\n>\n> - a list item\n> second line\n"),
            "every line of the note keeps its blockquote marker: {rendered}"
        );
    }

    #[test]
    fn a_multi_line_run_note_stays_in_one_list_item() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(
            &mut state,
            &RunEvent::NoteAdded {
                at: "t".into(),
                step: None,
                text: "line one\nline two".into(),
            },
        )
        .unwrap();

        let rendered = render_record(&state, &[]);

        assert!(
            rendered.contains("## Run notes\n\n- line one\n  line two\n"),
            "continuation lines are indented under the bullet: {rendered}"
        );
    }

    #[test]
    fn checklist_state_pairs_each_item_with_its_tick() {
        let body = "- [ ] first\n- [ ] second\n- [ ] third";
        assert_eq!(
            checklist_state(body, &[true, false]),
            vec![
                ("first".to_owned(), true),
                ("second".to_owned(), false),
                ("third".to_owned(), false),
            ]
        );
    }

    #[test]
    fn body_without_checklist_keeps_the_prose_and_drops_the_items() {
        let body = "Read this.\n\n- [ ] first item\n- [x] second item\n\nThen this.";
        assert_eq!(body_without_checklist(body), "Read this.\n\n\nThen this.");
    }

    #[test]
    fn checklist_items_carry_the_text_and_drop_the_marker() {
        let body =
            "Read this.\n\n- [ ] first item\n- [x] second item\n* [ ] third [[link]]\n\nDone.";
        assert_eq!(
            checklist_items(body),
            vec!["first item", "second item", "third [[link]]"]
        );
    }

    #[test]
    fn a_body_without_items_has_no_checklist_items() {
        assert!(checklist_items("Just prose.\n\nAnd a plain list:\n- one").is_empty());
    }

    #[test]
    fn only_the_marker_a_bullet_owns_is_rewritten() {
        // A checkbox item may contain brackets of its own; the item text has to survive.
        let body = "- [ ] record the value of `a[0]` and [x] in the field";
        let rendered = body_with_checkbox_state(body, &[true]);
        assert_eq!(
            rendered,
            "- [x] record the value of `a[0]` and [x] in the field"
        );
    }

    #[test]
    fn stamp_overwrites_a_stale_renderer_timestamp() {
        let mut event = RunEvent::CaptureRecorded {
            at: "1999-01-01T00:00:00Z".to_owned(),
            step: "s".into(),
            key: "k".into(),
            value: "v".into(),
            unit: None,
        };
        event.stamp("2026-09-25T12:00:00Z".to_owned());
        assert_eq!(event.at(), "2026-09-25T12:00:00Z");
    }

    #[test]
    fn an_opened_then_closed_step_forms_a_time_window() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(&mut state, &RunEvent::StepOpened { at: "2026-09-25T12:00:00Z".into(), step: "s".into() }).unwrap();
        apply(&mut state, &RunEvent::StepStatusChanged { at: "2026-09-25T12:03:15Z".into(), step: "s".into(), status: "done".into(), reason: None }).unwrap();
        let step = state.step("s").unwrap();
        assert_eq!(step.opened_at.as_deref(), Some("2026-09-25T12:00:00Z"));
        assert_eq!(step.ended_at.as_deref(), Some("2026-09-25T12:03:15Z"));
    }

    #[test]
    fn closing_without_opening_leaves_no_start() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(&mut state, &RunEvent::StepStatusChanged { at: "2026-09-25T12:00:00Z".into(), step: "s".into(), status: "done".into(), reason: None }).unwrap();
        assert_eq!(state.step("s").unwrap().opened_at, None);
        assert_eq!(state.step("s").unwrap().ended_at.as_deref(), Some("2026-09-25T12:00:00Z"));
    }

    #[test]
    fn reopening_a_step_moves_its_start_forward() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(&mut state, &RunEvent::StepOpened { at: "2026-09-25T12:00:00Z".into(), step: "s".into() }).unwrap();
        apply(&mut state, &RunEvent::StepStatusChanged { at: "2026-09-25T12:05:00Z".into(), step: "s".into(), status: "done".into(), reason: None }).unwrap();
        apply(&mut state, &RunEvent::StepStatusChanged { at: "2026-09-25T12:10:00Z".into(), step: "s".into(), status: "open".into(), reason: Some("reopen".into()) }).unwrap();
        apply(&mut state, &RunEvent::StepOpened { at: "2026-09-25T12:11:00Z".into(), step: "s".into() }).unwrap();
        apply(&mut state, &RunEvent::StepStatusChanged { at: "2026-09-25T12:12:00Z".into(), step: "s".into(), status: "done".into(), reason: None }).unwrap();
        let step = state.step("s").unwrap();
        assert_eq!(step.opened_at.as_deref(), Some("2026-09-25T12:11:00Z"));
        assert_eq!(step.ended_at.as_deref(), Some("2026-09-25T12:12:00Z"));
        assert_eq!(step.status, StepStatus::Done);
    }

    #[test]
    fn a_step_added_mid_run_is_merged_at_replay() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(&mut state, &RunEvent::StepAdded { at: "2026-09-25T12:00:00Z".into(), id: "adhoc-001".into(), title: "Extra line".into(), after: Some("s1".into()) }).unwrap();
        apply(&mut state, &RunEvent::StepOpened { at: "2026-09-25T12:01:00Z".into(), step: "adhoc-001".into() }).unwrap();
        apply(&mut state, &RunEvent::StepStatusChanged { at: "2026-09-25T12:02:00Z".into(), step: "adhoc-001".into(), status: "done".into(), reason: None }).unwrap();
        assert_eq!(state.added_steps.len(), 1);
        assert_eq!(state.added_steps[0].id, "adhoc-001");
        assert_eq!(state.added_steps[0].after.as_deref(), Some("s1"));
        assert_eq!(state.step("adhoc-001").unwrap().status, StepStatus::Done);
    }

    #[test]
    fn an_attachment_carries_a_time_range_when_present() {
        let mut state = RunState::default();
        apply(&mut state, &started()).unwrap();
        apply(&mut state, &RunEvent::AttachmentAdded {
            at: "2026-09-25T12:00:00Z".into(),
            step: None,
            path: "logs/mag_raw.csv".into(),
            sha256: "abcd".into(),
            size: 10,
            t_min: Some("2026-09-25T12:00:00Z".into()),
            t_max: Some("2026-09-25T12:05:00Z".into()),
            row_count: Some(300),
        }).unwrap();
        let att = &state.run_attachments[0];
        assert_eq!(att.t_min.as_deref(), Some("2026-09-25T12:00:00Z"));
        assert_eq!(att.row_count, Some(300));
    }
}
