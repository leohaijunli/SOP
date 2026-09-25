//! The editor's text operations.
//!
//! Every test here is about what an edit must *not* do as much as what it must do: keep
//! the neighbours, keep the comments, keep the keys this build does not know about, and
//! leave text that still parses. An edit that produces a valid file by rewriting the
//! whole thing would pass a "does it work" test and fail these.

use sop_core::Document;
use sop_core::authoring::{
    self, CaptureDraft, ExpectedDraft, ItemRef, Move, StepChanges, StepDraft,
};

const SAMPLE: &str = r#"---
kind: checklist
sop_id: sample
title: Sample
version: 1
updated: 2026-01-01
applies_to: [ground-survey]
---

Intro prose.

<!-- include: procedures/alpha.md -->

## First

```yaml step
id: first
kind: check
severity: normal
captures:
  - key: note
    label: A note
    type: text
    required: false
```

Read this first.

- [ ] Done

## Second

```yaml step
id: second
kind: measure
severity: critical
# a comment worth keeping
future_key: keep me
```

Second prose.
"#;

fn parse(text: &str) -> Document {
    Document::parse(text).expect("the result of an edit must parse")
}

fn step_ids(text: &str) -> Vec<String> {
    parse(text)
        .steps
        .iter()
        .filter_map(|step| step.id.clone())
        .collect()
}

fn draft(id: &str, title: &str) -> StepDraft {
    StepDraft {
        id: id.to_owned(),
        title: title.to_owned(),
        kind: "check".to_owned(),
        severity: "normal".to_owned(),
        deprecated: false,
        prose: String::new(),
        captures: Vec::new(),
    }
}

fn capture(key: &str, capture_type: &str) -> CaptureDraft {
    CaptureDraft {
        key: key.to_owned(),
        label: format!("Label for {key}"),
        capture_type: capture_type.to_owned(),
        required: true,
        ..CaptureDraft::default()
    }
}

// ------------------------------------------------------------------- adding

#[test]
fn a_new_step_is_appended_and_reads_back_as_written() {
    let mut draft = draft("third", "Third");
    draft.severity = "critical".to_owned();
    draft.kind = "measure".to_owned();
    draft.prose = "Do the thing.\n\n- [ ] Thing done".to_owned();
    let mut height = capture("staff_height", "number");
    height.unit = Some("m".to_owned());
    height.expected = Some(ExpectedDraft::Range {
        min: Some(0.5),
        max: Some(2.0),
    });
    draft.captures = vec![height, capture("sensor_choice", "select")];
    draft.captures[1].options = vec!["cal01".to_owned(), "cal02".to_owned()];

    let text = authoring::add_step(SAMPLE, &draft, None).unwrap();
    let doc = parse(&text);
    assert_eq!(step_ids(&text), ["first", "second", "third"]);

    let added = doc.steps.last().unwrap();
    assert_eq!(added.title.as_deref(), Some("Third"));
    assert_eq!(added.severity.as_deref(), Some("critical"));
    assert_eq!(added.key.as_ref().map(|k| k.as_str()), Some("measure"));
    assert!(added.prose.contains("Thing done"), "{}", added.prose);

    let height = &added.captures[0];
    assert_eq!(height.key.as_deref(), Some("staff_height"));
    assert_eq!(height.unit.as_deref(), Some("m"));
    assert_eq!(height.required, Some(true));
    assert!(text.contains("  - key: sensor_choice"), "{text}");
}

#[test]
fn a_step_can_be_inserted_after_a_named_entry() {
    let text = authoring::add_step(
        SAMPLE,
        &draft("between", "Between"),
        Some(&ItemRef::Step("first".to_owned())),
    )
    .unwrap();
    assert_eq!(step_ids(&text), ["first", "between", "second"]);
    // The include marker stays where it was.
    assert!(parse(&text).includes[0].line < parse(&text).steps[0].line);
}

