//! YAML front matter: splitting it off the body, and typed access to it.
//!
//! The mapping is kept raw rather than deserialised into a per-kind struct, because
//! unknown keys must be reported and preserved. See `SPEC-COMPAT.md`.

use serde_norway::{Mapping, Value};

use crate::error::ParseError;

const DELIMITER: &str = "---";

#[derive(Debug, Clone)]
pub struct FrontMatter {
    pub map: Mapping,
    /// Lines consumed by the front matter, including both delimiters.
    pub lines: usize,
}

impl FrontMatter {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.map.get(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.map.keys().filter_map(Value::as_str)
    }

    /// `None` when absent; `Some(None)` when present but not a string.
    pub fn str(&self, key: &str) -> Option<Option<&str>> {
        self.get(key).map(Value::as_str)
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(Value::as_bool)
    }

    pub fn i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(Value::as_i64)
    }

    /// A sequence of strings, or a single string treated as a one-element sequence.
    /// A scalar is accepted because `applies_to: calibration` is a natural thing to
    /// write and would otherwise be a confusing error.
    pub fn string_list(&self, key: &str) -> Vec<String> {
        match self.get(key) {
            None => Vec::new(),
            Some(Value::String(single)) => vec![single.clone()],
            Some(Value::Sequence(items)) => items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            Some(_) => Vec::new(),
        }
    }

    /// `true` when the key is present but is not a sequence or scalar of strings,
    /// so a caller can report a type error rather than silently seeing an empty list.
    pub fn string_list_is_well_formed(&self, key: &str) -> bool {
        match self.get(key) {
            None => true,
            Some(Value::String(_)) => true,
            Some(Value::Sequence(items)) => items.iter().all(|item| item.is_string()),
            Some(_) => false,
        }
    }
}

/// Split a document into front matter and body.
///
/// Returns the front matter, the body, and the 1-based line number the body starts on.
pub fn split(text: &str) -> Result<(FrontMatter, String, usize), ParseError> {
    let lines: Vec<&str> = text.split('\n').collect();

    if lines.first().map(|line| line.trim()) != Some(DELIMITER) {
        return Err(ParseError::MissingFrontMatter { line: 1 });
    }

    let close = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim() == DELIMITER)
        .map(|(index, _)| index)
        .ok_or(ParseError::UnclosedFrontMatter)?;

    let raw = lines[1..close].join("\n");
    let map = if raw.trim().is_empty() {
        Mapping::new()
    } else {
        match serde_norway::from_str::<Value>(&raw)? {
            Value::Mapping(map) => map,
            _ => return Err(ParseError::FrontMatterNotMapping),
        }
    };

    let body = lines[close + 1..].join("\n");
    let body_start_line = close + 2;

    Ok((
        FrontMatter {
            map,
            lines: close + 1,
        },
        body,
        body_start_line,
    ))
}

/// Rewrite one front-matter field, leaving the rest of the document byte-for-byte alone.
///
/// This is what makes an edit command safe to run on a file a person wrote by hand: the
/// body, the comments, the key order, and every key this build does not know about all
/// survive, which is what `SPEC-COMPAT.md` promises a writer must do. A missing key is
/// added just before the closing delimiter rather than at the end of the file.
///
/// Returns `None` when the text has no front matter to edit.
pub fn set_field(text: &str, key: &str, value: &str) -> Option<String> {
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    if lines.first()?.trim() != DELIMITER {
        return None;
    }
    let close = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim() == DELIMITER)
        .map(|(index, _)| index)?;

    let mut replacement = format!("{key}: {}", format_scalar(value));
    match lines[1..close]
        .iter()
        .position(|line| top_level_key(line) == Some(key))
    {
        Some(offset) => lines[offset + 1] = std::mem::take(&mut replacement),
        None => lines.insert(close, replacement),
    }
    Some(lines.join("\n"))
}

/// The key a front-matter line declares, when the line declares one at the top level.
///
/// Indented lines belong to a nested mapping or a block scalar, and comments declare
/// nothing, so neither is ever treated as the key being replaced.
fn top_level_key(line: &str) -> Option<&str> {
    if line.starts_with([' ', '\t', '#']) {
        return None;
    }
    let (key, _) = line.split_once(':')?;
    Some(key.trim_end())
}

/// Render a value as a front-matter scalar, quoting it when a bare word would be read as
/// something other than text.
///
/// Front matter is YAML, and YAML readers disagree about bare words: to a 1.1 reader `yes`,
/// `no`, and `on` are booleans, while a 1.2 reader sees strings. A survey answer really can
/// be `yes`, and a record that means one thing to the reader that wrote it and another to
/// the reader that opens it later is exactly the failure this format exists to prevent, so
/// anything ambiguous is quoted.
pub fn format_scalar(value: &str) -> String {
    if needs_quotes(value) {
        let escaped = value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t");
        format!("\"{escaped}\"")
    } else {
        value.to_owned()
    }
}

fn needs_quotes(value: &str) -> bool {
    // Leading or trailing space is dropped by every YAML reader, and a writer that did
    // not notice would silently lose it.
    if value.is_empty() || value.trim() != value {
        return true;
    }
    // A canonical date stays bare: every reader agrees on it, and the format's date
    // checks expect it written the way the rest of the content writes it.
    if crate::vocab::is_iso_date(value) {
        return false;
    }
    // Words a YAML 1.1 reader and a YAML 1.2 reader disagree about. Quoting is the only
    // way to be sure a bare `no` reads as the text `no` everywhere.
    if matches!(
        value.to_ascii_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off" | "y" | "n" | "null" | "none" | "~"
    ) {
        return true;
    }
    if value.parse::<f64>().is_ok() {
        return true;
    }
    // A colon or a hash *inside* a plain scalar ends it early or starts a comment, and
    // both lose the rest of the line without saying so. That is what is worth quoting
    // against; a comma in the middle of `Renfrew, BC` is harmless.
    if value.contains(": ") || value.contains(" #") || value.ends_with(':') {
        return true;
    }
    if value.contains(['\n', '\t']) {
        return true;
    }
    // A plain scalar may not begin with an indicator character: it would change what the
    // line means rather than what it says.
    matches!(value.chars().next(), Some(first) if "-?:,[]{}#&*!|>'\"%@`".contains(first))
}
