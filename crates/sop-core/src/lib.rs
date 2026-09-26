//! Pure domain logic for the field-sop content format.
//!
//! This crate does not read files, does not know about paths, and does not depend on
//! any UI. See `docs/DESIGN.md` section 4 for why that rule exists.
//!
//! The format itself is specified in `SPEC.md`.

pub mod authoring;
pub mod check;
pub mod diagnostic;
pub mod document;
pub mod error;
pub mod fence;
pub mod front;
pub mod link;
pub mod run;
pub mod settings;
pub mod step;
pub mod timestamp;
pub mod vocab;

pub use authoring::{
    AuthoringError, CaptureDraft, ExpectedDraft, ItemRef, Move, StepChanges, StepDraft,
};
pub use diagnostic::{Diagnostic, Diagnostics, Severity};
pub use document::{Document, Heading, Include};
pub use error::ParseError;
pub use fence::Fence;
pub use front::FrontMatter;
pub use link::Link;
pub use run::{replay, render_record, RunError, RunEvent, RunState, RecordStep, StepState, StepStatus};
pub use settings::{DEFAULT_REMOTE, Settings, SettingsError};
pub use timestamp::now_utc_rfc3339;
pub use step::{Capture, CaptureType, Expected, ResultBlock, Step, StepKey};

/// The YAML value type used by front matter and step blocks.
///
/// Re-exported so callers that inspect front matter do not have to depend on the YAML
/// crate directly, and cannot end up with a second, incompatible copy of it.
pub use serde_norway::{Mapping, Number, Value};
pub use vocab::{
    APPLIES_TO, CAPTURE_TYPES, CHECKLIST_STATUS, DEFAULT_HELP_ORDER, EXPECTED_TYPES, HELP_AUDIENCE,
    RESULT_STATUS, RUN_STATUS, SEVERITIES, STEP_KINDS, SUPPORTED_SCHEMA,
};