#[test]
fn adding_a_step_twice_is_refused_rather_than_duplicated() {
    let error = authoring::add_step(SAMPLE, &draft("first", "Again"), None).unwrap_err();
    assert!(
        error.to_string().contains("already in this file"),
        "{error}"
    );
}

#[test]
fn a_draft_that_would_not_validate_is_refused_before_any_text_changes() {
    for (draft, expected) in [
        (draft("Not An Id", "Bad"), "not a valid id"),
        (draft("ok-id", "  "), "needs a title"),
    ] {
        let error = authoring::add_step(SAMPLE, &draft, None).unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }

    let mut bad_kind = draft("ok-id", "Bad kind");
    bad_kind.kind = "inspect".to_owned();
    let error = authoring::add_step(SAMPLE, &bad_kind, None).unwrap_err();
    assert!(error.to_string().contains("not a step kind"), "{error}");

    let mut bad_expected = draft("ok-id", "Bad expected");
    bad_expected.captures = vec![capture("count", "integer")];
    bad_expected.captures[0].expected = Some(ExpectedDraft::Range {
        min: Some(10.0),
        max: Some(1.0),
    });
    let error = authoring::add_step(SAMPLE, &bad_expected, None).unwrap_err();
    assert!(error.to_string().contains("cannot both hold"), "{error}");
}

// ----------------------------------------------------------------- removing

#[test]
fn removing_a_step_leaves_its_neighbours_alone() {
    let text = authoring::remove_step(SAMPLE, "first").unwrap();
    assert_eq!(step_ids(&text), ["second"]);
    assert!(
        text.contains("<!-- include: procedures/alpha.md -->"),
        "{text}"
    );
    assert!(text.contains("Second prose."), "{text}");
    assert!(!text.contains("Read this first."), "{text}");
    assert!(!text.contains("id: first"), "{text}");
    assert!(
        !text.contains("\n\n\n"),
        "no double blank line is left: {text:?}"
    );
}

#[test]
fn removing_a_step_that_is_not_there_says_which_one() {
    let error = authoring::remove_step(SAMPLE, "missing").unwrap_err();
    assert!(error.to_string().contains("no step 'missing'"), "{error}");
}

// ------------------------------------------------------------------ changing

#[test]
fn changing_a_step_keeps_its_captures_and_its_unknown_keys() {
    let changes = StepChanges {
        kind: Some("note".to_owned()),
        severity: Some("info".to_owned()),
        title: Some("Second, renamed".to_owned()),
        ..StepChanges::default()
    };
    let text = authoring::set_step(SAMPLE, "second", &changes).unwrap();

    assert!(text.contains("## Second, renamed"), "{text}");
    assert!(text.contains("kind: note"), "{text}");
    assert!(text.contains("severity: info"), "{text}");
    assert!(text.contains("# a comment worth keeping"), "{text}");
    assert!(text.contains("future_key: keep me"), "{text}");
    assert!(text.contains("Second prose."), "{text}");
    assert!(
        text.contains("  - key: note"),
        "First's captures are untouched"
    );

    let doc = parse(&text);
    let second = doc
        .steps
        .iter()
        .find(|s| s.id.as_deref() == Some("second"))
        .unwrap();
    assert_eq!(second.title.as_deref(), Some("Second, renamed"));
    assert_eq!(second.key.as_ref().map(|k| k.as_str()), Some("note"));
}

#[test]
fn prose_can_be_replaced_without_touching_the_block_above_it() {
    let changes = StepChanges {
        prose: Some("New instructions.\n\n- [ ] Newly checked".to_owned()),
        ..StepChanges::default()
    };
    let text = authoring::set_step(SAMPLE, "first", &changes).unwrap();
    let doc = parse(&text);
    let first = doc
        .steps
        .iter()
        .find(|s| s.id.as_deref() == Some("first"))
        .unwrap();
    assert!(first.prose.contains("New instructions."), "{}", first.prose);
    assert!(!first.prose.contains("Read this first."), "{}", first.prose);
    assert_eq!(first.captures.len(), 1, "the captures are still there");
    assert_eq!(step_ids(&text), ["first", "second"]);
}

