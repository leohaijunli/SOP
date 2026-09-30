//! Validation rules from `SPEC.md` section 13 that need only one document.
//!
//! This is the implementation of those rules; there is no second one to port from
//! anymore. Cross-file rules (does this include resolve, does this run cite a real
//! checklist, do these hashes match) belong to `sop-repo`, which can read the filesystem.

use crate::diagnostic::Diagnostics;
use crate::document::Document;
use crate::link::{self, Link};
use crate::step::{Capture, CaptureType, Expected, Step};
use crate::vocab::{
    APPLIES_TO, CAPTURE_TYPES, CHECKLIST_STATUS, HELP_AUDIENCE, RESULT_STATUS, RUN_CONCLUSION,
    RUN_STATUS, SEVERITIES, STEP_KINDS, SUPPORTED_SCHEMA, is_added_step_id, is_iso_date,
    is_valid_capture_key, is_valid_id, known_front_keys,
};

/// Front matter keys this build does not understand.
///
/// These are warnings, never errors: `SPEC-COMPAT.md` requires an unknown key to be
/// ignored and preserved, so a file written by a newer build still validates.
pub fn unknown_front_keys(doc: &Document, kind: &str) -> Diagnostics {
    let mut out = Diagnostics::new();
    for key in doc.front.keys() {
        if !known_front_keys(kind, key) {
            out.warning(
                format!("unknown front matter key '{key}'; ignored and preserved (SPEC-COMPAT.md)"),
                Some(1),
            );
        }
    }
    out
}

/// Formatting rules from `SPEC.md` section 3.
pub fn formatting(text: &str) -> Diagnostics {
    let mut out = Diagnostics::new();

    if text.contains('\r') {
        out.error("file contains CR characters; use LF line endings", None);
    }
    if !text.ends_with('\n') {
        out.error("file does not end with a newline", None);
    } else if text.ends_with("\n\n") {
        out.error("file ends with more than one newline", None);
    }
    for (index, line) in text.split('\n').enumerate() {
        let trimmed = line.trim_start_matches(' ');
        if trimmed.starts_with('\t') {
            out.error("indented with a tab", Some(index + 1));
        }
    }

    out
}

fn kind_name(value: Option<&serde_norway::Value>) -> String {
    match value {
        None => "None".to_owned(),
        Some(serde_norway::Value::String(text)) => format!("'{text}'"),
        Some(serde_norway::Value::Null) => "None".to_owned(),
        Some(serde_norway::Value::Bool(flag)) => if *flag { "True" } else { "False" }.to_owned(),
        Some(serde_norway::Value::Number(number)) => number.to_string(),
        Some(other) => format!("{other:?}"),
    }
}

/// Front matter checks for a procedure or a checklist.
///
/// Run records deliberately skip these: they are generated, not authored, and their
/// required keys are listed separately by [`run_front_matter`].
pub fn front_matter(doc: &Document, kind: &str) -> Diagnostics {
    let mut out = Diagnostics::new();
    let front = &doc.front;

    match front.get("schema") {
        None => {}
        Some(schema) => match schema.as_i64() {
            Some(value) if value <= SUPPORTED_SCHEMA => {}
            Some(value) => out.error(
                format!(
                    "file declares schema {value}, but this tooling supports at most \
                     {SUPPORTED_SCHEMA}; upgrade the tooling (see SPEC-COMPAT.md)"
                ),
                Some(1),
            ),
            None => out.error("'schema' must be an integer", Some(1)),
        },
    }

    out.extend(unknown_front_keys(doc, kind));

    if front.get("kind").and_then(|value| value.as_str()) != Some(kind) {
        out.error(
            format!(
                "front matter 'kind' must be '{kind}', found {}",
                kind_name(front.get("kind"))
            ),
            Some(1),
        );
    }

    for key in ["title", "version", "updated", "applies_to"] {
        if !front.contains(key) {
            out.error(
                format!("front matter is missing required key '{key}'"),
                Some(1),
            );
        }
    }

    if let Some(version) = front.get("version")
        && version.as_i64().is_none()
    {
        out.error("'version' must be an integer", Some(1));
    }

    if let Some(updated) = front.get("updated")
        && !updated.as_str().is_some_and(is_iso_date)
    {
        out.error("'updated' must be a YYYY-MM-DD date", Some(1));
    }

    if !front.string_list_is_well_formed("applies_to") {
        out.error(
            "'applies_to' must be a string or a sequence of strings",
            Some(1),
        );
    }
    let applies_to = front.string_list("applies_to");
    if applies_to.is_empty() {
        out.error(
            "'applies_to' must list at least one experiment type",
            Some(1),
        );
    }
    for value in &applies_to {
        if !APPLIES_TO.contains(&value.as_str()) {
            out.error(
                format!(
                    "'applies_to' value '{value}' is not in the controlled vocabulary ({})",
                    APPLIES_TO.join(", ")
                ),
                Some(1),
            );
        }
    }

    out
}

