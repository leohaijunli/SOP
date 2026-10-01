//! YAML front matter: splitting it off the body, and typed access to it.
//!
//! The mapping is kept raw rather than deserialised into a per-kind struct, because
//! unknown keys must be reported and preserved. See `SPEC-COMPAT.md`.

use serde::Serialize;
use serde_norway::{Mapping, Value};

use crate::error::ParseError;

const DELIMITER: &str = "---";

/// One value a checklist expects the operator to record at run start.
///
/// A checklist declares these under `conditions:` so the start form can show a field for
/// each one instead of a free-text box. `hint` is display-only: an example or a unit, as
/// an author would write it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConditionDecl {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

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

    /// The `conditions:` a checklist declares, in file order.
    ///
    /// Three spellings are accepted so an author picks the shape that reads best: a
    /// sequence of keys (`conditions: [weather, temp_c]`), a mapping of key to hint
    /// (`conditions: {temp_c: "degrees C"}`), or a single key as a bare string. Anything
    /// else is ignored here; the validator reports it.
    pub fn conditions(&self) -> Vec<ConditionDecl> {
        let mut out = Vec::new();
        let mut push = |key: &str, hint: Option<&str>| {
            let key = key.trim();
            if key.is_empty() {
                return;
            }
            let hint = hint
                .map(str::trim)
                .filter(|hint| !hint.is_empty())
                .map(str::to_owned);
            out.push(ConditionDecl {
                key: key.to_owned(),
                hint,
            });
        };
        match self.get("conditions") {
            None => {}
            Some(Value::String(single)) => push(single, None),
            Some(Value::Sequence(items)) => {
                for item in items {
                    if let Some(key) = item.as_str() {
                        push(key, None);
                    }
                }
            }
            Some(Value::Mapping(map)) => {
                for (key, value) in map {
                    if let Some(key) = key.as_str() {
                        push(key, value.as_str());
                    }
                }
            }
            Some(_) => {}
        }
        out
    }

    /// `true` when `conditions:` is present and well-formed: a key, a sequence of keys, or
    /// a mapping of keys to string hints.
    pub fn conditions_are_well_formed(&self) -> bool {
        match self.get("conditions") {
            None => true,
            Some(Value::String(_)) => true,
            Some(Value::Sequence(items)) => items.iter().all(Value::is_string),
            Some(Value::Mapping(map)) => map
                .iter()
                .all(|(key, value)| key.is_string() && value.is_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn front(body: &str) -> FrontMatter {
        let text = format!("---\n{body}\n---\n\nBody.");
        split(&text).unwrap().0
    }

    #[test]
    fn conditions_accept_a_list_a_mapping_and_a_bare_key() {
        let list = front("conditions: [weather, temp_c]");
        assert_eq!(
            list.conditions(),
            vec![
                ConditionDecl {
                    key: "weather".to_owned(),
                    hint: None
                },
                ConditionDecl {
                    key: "temp_c".to_owned(),
                    hint: None
                },
            ]
        );
        assert!(list.conditions_are_well_formed());

        let mapping = front("conditions:\n  temp_c: degrees C\n  weather: clear/sunny");
        let parsed = mapping.conditions();
        assert_eq!(parsed[0].key, "temp_c");
        assert_eq!(parsed[0].hint.as_deref(), Some("degrees C"));
        assert_eq!(parsed[1].key, "weather");
        assert!(mapping.conditions_are_well_formed());

        let single = front("conditions: weather");
        assert_eq!(single.conditions()[0].key, "weather");
        assert!(single.conditions_are_well_formed());
    }

    #[test]
    fn a_condition_with_no_key_is_dropped_and_a_bad_shape_is_reported() {
        let blanks = front("conditions: ['', '  ']");
        assert!(blanks.conditions().is_empty());
        assert!(blanks.conditions_are_well_formed());

        let bad = front("conditions:\n  - 3\n  - weather");
        assert!(!bad.conditions_are_well_formed());
    }
}