#[test]
fn a_change_that_names_an_unknown_step_is_refused() {
    let changes = StepChanges {
        severity: Some("info".to_owned()),
        ..StepChanges::default()
    };
    let error = authoring::set_step(SAMPLE, "missing", &changes).unwrap_err();
    assert!(error.to_string().contains("no step 'missing'"), "{error}");
}

// ------------------------------------------------------------------- moving

#[test]
fn a_step_moves_past_its_neighbour_and_nothing_else_changes() {
    let down =
        authoring::move_item(SAMPLE, &ItemRef::Step("first".to_owned()), Move::Down).unwrap();
    assert_eq!(step_ids(&down), ["second", "first"]);
    assert!(down.contains("Second prose."), "{down}");
    assert!(down.contains("Read this first."), "{down}");

    let up = authoring::move_item(&down, &ItemRef::Step("first".to_owned()), Move::Up).unwrap();
    assert_eq!(step_ids(&up), ["first", "second"]);
    assert_eq!(up, SAMPLE, "moving down and back up is a no-op");
}

#[test]
fn an_include_marker_moves_like_any_other_entry() {
    let text = authoring::move_item(
        SAMPLE,
        &ItemRef::Include("procedures/alpha.md".to_owned()),
        Move::Down,
    )
    .unwrap();
    let doc = parse(&text);
    assert!(
        doc.includes[0].line > doc.steps[0].title_line.unwrap(),
        "the include is now below the first step"
    );
    assert_eq!(step_ids(&text), ["first", "second"]);
}

#[test]
fn moving_off_the_end_of_the_list_is_refused() {
    // The include marker is the first entry in this file, so nothing is above it.
    let first = ItemRef::Include("procedures/alpha.md".to_owned());
    let error = authoring::move_item(SAMPLE, &first, Move::Up).unwrap_err();
    assert!(
        error.to_string().contains("nothing on that side"),
        "{error}"
    );

    let error =
        authoring::move_item(SAMPLE, &ItemRef::Step("second".to_owned()), Move::Down).unwrap_err();
    assert!(
        error.to_string().contains("nothing on that side"),
        "{error}"
    );
}

#[test]
fn a_step_moves_past_an_include_marker_as_well_as_another_step() {
    // Moving "first" up puts it above the include, which is a real thing to want: the
    // marker is an entry in the sequence, not a decoration.
    let text = authoring::move_item(SAMPLE, &ItemRef::Step("first".to_owned()), Move::Up).unwrap();
    let doc = parse(&text);
    assert!(doc.steps[0].title_line.unwrap() < doc.includes[0].line);
    assert_eq!(step_ids(&text), ["first", "second"]);
}

// ----------------------------------------------------------------- captures

#[test]
fn a_capture_can_be_added_to_a_step_that_has_none() {
    let mut chosen = capture("second_capture", "select");
    chosen.options = vec!["yes".to_owned(), "no".to_owned()];
    chosen.expected = Some(ExpectedDraft::Select("no".to_owned()));
    let text = authoring::set_capture(SAMPLE, "second", &chosen).unwrap();

    let doc = parse(&text);
    let second = doc
        .steps
        .iter()
        .find(|s| s.id.as_deref() == Some("second"))
        .unwrap();
    assert_eq!(second.captures.len(), 1);
    assert_eq!(second.captures[0].options, ["yes", "no"]);
    assert!(
        text.contains("options: [\"yes\", \"no\"]"),
        "a bare yes would be read as a boolean by a YAML 1.1 reader: {text}"
    );
}

