//! Steps and their captures.
//!
//! A step is a `##` heading plus an optional `yaml step` block plus prose. See
//! `SPEC.md` section 6.

use serde_norway::{Mapping, Value};

use crate::error::ParseError;
use crate::vocab::{CAPTURE_KEYS, CAPTURE_TYPES, STEP_KEYS, is_valid_capture_key};

/// The `kind` of a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepKey {
    Check,
    Measure,
    Select,
    Note,
    Gate,
    Unknown(String),
}

impl StepKey {
    pub fn parse(value: &str) -> Self {
        match value {
            "check" => StepKey::Check,
            "measure" => StepKey::Measure,
            "select" => StepKey::Select,
            "note" => StepKey::Note,
            "gate" => StepKey::Gate,
            other => StepKey::Unknown(other.to_owned()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            StepKey::Check => "check",
            StepKey::Measure => "measure",
            StepKey::Select => "select",
            StepKey::Note => "note",
            StepKey::Gate => "gate",
            StepKey::Unknown(other) => other,
        }
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, StepKey::Unknown(_))
    }
}

/// The `type` of a capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureType {
    Text,
    Number,
    Integer,
    Bool,
    Select,
    Datetime,
    Duration,
    Attach,
    Unknown(String),
}

impl CaptureType {
    pub fn parse(value: &str) -> Self {
        match value {
            "text" => CaptureType::Text,
            "number" => CaptureType::Number,
            "integer" => CaptureType::Integer,
            "bool" => CaptureType::Bool,
            "select" => CaptureType::Select,
            "datetime" => CaptureType::Datetime,
            "duration" => CaptureType::Duration,
            "attach" => CaptureType::Attach,
            other => CaptureType::Unknown(other.to_owned()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            CaptureType::Text => "text",
            CaptureType::Number => "number",
            CaptureType::Integer => "integer",
            CaptureType::Bool => "bool",
            CaptureType::Select => "select",
            CaptureType::Datetime => "datetime",
            CaptureType::Duration => "duration",
            CaptureType::Attach => "attach",
            CaptureType::Unknown(other) => other,
        }
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, CaptureType::Unknown(_))
    }

    /// Whether `expected` is meaningful for this type.
    pub fn accepts_expected(&self) -> bool {
        matches!(
            self,
            CaptureType::Number | CaptureType::Integer | CaptureType::Select | CaptureType::Bool
        )
    }
}

/// A declared expectation, used for highlighting only. See `SPEC.md` and D4.
#[derive(Debug, Clone, PartialEq)]
pub enum Expected {
    Range {
        min: Option<f64>,
        max: Option<f64>,
    },
    Bool(bool),
    Select(String),
    /// Present but unusable; carries the reason for the diagnostic.
    Malformed(String),
}

#[derive(Debug, Clone)]
pub struct Capture {
    pub key: Option<String>,
    pub label: Option<String>,
    pub capture_type: Option<CaptureType>,
    pub unit: Option<String>,
    pub required: Option<bool>,
    pub options: Vec<String>,
    pub expected: Option<Expected>,
    pub accept: Vec<String>,
    pub unknown_keys: Vec<String>,
    /// Every key written in the mapping, including ones whose value is null.
    ///
    /// Presence is tracked separately from the typed fields because the format treats
    /// `unit:` (null) and no `unit` key at all differently in some checks, and because
    /// a warning that fires on `accept: []` would be noise.
    pub present: Vec<String>,
    /// The mapping exactly as written.
    ///
    /// The typed fields are a view of this; the raw mapping is the source of truth for
    /// unknown keys, which must survive a read-modify-write cycle (`SPEC-COMPAT.md`).
    pub raw: Mapping,
    /// 1-based line of the `key:` entry, best effort.
    pub line: Option<usize>,
}

impl Capture {
    /// Whether the capture is structurally usable, independent of the checks that need
    /// a resolved checklist.
    pub fn key_is_well_formed(&self) -> bool {
        self.key.as_deref().is_some_and(is_valid_capture_key)
    }

    pub fn declares(&self, key: &str) -> bool {
        self.present.iter().any(|present| present == key)
    }
}

