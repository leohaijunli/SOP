//! Controlled vocabularies from `SPEC.md` section 2.
//!
//! Adding a value here is an additive change and does not bump the schema, unless the
//! value is a capture type. See `SPEC-COMPAT.md`.

/// The only schema this build understands.
pub const SUPPORTED_SCHEMA: i64 = 1;

/// Experiment types permitted in `applies_to`.
pub const APPLIES_TO: &[&str] = &[
    "calibration",
    "ground-survey",
    "interference",
    "navigation",
    "uav-survey",
];

pub const SEVERITIES: &[&str] = &["info", "normal", "critical"];

pub const STEP_KINDS: &[&str] = &["check", "measure", "select", "note", "gate"];

/// Capture types. An unknown type is a hard error, not a silent fallback to `text`,
/// which is why adding one is a breaking change.
pub const CAPTURE_TYPES: &[&str] = &[
    "text", "number", "integer", "bool", "select", "datetime", "duration", "attach",
];

/// Capture types on which `expected` is meaningful.
pub const EXPECTED_TYPES: &[&str] = &["number", "integer", "select", "bool"];

pub const CHECKLIST_STATUS: &[&str] = &["draft", "active", "retired"];

pub const RUN_STATUS: &[&str] = &["complete", "partial", "aborted"];

/// The operator's own verdict on the run. Distinct from `RUN_STATUS`: `complete` means
/// the run was executed, `conclusion` is the human judgement (pass / fail / inconclusive),
/// and the tool never derives one from the other (`DECISIONS.md` D4).
pub const RUN_CONCLUSION: &[&str] = &["pass", "fail", "inconclusive"];

pub const RESULT_STATUS: &[&str] = &["done", "skipped", "deviated"];

/// Front matter keys understood for every file kind.
pub const COMMON_FRONT_KEYS: &[&str] = &[
    "kind",
    "title",
    "version",
    "updated",
    "applies_to",
    "schema",
    "tags",
    "notes",
];

pub const PROCEDURE_FRONT_KEYS: &[&str] = &["procedure_id"];

pub const CHECKLIST_FRONT_KEYS: &[&str] = &["sop_id", "status", "equipment", "conditions"];

pub const RUN_FRONT_KEYS: &[&str] = &[
    "run_id",
    "sop",
    "sop_version",
    "sop_commit",
    "operator",
    "site",
    "plan",
    "case",
    "started",
    "ended",
    "status",
    "conclusion",
    "conclusion_note",
    "sensor",
    "hardware",
    "conditions",
    "clock",
    "markers",
    "logs",
    "events_sha256",
    "added_steps",
    "deviations_count",
];

/// Front matter keys specific to a help page.
pub const HELP_FRONT_KEYS: &[&str] = &["help_id", "section", "order", "audience", "summary"];

/// Front matter keys specific to the project file.
pub const PROJECT_FRONT_KEYS: &[&str] = &[
    "project_id",
    "institution",
    "lead",
    "started",
    "summary",
    "contact",
    "sites",
];

/// The one file that names the project this repository is for.
pub const PROJECT_FILE: &str = "project.md";

/// Who a help page is written for. The panel can filter on this.
pub const HELP_AUDIENCE: &[&str] = &["operator", "author", "maintainer"];

/// Pages without an explicit `order` sort after the ones that have one.
pub const DEFAULT_HELP_ORDER: i64 = 100;

pub const STEP_KEYS: &[&str] = &["id", "kind", "severity", "deprecated", "captures", "outputs"];

pub const CAPTURE_KEYS: &[&str] = &[
    "key", "label", "type", "unit", "required", "options", "expected", "accept",
];

pub const RESULT_KEYS: &[&str] = &[
    "step",
    "status",
    "reason",
    "opened_at",
    "ended_at",
    "captures",
    "acknowledged",
];

/// Whether `key` is understood in the front matter of a file of `kind`.
pub fn known_front_keys(kind: &str, key: &str) -> bool {
    if COMMON_FRONT_KEYS.contains(&key) {
        return true;
    }
    match kind {
        "procedure" => PROCEDURE_FRONT_KEYS.contains(&key),
        "checklist" => CHECKLIST_FRONT_KEYS.contains(&key),
        "run" => RUN_FRONT_KEYS.contains(&key),
        "help" => HELP_FRONT_KEYS.contains(&key),
        "project" => PROJECT_FRONT_KEYS.contains(&key),
        _ => false,
    }
}

/// An identifier: lowercase alphanumerics and hyphens, starting with an alphanumeric.
pub fn is_valid_id(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A capture key: like an identifier but with underscores instead of hyphens.
pub fn is_valid_capture_key(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Whether a step id names a step added mid-run (`adhoc-NNN`).
///
/// The tool generates these; the operator never types one. A step added to a run is not
/// part of the checklist it was started from, so a record may cite one that its snapshot
/// does not contain (`DESIGN.md` 6.6).
pub fn is_added_step_id(id: &str) -> bool {
    id.starts_with("adhoc-")
}

/// `YYYY-MM-DD`, loosely: shape and ranges checked, not the calendar.
pub fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = |range: core::ops::Range<usize>| bytes[range].iter().all(|b| b.is_ascii_digit());
    if !digits(0..4) || !digits(5..7) || !digits(8..10) {
        return false;
    }
    let month = value[5..7].parse::<u32>().unwrap_or(0);
    let day = value[8..10].parse::<u32>().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}
