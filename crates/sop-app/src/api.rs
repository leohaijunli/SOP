//! What the frontend sends and receives.
//!
//! The frontend is a renderer: it holds no rules, so every type here is either a query
//! result to display or an edit to hand to `sop-repo`, which decides whether it is
//! allowed. Names are camelCase on the wire because the other side is TypeScript.

use serde::{Deserialize, Serialize};
use sop_core::authoring::{CaptureDraft, ExpectedDraft, StepChanges, StepDraft};
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub working_copy: String,
    pub settings_path: String,
    pub project_title: Option<String>,
    pub project_id: Option<String>,
    pub institution: Option<String>,
    pub git: GitInfo,
    pub content: ContentCounts,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub is_repository: bool,
    pub branch: Option<String>,
    pub remote: String,
    pub url: Option<String>,
    pub url_has_credentials: bool,
    pub dirty: bool,
    pub ahead: Option<u64>,
    pub behind: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentCounts {
    pub procedures: usize,
    pub checklists: usize,
    pub runs: usize,
    pub help: usize,
    pub log_directories: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingRow {
    pub key: String,
    pub value: Option<String>,
    pub description: String,
    /// True when this key is kept in the working copy rather than on this machine.
    pub in_repository: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectField {
    pub key: String,
    pub value: Option<String>,
    pub description: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub path: String,
    pub fields: Vec<ProjectField>,
}

/// A capture as the form sends it.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureInput {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub capture_type: String,
    pub unit: Option<String>,
    pub required: bool,
    pub options: Vec<String>,
    pub expected: Option<ExpectedInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "form", rename_all = "camelCase")]
pub enum ExpectedInput {
    Range { min: Option<f64>, max: Option<f64> },
    Bool { value: bool },
    Select { value: String },
}

/// A run as the execution view renders it: step definitions merged with run state.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunView {
    pub sop: String,
    pub run_id: String,
    pub operator: Option<String>,
    pub site: Option<String>,
    pub started: Option<String>,
    pub ended: Option<String>,
    pub run_status: Option<String>,
    pub snapshot_sha256: Option<String>,
    pub sop_version: Option<String>,
    pub sop_commit: Option<String>,
    pub deviations_count: usize,
    pub sensor: Option<SensorView>,
    pub hardware: Vec<String>,
    pub conditions: BTreeMap<String, String>,
    pub steps: Vec<RunStepView>,
    pub run_notes: Vec<String>,
    pub run_attachments: Vec<RunAttachmentView>,
    pub record_path: String,
}

/// The instrument as the start panel entered it and the view displays it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorView {
    pub model: Option<String>,
    pub serial: Option<String>,
    pub firmware: Option<String>,
}

/// What the start panel sends along with the run's identity.
///
/// Everything is optional, so a run can start with none of it and the record simply has
/// no instrument block. `conditions` is a map so a checklist can add a dimension without
/// a change here; the panel edits it as `key: value` lines.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunMetaInput {
    #[serde(default)]
    pub sensor_model: Option<String>,
    #[serde(default)]
    pub sensor_serial: Option<String>,
    #[serde(default)]
    pub sensor_firmware: Option<String>,
    #[serde(default)]
    pub hardware: Vec<String>,
    #[serde(default)]
    pub conditions: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStepView {
    pub id: String,
    pub title: String,
    pub prose: String,
    pub severity: Option<String>,
    pub kind: Option<String>,
    pub status: String,
    pub reason: Option<String>,
    /// The step's checkbox items as frozen at run start, each with its tick state.
    pub checklist: Vec<ChecklistItemView>,
    pub captures: Vec<RunCaptureView>,
    pub notes: Vec<String>,
}

/// One checkbox item: the text the template wrote, and what the operator left it as.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItemView {
    pub text: String,
    pub checked: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCaptureView {
    pub key: String,
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub capture_type: Option<String>,
    pub unit: Option<String>,
    pub required: bool,
    pub options: Vec<String>,
    pub expected: Option<serde_json::Value>,
    pub value: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAttachmentView {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

/// A whole step, as the "add step" form sends it.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepInput {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub severity: String,
    pub deprecated: bool,
    pub prose: String,
    pub captures: Vec<CaptureInput>,
}

/// The fields of an existing step to change. Absent means "leave it".
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepPatch {
    pub title: Option<String>,
    pub kind: Option<String>,
    pub severity: Option<String>,
    pub deprecated: Option<bool>,
    pub prose: Option<String>,
}

impl From<CaptureInput> for CaptureDraft {
    fn from(input: CaptureInput) -> Self {
        CaptureDraft {
            key: input.key,
            label: input.label,
            capture_type: input.capture_type,
            unit: input.unit.filter(|unit| !unit.trim().is_empty()),
            required: input.required,
            options: input.options,
            expected: input.expected.map(ExpectedDraft::from),
        }
    }
}

impl From<ExpectedInput> for ExpectedDraft {
    fn from(input: ExpectedInput) -> Self {
        match input {
            ExpectedInput::Range { min, max } => ExpectedDraft::Range { min, max },
            ExpectedInput::Bool { value } => ExpectedDraft::Bool(value),
            ExpectedInput::Select { value } => ExpectedDraft::Select(value),
        }
    }
}

impl From<StepInput> for StepDraft {
    fn from(input: StepInput) -> Self {
        StepDraft {
            id: input.id,
            title: input.title,
            kind: input.kind,
            severity: input.severity,
            deprecated: input.deprecated,
            prose: input.prose,
            captures: input.captures.into_iter().map(CaptureDraft::from).collect(),
        }
    }
}

impl From<StepPatch> for StepChanges {
    fn from(patch: StepPatch) -> Self {
        StepChanges {
            title: patch.title,
            kind: patch.kind,
            severity: patch.severity,
            deprecated: patch.deprecated,
            prose: patch.prose,
        }
    }
}
