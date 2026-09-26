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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::vocab::{RESULT_STATUS, RUN_STATUS};

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
    },
    StepOpened {
        at: String,
        step: String,
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
}

/// The state of one step during a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepState {
    pub status: StepStatus,
    pub reason: Option<String>,
    pub captures: BTreeMap<String, CaptureValue>,
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
    pub started: Option<String>,
    pub ended: Option<String>,
    pub run_status: Option<String>,
    pub steps: BTreeMap<String, StepState>,
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
    pub fn deviations(&self) -> usize {
        self.steps
            .values()
            .filter(|state| state.status == StepStatus::Deviated)
            .count()
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
        return Ok(());
    }

    if let RunEvent::RunStarted { .. } = event {
        return Err(RunError::AlreadyStarted);
    }
    if state.is_ended() {
        return Err(RunError::Ended);
    }

    match event {
        RunEvent::StepOpened { step, .. } => {
            state.steps.entry(step.clone()).or_default();
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
            ..
        } => {
            let attachment = Attachment {
                path: path.clone(),
                sha256: sha256.clone(),
                size: *size,
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
            step_state.status = parsed;
            step_state.reason = reason.clone();
        }
        RunEvent::RunEnded { status, .. } => {
            if !RUN_STATUS.contains(&status.as_str()) {
                return Err(RunError::BadRunStatus(status.clone(), RUN_STATUS.join(", ")));
            }
            state.run_status = Some(status.clone());
            state.ended = Some(event.at());
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
            | RunEvent::CheckboxToggled { at, .. }
            | RunEvent::CaptureRecorded { at, .. }
            | RunEvent::CaptureCleared { at, .. }
            | RunEvent::NoteAdded { at, .. }
            | RunEvent::AttachmentAdded { at, .. }
            | RunEvent::StepStatusChanged { at, .. }
            | RunEvent::RunEnded { at, .. } => at.clone(),
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
    if let (Some(version), Some(commit)) = (state.sop_version.as_deref(), state.sop_commit.as_deref()) {
        out.push_str(&format!("sop_version: {version}\nsop_commit: {commit}\n"));
    }
    out.push('\n');

    for step in steps {
        let step_state = state.steps.get(&step.id);
        out.push_str(&format!("## {}\n\n", step.title));
        if !step.body.trim().is_empty() {
            out.push_str(step.body.trim_end());
            out.push_str("\n\n");
        }
        let Some(state) = step_state else {
            out.push_str("_(not reached)_\n\n");
            continue;
        };
        if state.status == StepStatus::Open {
            out.push_str("_(not completed)_\n\n");
            continue;
        }
        out.push_str("```yaml result\n");
        out.push_str(&format!("step: {}\n", step.id));
        out.push_str(&format!("status: {}\n", state.status.as_str()));
        if let Some(reason) = &state.reason {
            out.push_str(&format!("reason: {}\n", yaml_scalar(reason)));
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
        out.push_str("```\n\n");
        for note in &state.notes {
            out.push_str(&format!("> note: {note}\n"));
        }
        for attachment in &state.attachments {
            out.push_str(&format!(
                "> attachment: {path} (sha256 {sha256}, {size} bytes)\n",
                path = attachment.path,
                sha256 = &attachment.sha256[..attachment.sha256.len().min(12)],
                size = attachment.size
            ));
        }
        if !state.checkboxes.is_empty() {
            out.push('\n');
        }
    }

    if !state.run_notes.is_empty() {
        out.push_str("## Run notes\n\n");
        for note in &state.run_notes {
            out.push_str(&format!("- {note}\n"));
        }
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
    }

    if let Some(status) = &state.run_status {
        out.push_str(&format!("\nstatus: {status}\n"));
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
}