/// Front matter checks specific to a run record.
pub fn run_front_matter(doc: &Document) -> Diagnostics {
    let mut out = Diagnostics::new();
    let front = &doc.front;

    if front.get("kind").and_then(|value| value.as_str()) != Some("run") {
        out.error("front matter 'kind' must be 'run'", Some(1));
    }

    for key in [
        "run_id",
        "sop",
        "sop_version",
        "operator",
        "site",
        "started",
        "deviations_count",
    ] {
        if !front.contains(key) {
            out.error(
                format!("front matter is missing required key '{key}'"),
                Some(1),
            );
        }
    }

    // `status` and `ended` are written together when the run is ended, and are both
    // absent while it is still in progress. A record on disk mid-run is normal - it is
    // what crash recovery replays - so an unfinished record is not an error. A record
    // with one key and not the other was edited by hand.
    if front.contains("status") != front.contains("ended") {
        out.error("'status' and 'ended' must be written together", Some(1));
    }

    if let Some(value) = front.get("sop_version")
        && value.as_i64().is_none()
    {
        out.error("'sop_version' must be an integer", Some(1));
    }
    if let Some(value) = front.get("deviations_count")
        && value.as_i64().is_none()
    {
        out.error("'deviations_count' must be an integer", Some(1));
    }
    match front.get("status").and_then(|value| value.as_str()) {
        Some(status) if RUN_STATUS.contains(&status) => {}
        Some(other) => out.error(
            format!(
                "'status' must be one of {}; found '{other}'",
                RUN_STATUS.join(", ")
            ),
            Some(1),
        ),
        None => {}
    }

    // A conclusion is the operator's own verdict, so it only makes sense on a finished
    // run, and it is written with the one-sentence note that says why. A record with a
    // verdict but no end, or a verdict without a note, was edited by hand.
    if front.contains("conclusion") || front.contains("conclusion_note") {
        if !front.contains("status") {
            out.error("'conclusion' requires the run to be ended", Some(1));
        }
        match front.get("conclusion").and_then(|value| value.as_str()) {
            Some(conclusion) if RUN_CONCLUSION.contains(&conclusion) => {}
            Some(other) => out.error(
                format!(
                    "'conclusion' must be one of {}; found '{other}'",
                    RUN_CONCLUSION.join(", ")
                ),
                Some(1),
            ),
            None => out.error("'conclusion' is required alongside 'conclusion_note'", Some(1)),
        }
        if front.get("conclusion_note").and_then(|value| value.as_str()).is_none_or(str::is_empty) {
            out.error("'conclusion_note' must be a one-sentence summary", Some(1));
        }
    }

    // `sensor`, `hardware`, and `conditions` are the run's instrument and environment.
    // They are optional - a run recorded without them is still a valid record - but when
    // they are present their shape is checked, so a mistyped key does not silently
    // disappear from the record that a later analysis reads.
    if let Some(value) = front.get("sensor") {
        match value.as_mapping() {
            Some(map) => {
                for (key, item) in map {
                    if key.as_str().is_none() || item.is_mapping() || item.is_sequence() {
                        out.error("every 'sensor' key and value must be text", Some(1));
                        break;
                    }
                }
            }
            None => out.error("'sensor' must be a mapping like 'model: ...'", Some(1)),
        }
    }
    if front.get("hardware").is_some() && !front.string_list_is_well_formed("hardware") {
        out.error("'hardware' must be a list of text items", Some(1));
    }
    if let Some(value) = front.get("conditions") {
        match value.as_mapping() {
            Some(map) => {
                for (key, item) in map {
                    // A scalar value is fine even when YAML reads it as a number, so
                    // `temp_c: 12` is accepted; only a nested block is wrong.
                    if key.as_str().is_none() || item.is_mapping() || item.is_sequence() {
                        out.error("every 'conditions' key and value must be text", Some(1));
                        break;
                    }
                }
            }
            None => out.error(
                "'conditions' must be a mapping like 'weather: clear'",
                Some(1),
            ),
        }
    }

    out
}