#[test]
fn an_existing_capture_is_replaced_in_place() {
    let mut edited = capture("note", "text");
    edited.label = "A better note".to_owned();
    let text = authoring::set_capture(SAMPLE, "first", &edited).unwrap();

    assert!(text.contains("label: A better note"), "{text}");
    assert!(!text.contains("label: A note"), "{text}");
    assert_eq!(
        parse(&text).steps[0].captures.len(),
        1,
        "not appended twice"
    );
}

#[test]
fn a_capture_is_removed_with_its_header_when_it_was_the_only_one() {
    let text = authoring::remove_capture(SAMPLE, "first", "note").unwrap();
    assert!(!text.contains("key: note"), "{text}");
    assert!(
        !text.contains("captures:"),
        "an empty captures key is removed: {text}"
    );

    let doc = parse(&text);
    assert_eq!(doc.steps[0].captures.len(), 0);
    assert_eq!(doc.steps[0].id.as_deref(), Some("first"));
}

#[test]
fn a_second_capture_can_be_added_beside_the_first() {
    let text = authoring::set_capture(SAMPLE, "first", &capture("second_note", "text")).unwrap();
    let doc = parse(&text);
    assert_eq!(doc.steps[0].captures.len(), 2);
    assert_eq!(
        doc.steps[0].captures[0].key.as_deref(),
        Some("note"),
        "the existing capture keeps its place"
    );
}

// ------------------------------------------------------------------ includes

#[test]
fn an_include_marker_can_be_added_and_removed() {
    let text = authoring::add_include(SAMPLE, "procedures/beta.md", None).unwrap();
    let doc = parse(&text);
    assert_eq!(doc.includes.len(), 2);
    assert_eq!(doc.includes[0].target, "procedures/alpha.md");
    assert_eq!(doc.includes[1].target, "procedures/beta.md");

    let text = authoring::remove_include(&text, "procedures/beta.md").unwrap();
    assert_eq!(parse(&text).includes.len(), 1);
    assert_eq!(text.matches("<!-- include").count(), 1);
}

#[test]
fn including_the_same_procedure_twice_is_refused() {
    let error = authoring::add_include(SAMPLE, "procedures/alpha.md", None).unwrap_err();
    assert!(error.to_string().contains("already included"), "{error}");
}

#[test]
fn every_edit_leaves_a_file_that_still_parses() {
    // A cheap cross-check: whatever an operation produces, it is a document. This is the
    // invariant the whole module rests on, and it is easy to break with an off-by-one.
    let edits: Vec<String> = vec![
        authoring::add_step(SAMPLE, &draft("extra", "Extra"), None).unwrap(),
        authoring::remove_step(SAMPLE, "second").unwrap(),
        authoring::set_step(
            SAMPLE,
            "first",
            &StepChanges {
                prose: Some("Replaced.".to_owned()),
                ..StepChanges::default()
            },
        )
        .unwrap(),
        authoring::set_step(
            SAMPLE,
            "first",
            &StepChanges {
                prose: Some(String::new()),
                ..StepChanges::default()
            },
        )
        .unwrap(),
        authoring::move_item(SAMPLE, &ItemRef::Step("second".to_owned()), Move::Up).unwrap(),
        authoring::set_capture(SAMPLE, "first", &capture("extra", "text")).unwrap(),
        authoring::remove_capture(SAMPLE, "first", "note").unwrap(),
        authoring::add_include(SAMPLE, "procedures/beta.md", None).unwrap(),
        authoring::remove_include(SAMPLE, "procedures/alpha.md").unwrap(),
    ];
    for (index, text) in edits.iter().enumerate() {
        let doc = parse(text);
        assert!(doc.line_count > 0, "edit {index} produced nothing");
        assert_eq!(
            doc.front.str("sop_id"),
            Some(Some("sample")),
            "edit {index}"
        );
        assert!(text.ends_with('\n'), "edit {index} must end with a newline");
        assert!(
            !text.contains("\n\n\n"),
            "edit {index} left a double blank line"
        );
    }
}