#[derive(Debug, Clone)]
pub struct Step {
    /// 1-based line of the opening fence of the `yaml step` block.
    pub line: usize,
    pub id: Option<String>,
    pub key: Option<StepKey>,
    pub severity: Option<String>,
    pub deprecated: bool,
    pub captures: Vec<Capture>,
    pub unknown_keys: Vec<String>,
    /// Every key written in the mapping, including ones whose value is null.
    pub present: Vec<String>,
    /// The mapping exactly as written. See [`Capture::raw`].
    pub raw: Mapping,
    /// The prose of the step: everything between its `##` heading and the next `##`
    /// heading, with the `yaml step` block itself removed.
    ///
    /// This is what the operator reads, so it travels with the step rather than being
    /// recovered from the file later.
    pub prose: String,
    /// The nearest preceding `##` heading, when there is one.
    pub title: Option<String>,
    pub title_line: Option<usize>,
    /// 1-based, half-open: from the `##` heading to the end of the step's prose.
    pub span: (usize, usize),
    /// 1-based, half-open: the `yaml step` fence itself.
    pub block_span: (usize, usize),
    /// Repository-relative path, filled in by `sop-repo`.
    pub source: Option<String>,
}

impl Step {
    pub fn id_or_placeholder(&self) -> &str {
        self.id.as_deref().unwrap_or("<missing id>")
    }

    pub fn is_known_key(&self) -> bool {
        self.key.as_ref().is_some_and(StepKey::is_known)
    }

    pub fn declares(&self, key: &str) -> bool {
        self.present.iter().any(|present| present == key)
    }
}

fn empty_mapping_as_ok(content: &str, info: &str, line: usize) -> Result<Mapping, ParseError> {
    if content.trim().is_empty() {
        return Ok(Mapping::new());
    }
    match serde_norway::from_str::<Value>(content) {
        Ok(Value::Mapping(map)) => Ok(map),
        Ok(_) => Err(ParseError::BlockNotMapping {
            info: info.to_owned(),
            line,
        }),
        Err(source) => Err(ParseError::BlockYaml {
            info: info.to_owned(),
            line,
            source,
        }),
    }
}

/// Keys present in `map` that are not in `allowed`.
pub fn unknown_keys(map: &Mapping, allowed: &[&str]) -> Vec<String> {
    let mut found: Vec<String> = map
        .keys()
        .filter_map(Value::as_str)
        .filter(|key| !allowed.contains(key))
        .map(str::to_owned)
        .collect();
    found.sort();
    found
}