/// Front matter checks for a help page.
///
/// Help is not part of an experiment, so it carries no `version` and no `applies_to`:
/// a page is edited in place and the panel always shows the current text.
pub fn help_front_matter(doc: &Document) -> Diagnostics {
    let mut out = Diagnostics::new();
    let front = &doc.front;

    out.extend(unknown_front_keys(doc, "help"));

    if front.get("kind").and_then(|value| value.as_str()) != Some("help") {
        out.error(
            format!(
                "front matter 'kind' must be 'help', found {}",
                kind_name(front.get("kind"))
            ),
            Some(1),
        );
    }

    for key in ["title", "section", "updated"] {
        if !front.contains(key) {
            out.error(
                format!("front matter is missing required key '{key}'"),
                Some(1),
            );
        }
    }

    if let Some(updated) = front.get("updated")
        && !updated.as_str().is_some_and(is_iso_date)
    {
        out.error("'updated' must be a YYYY-MM-DD date", Some(1));
    }

    for key in ["title", "section"] {
        if let Some(value) = front.get(key)
            && value.as_str().is_none_or(|text| text.trim().is_empty())
        {
            out.error(format!("'{key}' must be a non-empty string"), Some(1));
        }
    }

    if let Some(order) = front.get("order")
        && order.as_i64().is_none()
    {
        out.error("'order' must be an integer", Some(1));
    }

    if let Some(audience) = front.get("audience") {
        match audience.as_str() {
            Some(value) if HELP_AUDIENCE.contains(&value) => {}
            _ => out.error(
                format!(
                    "'audience' {} must be one of {}",
                    kind_name(Some(audience)),
                    HELP_AUDIENCE.join(", ")
                ),
                Some(1),
            ),
        }
    }

    out
}

/// Front matter checks for `project.md`, the file that names the project.
///
/// The project is repository content rather than an application setting, so that every
/// operator and every record means the same thing by the name, and so that renaming a
/// campaign is a reviewed change like any other.
pub fn project_front_matter(doc: &Document) -> Diagnostics {
    let mut out = Diagnostics::new();
    let front = &doc.front;

    out.extend(unknown_front_keys(doc, "project"));

    if front.get("kind").and_then(|value| value.as_str()) != Some("project") {
        out.error(
            format!(
                "front matter 'kind' must be 'project', found {}",
                kind_name(front.get("kind"))
            ),
            Some(1),
        );
    }

    for key in ["project_id", "title", "updated"] {
        if !front.contains(key) {
            out.error(
                format!("front matter is missing required key '{key}'"),
                Some(1),
            );
        }
    }

    match front.get("project_id").and_then(|value| value.as_str()) {
        None if front.contains("project_id") => out.error(
            format!(
                "'project_id' {} is not a valid id",
                kind_name(front.get("project_id"))
            ),
            Some(1),
        ),
        None => {}
        Some(id) if !is_valid_id(id) => {
            out.error(format!("'project_id' '{id}' is not a valid id"), Some(1));
        }
        Some(_) => {}
    }

    if let Some(title) = front.get("title")
        && title.as_str().is_none_or(|text| text.trim().is_empty())
    {
        out.error("'title' must be a non-empty string", Some(1));
    }

    for key in ["updated", "started"] {
        if let Some(value) = front.get(key)
            && !value.as_str().is_some_and(is_iso_date)
        {
            out.error(format!("'{key}' must be a YYYY-MM-DD date"), Some(1));
        }
    }

    if !front.string_list_is_well_formed("applies_to") {
        out.error(
            "'applies_to' must be a string or a sequence of strings",
            Some(1),
        );
    }
    for value in front.string_list("applies_to") {
        if !APPLIES_TO.contains(&value.as_str()) {
            out.error(
                format!(
                    "'applies_to' value '{value}' is not in the controlled vocabulary ({})",
                    APPLIES_TO.join(", ")
                ),
                Some(1),
            );
        }
    }

    out
}

/// `##` chapters that are not steps, because no `yaml step` block follows them.
///
/// A step is a heading *and* a block: the block carries the `id` a run record cites,
/// which is why the id cannot be derived from the heading text. A heading without one
/// is a chapter of prose, and saying so here is the difference between "no steps
/// defined" and knowing which chapter the tool ignored.
pub fn headings_without_steps(doc: &Document) -> Diagnostics {
    let mut out = Diagnostics::new();
    for heading in doc.step_headings() {
        if doc
            .steps
            .iter()
            .any(|step| step.title_line == Some(heading.line))
        {
            continue;
        }
        out.error(
            format!(
                "chapter '{}' has no `yaml step` block, so it is not a step; add one with an `id`",
                heading.title
            ),
            Some(heading.line),
        );
    }
    out
}

