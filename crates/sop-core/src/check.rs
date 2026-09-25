//! Validation rules from `SPEC.md` section 13 that need only one document.
//!
//! This is a port of the reference implementation in `tools/validate.py`, which is the
//! executable definition of the rules until the port is proven. Cross-file rules (does
//! this include resolve, does this run cite a real checklist, do these hashes match)
//! belong to `sop-repo`, which can read the filesystem.

use crate::diagnostic::Diagnostics;
use crate::document::Document;
use crate::link::{self, Link};
use crate::step::{Capture, CaptureType, Expected, Step};
use crate::vocab::{
    APPLIES_TO, CAPTURE_TYPES, CHECKLIST_STATUS, HELP_AUDIENCE, RESULT_STATUS, RUN_STATUS,
    SEVERITIES, STEP_KINDS, SUPPORTED_SCHEMA, is_iso_date, is_valid_capture_key, is_valid_id,
    known_front_keys,
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
/// required keys are listed separately by [`run_front_matter`]. This matches
/// `tools/validate.py`.
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
        "status",
        "deviations_count",
    ] {
        if !front.contains(key) {
            out.error(
                format!("front matter is missing required key '{key}'"),
                Some(1),
            );
        }
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

/// Step checks for procedures and checklists.
pub fn steps(doc: &Document, require_steps: bool) -> Diagnostics {
    let mut out = Diagnostics::new();
    if doc.steps.is_empty() && require_steps {
        out.error("no steps defined", None);
    }

    let mut seen: Vec<&str> = Vec::new();
    for step in &doc.steps {
        let placeholder = step
            .id
            .as_deref()
            .map_or("<missing id>".to_owned(), |id| format!("'{id}'"));

        match step.id.as_deref() {
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

/// Run results, once the cited checklist's step ids are known.
pub fn run_results(doc: &Document, sop: &str, checklist_step_ids: &[String]) -> Diagnostics {
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
            Some(id) if !checklist_step_ids.iter().any(|known| known == id) => out.error(
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
            None => out.error(format!("result {name} has no 'status'"), Some(result.line)),
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
pub fn complete_run_coverage(doc: &Document, checklist_step_ids: &[String]) -> Diagnostics {
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
            out.error(
                format!("run is 'complete' but step '{id}' has no result"),
                Some(1),
            );
        }
    }
    for result in &doc.results {
        if result.status.as_deref() == Some("skipped") {
            out.error(
                format!(
                    "run is 'complete' but step {} was skipped",
                    result
                        .step
                        .as_deref()
                        .map_or("<missing>".to_owned(), |s| format!("'{s}'"))
                ),
                Some(result.line),
            );
        }
    }
    out
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
