//! Editing content without rewriting it.
//!
//! Every function here takes a file's text and returns new text. Nothing touches the
//! disk, nothing reflows a paragraph, and nothing drops a key: the editor changes only
//! the lines that describe the thing being changed and copies everything else through
//! byte for byte. `SPEC-COMPAT.md` requires unknown keys to survive a read-modify-write
//! cycle, and the simplest way to be sure of that is never to read them.
//!
//! These are the operations behind the editor form, and behind anything else that grows
//! content on an operator's behalf. They are deliberately about *one* item at a time: a
//! generator that emitted a whole file would produce a diff nobody can review, which is
//! the opposite of what a versioned procedure is for.

use std::ops::Range;

use crate::document::Document;
use crate::error::ParseError;
use crate::front::format_scalar;
use crate::vocab::{CAPTURE_TYPES, SEVERITIES, STEP_KINDS, is_valid_capture_key, is_valid_id};

#[derive(Debug, thiserror::Error)]
pub enum AuthoringError {
    #[error("{0}")]
    Parse(#[from] ParseError),

    #[error("there is no step '{0}' in this file")]
    NoSuchStep(String),

    #[error("there is no include of '{0}' in this file")]
    NoSuchInclude(String),

    #[error("'{0}' is not a field this editor can change")]
    UnknownField(String),

    #[error("{0}")]
    Invalid(String),

    #[error("the shape of this step is not one the editor changes: {0}")]
    Unsupported(String),
}

/// One addressable thing in a checklist or procedure, in document order.
///
/// A checklist is a sequence of steps and includes, so moving one entry up means moving
/// it past whichever kind of entry is above it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemRef {
    Step(String),
    Include(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Up,
    Down,
}

/// A step as the editor writes it.
#[derive(Debug, Clone, Default)]
pub struct StepDraft {
    pub id: String,
    pub title: String,
    /// One of `STEP_KINDS`.
    pub kind: String,
    /// One of `SEVERITIES`.
    pub severity: String,
    pub deprecated: bool,
    /// The Markdown the operator reads, after the block. May be empty.
    pub prose: String,
    pub captures: Vec<CaptureDraft>,
}

/// A capture as the editor writes it.
#[derive(Debug, Clone, Default)]
pub struct CaptureDraft {
    pub key: String,
    pub label: String,
    /// One of `CAPTURE_TYPES`.
    pub capture_type: String,
    pub unit: Option<String>,
    pub required: bool,
    pub options: Vec<String>,
    pub expected: Option<ExpectedDraft>,
}

/// A declared expectation. Highlighted, never judged. See `SPEC.md` section 6.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedDraft {
    Range { min: Option<f64>, max: Option<f64> },
    Bool(bool),
    Select(String),
}

/// The fields of a step the editor may change. `None` means "leave it alone".
#[derive(Debug, Clone, Default)]
pub struct StepChanges {
    pub title: Option<String>,
    pub kind: Option<String>,
    pub severity: Option<String>,
    pub deprecated: Option<bool>,
    pub prose: Option<String>,
}

// ------------------------------------------------------------------ line surgery