/// Step checks for procedures and checklists.
pub fn steps(doc: &Document, require_steps: bool) -> Diagnostics {
    let mut out = Diagnostics::new();
    if doc.steps.is_empty() && require_steps {
        out.error(
            "no steps defined: a step is a `##` chapter with a `yaml step` block under it",
            None,
        );
    }

    let mut seen: Vec<&str> = Vec::new();
    for step in &doc.steps {
        let placeholder = step
            .id
            .as_deref()
            .map_or("<missing id>".to_owned(), |id| format!("'{id}'"));

        match step.id.as_deref() {
            // `id: 7` is what this mistake looks like: YAML reads a bare number as an
            // integer, so the id never arrives as a string. Saying "no id" would be
            // true and useless, so name the shape the fix has to take.
            None if step.declares("id") => out.error(
                "step 'id' must be a string; write an all-digit id in quotes (`id: \"7\"`)",
                Some(step.line),
            ),
            None => out.error("step has no 'id'", Some(step.line)),
            Some(id) if !is_valid_id(id) => {
                out.error(format!("step id '{id}' is not a valid id"), Some(step.line));
            }
            Some(id) if seen.contains(&id) => {
                out.error(format!("duplicate step id '{id}'"), Some(step.line));
            }
            Some(id) => seen.push(id),
        }

        match &step.key {
            None => out.error(format!("step {placeholder} has no 'kind'"), Some(step.line)),
            Some(key) if !key.is_known() => out.error(
                format!(
                    "step {placeholder}: kind '{}' is not one of {}",
                    key.as_str(),
                    STEP_KINDS.join(", ")
                ),
                Some(step.line),
            ),
            Some(_) => {}
        }

        // `severity` defaults to normal when the key is absent, but a present key with a
        // value we do not recognise is an error, including an explicit null.
        if step.declares("severity") {
            let severity = step.severity.as_deref().unwrap_or("");
            if !SEVERITIES.contains(&severity) {
                out.error(
                    format!(
                        "step {placeholder}: severity '{severity}' is not one of {}",
                        SEVERITIES.join(", ")
                    ),
                    Some(step.line),
                );
            }
        }

        for key in &step.unknown_keys {
            out.warning(
                format!("step {placeholder}: unknown key '{key}'; ignored and preserved (SPEC-COMPAT.md)"),
                Some(step.line),
            );
        }

        for capture in &step.captures {
            captures(&mut out, step, capture);
        }
    }

    out
}

fn captures(out: &mut Diagnostics, step: &Step, capture: &Capture) {
    let step_id = step.id_or_placeholder();
    let line = capture.line.or(Some(step.line));
    let name = capture
        .key
        .as_deref()
        .map_or("<missing key>".to_owned(), |key| format!("'{key}'"));
    let where_ = format!("step '{step_id}': capture {name}");

    match capture.key.as_deref() {
        None => out.error(format!("{where_} has no 'key'"), line),
        Some(key) if !is_valid_capture_key(key) => {
            out.error(format!("{where_}: bad capture key '{key}'"), line)
        }
        Some(_) => {}
    }

    if !capture.declares("label") {
        out.error(format!("{where_} has no 'label'"), line);
    }

    match &capture.capture_type {
        None if !capture.declares("type") => {
            out.error(format!("{where_} has no 'type'"), line);
        }
        None => out.error(
            format!(
                "{where_}: type '' is not one of {}",
                CAPTURE_TYPES.join(", ")
            ),
            line,
        ),
        Some(other) if !other.is_known() => out.error(
            format!(
                "{where_}: type '{}' is not one of {}",
                other.as_str(),
                CAPTURE_TYPES.join(", ")
            ),
            line,
        ),
        Some(CaptureType::Select) if capture.options.is_empty() => out.error(
            format!("{where_} is a select but declares no 'options'"),
            line,
        ),
        Some(_) => {}
    }

    if matches!(capture.capture_type, Some(CaptureType::Attach)) {
        for meaningless in ["unit", "options"] {
            if capture.declares(meaningless) {
                out.warning(
                    format!(
                        "{where_} is an attach but declares '{meaningless}', which has no meaning for a file"
                    ),
                    line,
                );
            }
        }
    } else if capture.declares("accept") {
        out.warning(
            format!("{where_} declares 'accept' but is not an attach"),
            line,
        );
    }

    for key in &capture.unknown_keys {
        out.warning(
            format!("{where_}: unknown key '{key}'; ignored and preserved (SPEC-COMPAT.md)"),
            line,
        );
    }

    expected(out, &where_, capture, line);
}