/// Every string key written in `map`, in document order.
pub fn present_keys(map: &Mapping) -> Vec<String> {
    map.keys()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn as_string(map: &Mapping, key: &str) -> Option<String> {
    map.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn as_bool(map: &Mapping, key: &str) -> Option<bool> {
    map.get(key).and_then(Value::as_bool)
}

fn string_sequence(map: &Mapping, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(Value::Sequence(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        Some(Value::String(single)) => vec![single.clone()],
        _ => Vec::new(),
    }
}

/// Find the 1-based line of the capture whose `key` is `key`, for diagnostics.
///
/// Captures are written as sequence items, so the entry is spelled `- key: name`, not
/// `name:`. Anything that does not resolve simply falls back to the enclosing step's
/// line, which is why this is allowed to be approximate.
fn locate_key_line(content: &str, content_start_line: usize, key: &str) -> Option<usize> {
    let spaced = format!("{key} ");
    let commented = format!("{key}#");
    content
        .split('\n')
        .position(|line| {
            if line.trim_start().starts_with('#') {
                return false;
            }
            let line = line.trim_start();
            let line = line.strip_prefix('-').unwrap_or(line).trim_start();
            let Some(value) = line.strip_prefix("key:") else {
                return false;
            };
            let value = value.trim_start();
            value == key || value.starts_with(&spaced) || value.starts_with(&commented)
        })
        .map(|offset| content_start_line + offset)
}

fn parse_expected(value: &Value) -> Expected {
    match value {
        Value::Bool(flag) => Expected::Bool(*flag),
        Value::String(text) => Expected::Select(text.clone()),
        Value::Number(number) => Expected::Malformed(format!(
            "a bare number ({number}); use a mapping with min and/or max"
        )),
        Value::Mapping(map) => {
            let extras: Vec<&str> = map
                .keys()
                .filter_map(Value::as_str)
                .filter(|key| *key != "min" && *key != "max")
                .collect();
            if !extras.is_empty() {
                return Expected::Malformed(format!("unexpected key(s): {}", extras.join(", ")));
            }
            for bound in ["min", "max"] {
                if let Some(item) = map.get(bound)
                    && item.as_f64().is_none()
                {
                    return Expected::Malformed(format!("'{bound}' must be a number"));
                }
            }
            Expected::Range {
                min: map.get("min").and_then(Value::as_f64),
                max: map.get("max").and_then(Value::as_f64),
            }
        }
        Value::Null => Expected::Malformed("empty".to_owned()),
        other => Expected::Malformed(format!("unsupported shape ({other:?})")),
    }
}

impl Capture {
    fn from_value(value: &Value, content: &str, content_start_line: usize) -> Self {
        let Some(map) = value.as_mapping() else {
            return Capture {
                key: None,
                label: None,
                capture_type: None,
                unit: None,
                required: None,
                options: Vec::new(),
                expected: None,
                accept: Vec::new(),
                unknown_keys: Vec::new(),
                present: Vec::new(),
                raw: Mapping::new(),
                line: None,
            };
        };
        let key = as_string(map, "key");
        let line = key
            .as_deref()
            .and_then(|name| locate_key_line(content, content_start_line, name));
        Capture {
            label: as_string(map, "label"),
            capture_type: as_string(map, "type").map(|text| CaptureType::parse(&text)),
            unit: as_string(map, "unit"),
            required: as_bool(map, "required"),
            options: string_sequence(map, "options"),
            expected: map.get("expected").map(parse_expected),
            accept: string_sequence(map, "accept"),
            unknown_keys: unknown_keys(map, CAPTURE_KEYS),
            present: present_keys(map),
            raw: map.clone(),
            key,
            line,
        }
    }
}

/// Parse a `yaml step` block.
pub fn parse_step(
    info: &str,
    content: &str,
    line: usize,
    content_start_line: usize,
) -> Result<Step, ParseError> {
    let map = empty_mapping_as_ok(content, info, line)?;

    let captures = match map.get("captures") {
        None => Vec::new(),
        Some(Value::Sequence(items)) => items
            .iter()
            .map(|item| Capture::from_value(item, content, content_start_line))
            .collect(),
        Some(_) => {
            return Err(ParseError::Invalid {
                message: "step 'captures' must be a sequence of mappings".to_owned(),
                line,
            });
        }
    };

    Ok(Step {
        line,
        id: as_string(&map, "id"),
        key: as_string(&map, "kind").map(|text| StepKey::parse(&text)),
        severity: as_string(&map, "severity"),
        deprecated: as_bool(&map, "deprecated").unwrap_or(false),
        captures,
        unknown_keys: unknown_keys(&map, STEP_KEYS),
        present: present_keys(&map),
        raw: map,
        prose: String::new(),
        title: None,
        title_line: None,
        span: (line, line),
        block_span: (line, line),
        source: None,
    })
}

/// Parse a `yaml result` block from a run record.
pub fn parse_result(info: &str, content: &str, line: usize) -> Result<ResultBlock, ParseError> {
    let map = empty_mapping_as_ok(content, info, line)?;
    let captures = match map.get("captures") {
        Some(Value::Mapping(map)) => map.clone(),
        _ => Mapping::new(),
    };
    Ok(ResultBlock {
        line,
        step: as_string(&map, "step"),
        status: as_string(&map, "status"),
        reason: as_string(&map, "reason"),
        captures,
        unknown_keys: unknown_keys(&map, crate::vocab::RESULT_KEYS),
    })
}

#[derive(Debug, Clone)]
pub struct ResultBlock {
    pub line: usize,
    pub step: Option<String>,
    pub status: Option<String>,
    pub reason: Option<String>,
    /// The `captures` mapping of a result block, kept for roll-up and export.
    pub captures: Mapping,
    pub unknown_keys: Vec<String>,
}

/// `CAPTURE_TYPES` as a list for error messages.
pub fn capture_type_list() -> String {
    CAPTURE_TYPES.join(", ")
}