/// Split into lines with no trailing empty element, so indices are the lines.
fn split_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// Join lines back, always ending with exactly one newline.
fn join_lines(lines: &[String]) -> String {
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Insert `block` at `index`, keeping one blank line between it and its neighbours.
fn insert_block(lines: &mut Vec<String>, mut index: usize, block: Vec<String>) {
    if index > 0 && !lines[index - 1].trim().is_empty() {
        lines.insert(index, String::new());
        index += 1;
    }
    let length = block.len();
    lines.splice(index..index, block);
    let after = index + length;
    if after < lines.len() && !lines[after].trim().is_empty() {
        lines.insert(after, String::new());
    }
}

/// Remove a range and at most one of the blank lines it leaves behind.
fn remove_range(lines: &mut Vec<String>, range: Range<usize>) {
    if range.is_empty() {
        return;
    }
    let start = range.start;
    lines.drain(range);
    if start > 0
        && start < lines.len()
        && lines[start - 1].trim().is_empty()
        && lines[start].trim().is_empty()
    {
        lines.remove(start);
    }
}

/// Overwrite one top-level key inside a range of lines, adding it at the end of the
/// range when it is not there yet.
fn set_key_in(lines: &mut Vec<String>, range: Range<usize>, key: &str, value: &str) {
    let line = format!("{key}: {value}");
    for index in range.clone() {
        if top_level_key_at(&lines[index]).as_deref() == Some(key) {
            lines[index] = line;
            return;
        }
    }
    lines.insert(range.end, line);
}

/// The key a line declares, when it declares one at the top level of a block body.
fn top_level_key_at(line: &str) -> Option<String> {
    if line.starts_with([' ', '\t', '#']) {
        return None;
    }
    let (key, _) = line.split_once(':')?;
    Some(key.trim_end().to_owned())
}

// ---------------------------------------------------------------- step locations

/// Where a step sits: 0-based, half-open, covering its heading through the blank lines
/// before the next heading.
fn step_range(doc: &Document, id: &str) -> Option<Range<usize>> {
    let step = doc
        .steps
        .iter()
        .find(|step| step.id.as_deref() == Some(id))?;
    Some(step.span.0 - 1..step.span.1 - 1)
}

/// The lines between a step's `yaml step` fences, 0-based and half-open.
fn block_content(doc: &Document, id: &str) -> Option<Range<usize>> {
    let step = doc
        .steps
        .iter()
        .find(|step| step.id.as_deref() == Some(id))?;
    (step.block_span.0 < step.block_span.1).then(|| step.block_span.0..step.block_span.1 - 2)
}

/// The lines between a step's heading and the end of its prose, 0-based, half-open.
///
/// This is where prose goes: after the block when there is one, after the heading when
/// there is not.
fn prose_range(doc: &Document, id: &str) -> Option<Range<usize>> {
    let step = doc
        .steps
        .iter()
        .find(|step| step.id.as_deref() == Some(id))?;
    let start = if step.block_span.0 < step.block_span.1 {
        step.block_span.1 - 1
    } else {
        step.title_line.unwrap_or(step.line)
    };
    Some(start..step.span.1 - 1)
}

/// Every step and include in the file, in document order.
fn items(doc: &Document) -> Vec<ItemRef> {
    let mut found: Vec<(usize, ItemRef)> = doc
        .steps
        .iter()
        .filter_map(|step| step.id.clone().map(|id| (step.span.0, ItemRef::Step(id))))
        .chain(
            doc.includes
                .iter()
                .map(|include| (include.line, ItemRef::Include(include.target.clone()))),
        )
        .collect();
    found.sort_by_key(|(line, _)| *line);
    found.into_iter().map(|(_, item)| item).collect()
}

fn item_range(doc: &Document, item: &ItemRef) -> Option<Range<usize>> {
    match item {
        ItemRef::Step(id) => step_range(doc, id),
        ItemRef::Include(target) => doc
            .includes
            .iter()
            .find(|include| include.target == *target)
            .map(|include| include.line - 1..include.line),
    }
}

// -------------------------------------------------------------------- validation

fn check_id(id: &str) -> Result<(), AuthoringError> {
    if is_valid_id(id) {
        Ok(())
    } else {
        Err(AuthoringError::Invalid(format!(
            "'{id}' is not a valid id: lowercase letters, digits, and hyphens, starting \
             with a letter or digit"
        )))
    }
}

fn one_line(value: &str) -> bool {
    !value.contains('\n')
}

fn check_step_draft(draft: &StepDraft) -> Result<(), AuthoringError> {
    check_id(&draft.id)?;
    if draft.title.trim().is_empty() {
        return Err(AuthoringError::Invalid(
            "a step needs a title, because it is the heading an operator reads".to_owned(),
        ));
    }
    if !one_line(&draft.title) {
        return Err(AuthoringError::Invalid(
            "a step title must fit on one line".to_owned(),
        ));
    }
    if !STEP_KINDS.contains(&draft.kind.as_str()) {
        return Err(AuthoringError::Invalid(format!(
            "'{}' is not a step kind; expected one of {}",
            draft.kind,
            STEP_KINDS.join(", ")
        )));
    }
    if !SEVERITIES.contains(&draft.severity.as_str()) {
        return Err(AuthoringError::Invalid(format!(
            "'{}' is not a severity; expected one of {}",
            draft.severity,
            SEVERITIES.join(", ")
        )));
    }
    for capture in &draft.captures {
        check_capture_draft(capture)?;
    }
    let mut keys: Vec<&str> = draft.captures.iter().map(|c| c.key.as_str()).collect();
    keys.sort_unstable();
    let before = keys.len();
    keys.dedup();
    if keys.len() != before {
        return Err(AuthoringError::Invalid(
            "two captures in this step have the same key".to_owned(),
        ));
    }
    Ok(())
}

fn check_capture_draft(draft: &CaptureDraft) -> Result<(), AuthoringError> {
    if !is_valid_capture_key(&draft.key) {
        return Err(AuthoringError::Invalid(format!(
            "'{}' is not a valid capture key",
            draft.key
        )));
    }
    if draft.label.trim().is_empty() {
        return Err(AuthoringError::Invalid(format!(
            "'{}' has no label; the label is what the operator reads",
            draft.key
        )));
    }
    if !one_line(&draft.label) {
        return Err(AuthoringError::Invalid(format!(
            "'{}' has a label spanning lines",
            draft.key
        )));
    }
    if !CAPTURE_TYPES.contains(&draft.capture_type.as_str()) {
        return Err(AuthoringError::Invalid(format!(
            "'{}' is not a capture type; expected one of {}",
            draft.capture_type,
            CAPTURE_TYPES.join(", ")
        )));
    }
    if draft.capture_type == "select" {
        if draft.options.is_empty() {
            return Err(AuthoringError::Invalid(format!(
                "'{}' is a select, so it needs at least one option",
                draft.key
            )));
        }
    } else if !draft.options.is_empty() {
        return Err(AuthoringError::Invalid(format!(
            "'{}' is a {} and cannot have options",
            draft.key, draft.capture_type
        )));
    }
    if let Some(expected) = &draft.expected {
        let allowed = matches!(
            draft.capture_type.as_str(),
            "number" | "integer" | "select" | "bool"
        );
        if !allowed {
            return Err(AuthoringError::Invalid(format!(
                "'{}' is a {}, and only number, integer, select, and bool captures have \
                 an expected value",
                draft.key, draft.capture_type
            )));
        }
        match (expected, draft.capture_type.as_str()) {
            (ExpectedDraft::Range { min, max }, "number" | "integer") => {
                if min.is_none() && max.is_none() {
                    return Err(AuthoringError::Invalid(format!(
                        "'{}' declares an empty expected range",
                        draft.key
                    )));
                }
                if let (Some(min), Some(max)) = (min, max)
                    && min > max
                {
                    return Err(AuthoringError::Invalid(format!(
                        "'{}' expects at least {min} and at most {max}, which cannot both hold",
                        draft.key
                    )));
                }
            }
            (ExpectedDraft::Bool(_), "bool") => {}
            (ExpectedDraft::Select(value), "select") => {
                if !draft.options.contains(value) {
                    return Err(AuthoringError::Invalid(format!(
                        "'{}' expects '{value}', which is not one of its options",
                        draft.key
                    )));
                }
            }
            _ => {
                return Err(AuthoringError::Invalid(format!(
                    "'{}' has an expected value that does not match its type",
                    draft.key
                )));
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------------------- rendering

fn scalar(value: &str) -> String {
    format_scalar(value)
}

fn step_lines(draft: &StepDraft) -> Vec<String> {
    let mut out = vec![format!("## {}", draft.title.trim()), String::new()];
    out.push("```yaml step".to_owned());
    out.push(format!("id: {}", scalar(draft.id.trim())));
    out.push(format!("kind: {}", draft.kind));
    out.push(format!("severity: {}", draft.severity));
    if draft.deprecated {
        out.push("deprecated: true".to_owned());
    }
    if !draft.captures.is_empty() {
        out.push("captures:".to_owned());
        for capture in &draft.captures {
            out.extend(capture_lines(capture));
        }
    }
    out.push("```".to_owned());

    let prose = draft.prose.trim_end();
    if !prose.is_empty() {
        out.push(String::new());
        out.extend(prose.split('\n').map(str::to_owned));
    }
    out
}

fn capture_lines(draft: &CaptureDraft) -> Vec<String> {
    let mut out = vec![
        format!("  - key: {}", scalar(draft.key.trim())),
        format!("    label: {}", scalar(draft.label.trim())),
        format!("    type: {}", draft.capture_type),
    ];
    if let Some(unit) = draft
        .unit
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())
    {
        out.push(format!("    unit: {}", scalar(unit)));
    }
    if !draft.options.is_empty() {
        let options: Vec<String> = draft.options.iter().map(|o| scalar(o.trim())).collect();
        out.push(format!("    options: [{}]", options.join(", ")));
    }
    match &draft.expected {
        Some(ExpectedDraft::Range { min, max }) => {
            out.push("    expected:".to_owned());
            if let Some(min) = min {
                out.push(format!("      min: {min}"));
            }
            if let Some(max) = max {
                out.push(format!("      max: {max}"));
            }
        }
        Some(ExpectedDraft::Bool(value)) => out.push(format!("    expected: {value}")),
        Some(ExpectedDraft::Select(value)) => {
            out.push(format!("    expected: {}", scalar(value)));
        }
        None => {}
    }
    out.push(format!("    required: {}", draft.required));
    out
}

// ------------------------------------------------------------------ operations

/// Add a step, after another entry or at the end of the file.
pub fn add_step(
    text: &str,
    draft: &StepDraft,
    after: Option<&ItemRef>,
) -> Result<String, AuthoringError> {
    check_step_draft(draft)?;
    let doc = Document::parse(text)?;
    if doc
        .steps
        .iter()
        .any(|step| step.id.as_deref() == Some(draft.id.as_str()))
    {
        return Err(AuthoringError::Invalid(format!(
            "step '{}' is already in this file",
            draft.id
        )));
    }

    let mut lines = split_lines(text);
    let index = match after {
        None => lines.len(),
        Some(item) => match item_range(&doc, item) {
            Some(range) => range.end,
            None => return Err(missing(item)),
        },
    };
    insert_block(&mut lines, index, step_lines(draft));
    Ok(join_lines(&lines))
}

/// Remove a step, and nothing else.
pub fn remove_step(text: &str, id: &str) -> Result<String, AuthoringError> {
    let doc = Document::parse(text)?;
    let range = step_range(&doc, id).ok_or_else(|| AuthoringError::NoSuchStep(id.to_owned()))?;
    let mut lines = split_lines(text);
    remove_range(&mut lines, range);
    Ok(join_lines(&lines))
}

/// Change the fields of a step, leaving its captures and any key this editor does not
/// know about exactly as they were.
pub fn set_step(text: &str, id: &str, changes: &StepChanges) -> Result<String, AuthoringError> {
    let doc = Document::parse(text)?;
    let step = doc
        .steps
        .iter()
        .find(|step| step.id.as_deref() == Some(id))
        .ok_or_else(|| AuthoringError::NoSuchStep(id.to_owned()))?;
    let span = step_range(&doc, id).ok_or_else(|| AuthoringError::NoSuchStep(id.to_owned()))?;
    let block = block_content(&doc, id);
    let prose = prose_range(&doc, id).unwrap_or(span.end..span.end);
    let title_line = step.title_line.map(|line| line - 1);
    let step_line = step.line - 1;

    let mut lines = split_lines(text);

    // Prose first: its range is below the block, and nothing above it moves.
    if let Some(new_prose) = &changes.prose {
        let trimmed = new_prose.trim_end();
        let mut replacement: Vec<String> = if trimmed.is_empty() {
            Vec::new()
        } else {
            let mut out = vec![String::new()];
            out.extend(trimmed.split('\n').map(str::to_owned));
            out
        };
        lines.splice(prose.clone(), replacement.drain(..));
    }

    if let Some(title) = &changes.title {
        if title.trim().is_empty() || !one_line(title) {
            return Err(AuthoringError::Invalid(
                "a step title must be one non-empty line".to_owned(),
            ));
        }
        match title_line {
            Some(index) => lines[index] = format!("## {}", title.trim()),
            None => {
                let mut heading = vec![format!("## {}", title.trim()), String::new()];
                heading.append(&mut lines.split_off(step_line));
                lines.append(&mut heading);
            }
        }
    }

    let Some(block) = block else {
        if changes.kind.is_some() || changes.severity.is_some() || changes.deprecated.is_some() {
            return Err(AuthoringError::Unsupported(format!(
                "step '{id}' has no yaml step block, so it has no kind, severity, or \
                 deprecated flag to change"
            )));
        }
        return Ok(join_lines(&lines));
    };

    if let Some(kind) = &changes.kind {
        if !STEP_KINDS.contains(&kind.as_str()) {
            return Err(AuthoringError::Invalid(format!(
                "'{kind}' is not a step kind; expected one of {}",
                STEP_KINDS.join(", ")
            )));
        }
        set_key_in(&mut lines, block.clone(), "kind", kind);
    }
    if let Some(severity) = &changes.severity {
        if !SEVERITIES.contains(&severity.as_str()) {
            return Err(AuthoringError::Invalid(format!(
                "'{severity}' is not a severity; expected one of {}",
                SEVERITIES.join(", ")
            )));
        }
        set_key_in(&mut lines, block.clone(), "severity", severity);
    }
    if let Some(deprecated) = changes.deprecated {
        set_key_in(&mut lines, block, "deprecated", &deprecated.to_string());
    }

    Ok(join_lines(&lines))
}

/// Move an entry past the one above or below it.
pub fn move_item(text: &str, item: &ItemRef, direction: Move) -> Result<String, AuthoringError> {
    let doc = Document::parse(text)?;
    let order = items(&doc);
    let index = order
        .iter()
        .position(|candidate| candidate == item)
        .ok_or_else(|| missing(item))?;

    let neighbour = match direction {
        Move::Up if index > 0 => index - 1,
        Move::Down if index + 1 < order.len() => index + 1,
        Move::Up | Move::Down => {
            return Err(AuthoringError::Invalid(
                "there is nothing on that side to move past".to_owned(),
            ));
        }
    };

    let range = item_range(&doc, item).expect("the item was found a moment ago");
    let neighbour_range =
        item_range(&doc, &order[neighbour]).expect("the neighbour is in the file");
    let mut lines = split_lines(text);
    let moved: Vec<String> = lines[range.clone()].to_vec();
    lines.drain(range.clone());
    let removed = range.len();

    let insert_at = match direction {
        Move::Up => neighbour_range.start,
        Move::Down => neighbour_range.end - removed,
    };
    lines.splice(insert_at..insert_at, moved);
    Ok(join_lines(&lines))
}

/// Add an include marker, which is how one checklist reuses a procedure's steps.
pub fn add_include(
    text: &str,
    target: &str,
    after: Option<&ItemRef>,
) -> Result<String, AuthoringError> {
    if target.trim().is_empty() || !one_line(target) {
        return Err(AuthoringError::Invalid(
            "an include needs a target path on one line".to_owned(),
        ));
    }
    let doc = Document::parse(text)?;
    if doc.includes.iter().any(|include| include.target == target) {
        return Err(AuthoringError::Invalid(format!(
            "'{target}' is already included in this file"
        )));
    }

    let mut lines = split_lines(text);
    let index = match after {
        None => lines.len(),
        Some(item) => match item_range(&doc, item) {
            Some(range) => range.end,
            None => return Err(missing(item)),
        },
    };
    insert_block(
        &mut lines,
        index,
        vec![format!("<!-- include: {target} -->")],
    );
    Ok(join_lines(&lines))
}

/// Remove one include marker.
pub fn remove_include(text: &str, target: &str) -> Result<String, AuthoringError> {
    let doc = Document::parse(text)?;
    let item = ItemRef::Include(target.to_owned());
    let range = item_range(&doc, &item).ok_or_else(|| missing(&item))?;
    let mut lines = split_lines(text);
    remove_range(&mut lines, range);
    Ok(join_lines(&lines))
}

/// Add a capture to a step, or replace the capture that has the same key.
pub fn set_capture(
    text: &str,
    step_id: &str,
    draft: &CaptureDraft,
) -> Result<String, AuthoringError> {
    check_capture_draft(draft)?;
    let doc = Document::parse(text)?;
    let step = doc
        .steps
        .iter()
        .find(|step| step.id.as_deref() == Some(step_id))
        .ok_or_else(|| AuthoringError::NoSuchStep(step_id.to_owned()))?;
    let block = block_content(&doc, step_id).ok_or_else(|| {
        AuthoringError::Unsupported(format!("step '{step_id}' has no yaml step block"))
    })?;

    let mut lines = split_lines(text);
    match step
        .captures
        .iter()
        .find(|capture| capture.key.as_deref() == Some(draft.key.as_str()))
    {
        Some(existing) => {
            let start = existing.line.ok_or_else(|| {
                AuthoringError::Unsupported(format!(
                    "the capture '{}' cannot be located in the file, so it was not changed; \
                     edit it by hand",
                    draft.key
                ))
            })? - 1;
            let end = capture_end(&lines, start, block.end);
            lines.splice(start..end, capture_lines(draft));
        }
        None => {
            let (insert_at, needs_header) = captures_end(&lines, &block)?;
            let mut block_lines = capture_lines(draft);
            if needs_header {
                let mut out = vec!["captures:".to_owned()];
                out.append(&mut block_lines);
                block_lines = out;
            }
            lines.splice(insert_at..insert_at, block_lines);
        }
    }
    Ok(join_lines(&lines))
}

/// Remove a capture from a step. An empty `captures:` key is removed with it.
pub fn remove_capture(text: &str, step_id: &str, key: &str) -> Result<String, AuthoringError> {
    let doc = Document::parse(text)?;
    let step = doc
        .steps
        .iter()
        .find(|step| step.id.as_deref() == Some(step_id))
        .ok_or_else(|| AuthoringError::NoSuchStep(step_id.to_owned()))?;
    let block = block_content(&doc, step_id).ok_or_else(|| {
        AuthoringError::Unsupported(format!("step '{step_id}' has no yaml step block"))
    })?;
    let existing = step
        .captures
        .iter()
        .find(|capture| capture.key.as_deref() == Some(key))
        .ok_or_else(|| {
            AuthoringError::Invalid(format!("step '{step_id}' has no capture '{key}'"))
        })?;
    let start = existing.line.ok_or_else(|| {
        AuthoringError::Unsupported(format!("the capture '{key}' cannot be located in the file"))
    })? - 1;

    let mut lines = split_lines(text);
    let end = capture_end(&lines, start, block.end);
    let removed = end - start;
    lines.drain(start..end);
    // The block shrank by the lines that were just removed, so the header scan has to
    // look at where the block is now rather than where it was.
    drop_empty_captures_header(&mut lines, &(block.start..block.end - removed));
    Ok(join_lines(&lines))
}

/// The end of a capture entry: the next line at or above its own indentation, or the end
/// of the block.
fn capture_end(lines: &[String], start: usize, block_end: usize) -> usize {
    let indent = indent_of(&lines[start]);
    let mut end = start + 1;
    while end < block_end {
        let line = &lines[end];
        if !line.trim().is_empty() && indent_of(line) <= indent {
            break;
        }
        end += 1;
    }
    end
}

/// Where a new capture goes, and whether a `captures:` key has to be created for it.
fn captures_end(lines: &[String], block: &Range<usize>) -> Result<(usize, bool), AuthoringError> {
    let header = lines[block.clone()]
        .iter()
        .position(|line| top_level_key_at(line).as_deref() == Some("captures"))
        .map(|offset| block.start + offset);
    let Some(header) = header else {
        return Ok((block.end, true));
    };
    let mut end = header + 1;
    while end < block.end {
        let line = &lines[end];
        if !line.trim().is_empty() && indent_of(line) == 0 {
            break;
        }
        end += 1;
    }
    Ok((end, false))
}

/// Remove a `captures:` key that no longer has any captures under it.
fn drop_empty_captures_header(lines: &mut Vec<String>, block: &Range<usize>) {
    let Some(header) = lines[block.clone()]
        .iter()
        .position(|line| top_level_key_at(line).as_deref() == Some("captures"))
        .map(|offset| block.start + offset)
    else {
        return;
    };
    let empty = lines[header + 1..block.end]
        .iter()
        .all(|line| line.trim().is_empty());
    if empty {
        lines.drain(header..block.end);
    }
}

fn missing(item: &ItemRef) -> AuthoringError {
    match item {
        ItemRef::Step(id) => AuthoringError::NoSuchStep(id.clone()),
        ItemRef::Include(target) => AuthoringError::NoSuchInclude(target.clone()),
    }
}