/// `expected` is presentation only (`SPEC.md`, D4): it never fails a run, but a shape
/// the app cannot render is a content bug and is reported as one.
///
/// The capture's type decides what `expected` has to look like, so the shape that was
/// parsed is checked against the type rather than the other way round.
fn expected(out: &mut Diagnostics, where_: &str, capture: &Capture, line: Option<usize>) {
    let Some(value) = &capture.expected else {
        return;
    };

    match capture.capture_type {
        Some(CaptureType::Number) | Some(CaptureType::Integer) => match value {
            Expected::Range { min, max } => {
                if min.is_none() && max.is_none() {
                    out.error(
                        format!("{where_}: 'expected' must set 'min', 'max', or both"),
                        line,
                    );
                }
                if let (Some(low), Some(high)) = (min, max)
                    && low > high
                {
                    out.error(
                        format!(
                            "{where_}: 'expected.min' ({low}) is greater than 'expected.max' ({high})"
                        ),
                        line,
                    );
                }
            }
            Expected::Malformed(reason) => out.error(format!("{where_}: {reason}"), line),
            _ => out.error(
                format!("{where_}: 'expected' must be a mapping with 'min' and/or 'max'"),
                line,
            ),
        },
        Some(CaptureType::Bool) => {
            if !matches!(value, Expected::Bool(_)) {
                out.error(format!("{where_}: 'expected' must be true or false"), line);
            }
        }
        Some(CaptureType::Select) => match value {
            Expected::Select(choice) if capture.options.contains(choice) => {}
            Expected::Select(choice) => out.error(
                format!("{where_}: 'expected' value '{choice}' is not one of its own options"),
                line,
            ),
            _ => out.error(
                format!("{where_}: 'expected' must be one of its own options"),
                line,
            ),
        },
        _ => out.error(
            format!(
                "{where_}: 'expected' is valid only on {}",
                crate::vocab::EXPECTED_TYPES.join(", ")
            ),
            line,
        ),
    }
}

/// Templates must not contain pre-checked items. See `SPEC.md`.
pub fn prechecked_items(doc: &Document) -> Diagnostics {
    let mut out = Diagnostics::new();
    for (line, text) in crate::fence::lines_outside_fences(&doc.body, doc.body_start_line) {
        if let Some(state) = checkbox_state(text)
            && state != ' '
        {
            out.error(
                "pre-checked item '- [x]' in a template; use '- [ ]' (see SPEC.md, checkbox and prose list semantics)",
                Some(line),
            );
        }
    }
    out
}

/// The state character of a task list item, when the line is one.
///
/// List items are interpreted one at a time by their own syntax; a mixed list does not
/// change the meaning of its other items. See `SPEC.md`.
pub fn checkbox_state(line: &str) -> Option<char> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix('-')
        .or_else(|| trimmed.strip_prefix('*'))
        .or_else(|| trimmed.strip_prefix('+'))?;
    if !rest.starts_with(' ') {
        return None;
    }
    let rest = rest.trim_start().strip_prefix('[')?;
    let mut chars = rest.chars();
    let state = chars.next()?;
    if state != ' ' && state != 'x' && state != 'X' {
        return None;
    }
    if chars.next()? != ']' {
        return None;
    }
    Some(state)
}

/// Relative link targets that a caller must resolve against the filesystem.
pub fn link_targets(doc: &Document) -> Vec<Link> {
    link::scan(&doc.body, doc.body_start_line)
}

/// Ids that appear more than once across a resolved step list.
///
/// A duplicate inside one file is already reported by [`steps`], which has a better
/// line number, so only cross-file collisions are reported here.
pub fn duplicate_ids(steps: &[Step]) -> Diagnostics {
    let mut out = Diagnostics::new();
    let mut seen: Vec<(&str, &Option<String>)> = Vec::new();
    for step in steps {
        let Some(id) = step.id.as_deref() else {
            continue;
        };
        match seen.iter().find(|(existing, _)| *existing == id) {
            None => seen.push((id, &step.source)),
            Some((_, first_source)) => {
                let first = first_source.as_deref().unwrap_or("this file");
                let second = step.source.as_deref().unwrap_or("this file");
                if first == second {
                    continue;
                }
                out.error(
                    format!(
                        "step id '{id}' from {second} collides with {first}; included procedures must not share step ids"
                    ),
                    Some(step.line),
                );
            }
        }
    }
    out
}

/// Whether the step ids a record is checked against are its own revision or not.
///
/// The distinction matters for exactly two findings: a citation the step set does not
/// contain, and a `complete` run whose step set it does not cover. Both compare the
/// record against a revision, so they are errors only when that revision is the one the
/// record was written against. A record with no snapshot can only be compared against
/// the current checklist, which is a hint and not a verdict: the record may be right for
/// its own revision, and an edit to the SOP must not be refused because of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Citation {
    /// The step set is the revision the record was written against.
    Strict,
    /// The step set is the current checklist, because the record has no snapshot.
    Advisory,
}

impl Citation {
    /// Report a finding that compares the record against this step set.
    fn report(self, out: &mut Diagnostics, message: String, line: Option<usize>) {
        match self {
            Citation::Strict => out.error(message, line),
            Citation::Advisory => out.warning(
                format!(
                    "{message} (the run has no snapshot, so this was checked against the current checklist)"
                ),
                line,
            ),
        }
    }
}

/// Run results, once the cited checklist's step ids are known.
pub fn run_results(
    doc: &Document,
    sop: &str,
    checklist_step_ids: &[String],
    citation: Citation,
) -> Diagnostics {
    let mut out = Diagnostics::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut deviations = 0usize;

    for result in &doc.results {
        let name = result
            .step
            .as_deref()
            .map_or("<missing step>".to_owned(), |step| format!("'{step}'"));

        match result.step.as_deref() {
            None => out.error("result has no 'step'", Some(result.line)),
            // A step added mid-run (`adhoc-NNN`) is legal even though it is not in the
            // checklist the run was started from; it lives only in the event log.
            Some(id) if is_added_step_id(id) => seen.push(id),
            Some(id) if !checklist_step_ids.iter().any(|known| known == id) => citation.report(
                &mut out,
                format!("result references step '{id}', which is not in checklist '{sop}'"),
                Some(result.line),
            ),
            Some(id) if seen.contains(&id) => out.error(
                format!("more than one result for step '{id}'"),
                Some(result.line),
            ),
            Some(id) => seen.push(id),
        }

        match result.status.as_deref() {
            // No status is a valid answer: the step happened and the operator recorded
            // data for it but gave no outcome. An unknown string is still an error,
            // because it is a typo rather than a deliberate silence.
            None => {}
            Some(status) if !RESULT_STATUS.contains(&status) => out.error(
                format!(
                    "result {name}: status '{status}' must be one of {}",
                    RESULT_STATUS.join(", ")
                ),
                Some(result.line),
            ),
            Some("deviated") => {
                deviations += 1;
                if result.reason.is_none() {
                    out.error(
                        format!("result {name} is deviated but has no 'reason'"),
                        Some(result.line),
                    );
                }
            }
            Some("skipped") => {
                if result.reason.is_none() {
                    out.error(
                        format!("result {name} is skipped but has no 'reason'"),
                        Some(result.line),
                    );
                }
            }
            Some(_) => {}
        }

        for key in &result.unknown_keys {
            out.warning(
                format!("result {name}: unknown key '{key}'; ignored and preserved"),
                Some(result.line),
            );
        }
    }

    if let Some(declared) = doc.front.i64("deviations_count")
        && declared != deviations as i64
    {
        out.error(
            format!("'deviations_count' is {declared} but {deviations} result(s) are deviated"),
            Some(1),
        );
    }

    out
}

/// A `complete` run must account for every step and skip none.
pub fn complete_run_coverage(
    doc: &Document,
    checklist_step_ids: &[String],
    citation: Citation,
) -> Diagnostics {
    let mut out = Diagnostics::new();
    if doc.front.str("status").flatten() != Some("complete") {
        return out;
    }
    let recorded: Vec<&str> = doc
        .results
        .iter()
        .filter_map(|r| r.step.as_deref())
        .collect();
    for id in checklist_step_ids {
        if !recorded.contains(&id.as_str()) {
            citation.report(
                &mut out,
                format!("run is 'complete' but step '{id}' has no result"),
                Some(1),
            );
        }
    }
    for result in &doc.results {
        let name = result
            .step
            .as_deref()
            .map_or("<missing>".to_owned(), |s| format!("'{s}'"));
        match result.status.as_deref() {
            // A `complete` run cannot contain a step with no outcome: if the data exists
            // but the operator never said whether it was good, the run is not complete.
            None => citation.report(
                &mut out,
                format!("run is 'complete' but step {name} records no outcome"),
                Some(result.line),
            ),
            Some("skipped") => out.error(
                format!("run is 'complete' but step {name} was skipped"),
                Some(result.line),
            ),
            _ => {}
        }
    }
    out
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// A `complete` run must have each file each step declares it produces, attached to that
/// step. "Done" and "data collected" are the same thing here (`item 4`).
pub fn declared_outputs_present(
    doc: &Document,
    checklist_outputs: &[(String, Vec<String>)],
) -> Diagnostics {
    let mut out = Diagnostics::new();
    if doc.front.str("status").flatten() != Some("complete") {
        return out;
    }
    let logs: Vec<&serde_norway::Value> = match doc.front.get("logs") {
        Some(serde_norway::Value::Sequence(items)) => items.iter().collect(),
        Some(other) => vec![other],
        None => Vec::new(),
    };
    let basename = basename;
    for (step_id, declared) in checklist_outputs {
        if declared.is_empty() {
            continue;
        }
        let attached: Vec<String> = logs
            .iter()
            .filter_map(|entry| {
                let map = entry.as_mapping()?;
                if map.get("step").and_then(|v| v.as_str()) != Some(step_id.as_str()) {
                    return None;
                }
                Some(map.get("path").and_then(|v| v.as_str())?.to_owned())
            })
            .collect();
        let missing: Vec<String> = declared
            .iter()
            .filter(|output| !attached.iter().any(|path| basename(path) == **output))
            .cloned()
            .collect();
        if !missing.is_empty() {
            // The run may have ended before its logs were uploaded, so a declared output
            // that is not there yet is a pending warning, not a failure. It clears once
            // the output is attached (`DECISIONS.md` D29, Q4).
            out.warning(
                format!(
                    "declared output pending: run is 'complete' but step '{step_id}' has no attached output yet for: {}",
                    missing.join(", ")
                ),
                Some(1),
            );
        }
    }
    out
}

/// A step that ended (done / skipped / deviated) must record when it was opened. Without a
/// `StepOpened` there is no start to its window, so the timeline cannot place it; the step
/// was likely recorded by an older build, which is a warning rather than an error.
pub fn missing_step_opened(doc: &Document) -> Diagnostics {
    let mut out = Diagnostics::new();
    for result in &doc.results {
        if result.opened_at.is_none()
            && matches!(result.status.as_deref(), Some("done" | "skipped" | "deviated"))
        {
            out.warning(
                format!(
                    "step '{}' ended as '{}' but has no 'opened_at'; it was never opened, so its time window is unknown",
                    result.step.as_deref().unwrap_or("<unknown>"),
                    result.status.as_deref().unwrap_or("")
                ),
                Some(result.line),
            );
        }
    }
    out
}

/// A window whose end precedes its start is a clock that went backwards. The times are not
/// to be trusted, so this is an error rather than a soft warning.
pub fn time_regression(doc: &Document) -> Diagnostics {
    let mut out = Diagnostics::new();
    for result in &doc.results {
        let (Some(opened), Some(ended)) = (result.opened_at.as_deref(), result.ended_at.as_deref()) else {
            continue;
        };
        match (
            crate::timestamp::epoch_seconds(opened),
            crate::timestamp::epoch_seconds(ended),
        ) {
            (Some(a), Some(b)) if b < a => out.error(
                format!(
                    "step '{}' ends at {ended}, before it opened at {opened}; the clock went backwards",
                    result.step.as_deref().unwrap_or("<unknown>")
                ),
                Some(result.line),
            ),
            _ => {}
        }
    }
    out
}

/// A log with a recorded time range must overlap the run it was attached to, or the step
/// it was attached to. A file from the wrong day is the usual cause.
pub fn log_overlap(doc: &Document) -> Diagnostics {
    let mut out = Diagnostics::new();
    let run_start = doc.front.str("started").flatten().and_then(crate::timestamp::epoch_seconds);
    let run_end = doc.front.str("ended").flatten().and_then(crate::timestamp::epoch_seconds);
    let logs: Vec<&serde_norway::Value> = match doc.front.get("logs") {
        Some(serde_norway::Value::Sequence(items)) => items.iter().collect(),
        Some(other) => vec![other],
        None => Vec::new(),
    };
    for entry in logs {
        let Some(map) = entry.as_mapping() else { continue };
        let (Some(t_min), Some(t_max)) = (
            map.get("t_min").and_then(|v| v.as_str()).and_then(crate::timestamp::epoch_seconds),
            map.get("t_max").and_then(|v| v.as_str()).and_then(crate::timestamp::epoch_seconds),
        ) else { continue };
        let name = map.get("path").and_then(|v| v.as_str()).unwrap_or("<log>");
        // The window to overlap: the step it was attached to, else the run itself.
        let window = map
            .get("step")
            .and_then(|v| v.as_str())
            .and_then(|step| step_window(doc, step))
            .or_else(|| run_start.zip(run_end));
        let Some((start, end)) = window else { continue };
        if t_max < start || t_min > end {
            out.warning(
                format!(
                    "log '{name}' covers {t_min}..{t_max}, which does not overlap the {start}..{end} window of what it was attached to; it may be the wrong file"
                ),
                None,
            );
        }
    }
    out
}

/// A step's window as the record states it, by step id.
fn step_window(doc: &Document, step: &str) -> Option<(i64, i64)> {
    let result = doc.results.iter().find(|r| r.step.as_deref() == Some(step))?;
    let opened = crate::timestamp::epoch_seconds(result.opened_at.as_deref()?)?;
    let ended = crate::timestamp::epoch_seconds(result.ended_at.as_deref()?)?;
    Some((opened, ended))
}

/// An id in front matter must match the filename, and must be a valid id.
pub fn id_matches_filename(doc: &Document, key: &str, stem: &str) -> Diagnostics {
    let mut out = Diagnostics::new();
    match doc.front.get(key).and_then(|value| value.as_str()) {
        Some(id) if id == stem => {}
        Some(other) => out.error(
            format!("'{key}' '{other}' must match filename '{stem}'"),
            Some(1),
        ),
        None => {}
    }
    if doc.front.get(key).is_some()
        && !doc
            .front
            .get(key)
            .and_then(|value| value.as_str())
            .is_some_and(is_valid_id)
    {
        out.error(
            format!(
                "'{key}' {} is not a valid id",
                kind_name(doc.front.get(key))
            ),
            Some(1),
        );
    }
    out
}

/// A checklist's `status` must come from the controlled vocabulary.
pub fn checklist_status(doc: &Document) -> Diagnostics {
    let mut out = Diagnostics::new();
    if let Some(status) = doc.front.get("status") {
        let name = kind_name(Some(status));
        match status.as_str() {
            Some(text) if CHECKLIST_STATUS.contains(&text) => {}
            _ => out.error(
                format!(
                    "'status' {} must be one of {}",
                    name,
                    CHECKLIST_STATUS.join(", ")
                ),
                Some(1),
            ),
        }
    }
    out
}

/// A checklist's declared `conditions:` must be well-formed and name valid keys.
///
/// The keys are what the start form shows as fields and what a run records, so a typo
/// here becomes a run that asks for the wrong thing. This is a warning, not an error:
/// the run still works and an older reader ignores the key (`SPEC-COMPAT.md`).
pub fn checklist_conditions(doc: &Document) -> Diagnostics {
    let mut out = Diagnostics::new();
    if doc.front.get("conditions").is_none() {
        return out;
    }
    if !doc.front.conditions_are_well_formed() {
        out.warning(
            "'conditions' must be a key, a list of keys, or a mapping of key to hint",
            Some(1),
        );
        return out;
    }
    for condition in doc.front.conditions() {
        // The key becomes a `key: value` line in the record, so a space or a colon in it
        // would make the line ambiguous. Otherwise the key is free-form, to match the
        // `conditions` a run already records.
        if condition.key.contains(char::is_whitespace) || condition.key.contains(':') {
            out.warning(
                format!(
                    "condition '{}' must not contain spaces or colons; it is written as a \
                     `key: value` line",
                    condition.key
                ),
                Some(1),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::parse(text).unwrap()
    }

    #[test]
    fn a_complete_run_without_a_declared_output_is_a_pending_warning() {
        let record = "---\nkind: run\nrun_id: r\nsop: s\nstatus: complete\nstarted: 2026-09-25T12:00:00Z\ndeviations_count: 0\nlogs:\n  - path: runs/s/2026-09-25-r/logs/other.csv\n    step: s1\n    sha256: \"a\"\n---\n";
        let document = doc(record);
        let outputs = vec![("s1".to_owned(), vec!["mag_raw.csv".to_owned()])];
        let diags = declared_outputs_present(&document, &outputs);
        assert_eq!(diags.error_count(), 0, "a missing declared output is pending, not an error");
        assert_eq!(diags.warning_count(), 1);
    }

    #[test]
    fn a_declared_output_that_is_attached_passes() {
        let record = "---\nkind: run\nrun_id: r\nsop: s\nstatus: complete\nstarted: 2026-09-25T12:00:00Z\ndeviations_count: 0\nlogs:\n  - path: runs/s/2026-09-25-r/logs/mag_raw.csv\n    step: s1\n    sha256: \"a\"\n---\n";
        let document = doc(record);
        let outputs = vec![("s1".to_owned(), vec!["mag_raw.csv".to_owned()])];
        let diags = declared_outputs_present(&document, &outputs);
        assert!(diags.error_count() == 0, "{diags:?}");
    }
}
