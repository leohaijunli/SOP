//! Tests for the parsing half of `SPEC.md`.
//!
//! The rule set is covered end to end by `sop-repo`'s negative tests; these tests cover
//! the primitives those rules are built from, where a silent change would be hard to
//! attribute.

use sop_core::{CaptureType, Document, Expected, ParseError, check, front, link};

fn parse(text: &str) -> Document {
    Document::parse(text).expect("document should parse")
}

fn procedure(body: &str) -> String {
    format!(
        "---\nkind: procedure\nprocedure_id: sample\ntitle: Sample\nversion: 1\n\
         updated: 2026-01-02\napplies_to: [calibration]\n---\n\n{body}"
    )
}

// ---------------------------------------------------------------- front matter

#[test]
fn front_matter_is_split_off_and_line_numbers_are_absolute() {
    let doc = parse(&procedure("## A\n\ntext\n"));
    assert_eq!(doc.kind.as_deref(), Some("procedure"));
    assert_eq!(
        doc.body_start_line, 9,
        "the blank line after the closing delimiter"
    );
    assert_eq!(doc.headings[0].line, 10);
}

#[test]
fn an_empty_front_matter_block_is_an_empty_mapping() {
    let doc = parse("---\n---\n\nbody\n");
    assert!(doc.front.keys().next().is_none());
    assert_eq!(doc.body_start_line, 3);
}

#[test]
fn a_missing_front_matter_delimiter_is_reported_at_line_one() {
    let error = Document::parse("kind: procedure\n").unwrap_err();
    assert!(matches!(error, ParseError::MissingFrontMatter { line: 1 }));
}

#[test]
fn an_unclosed_front_matter_block_is_reported() {
    let error = Document::parse("---\nkind: procedure\n").unwrap_err();
    assert!(matches!(error, ParseError::UnclosedFrontMatter));
}

#[test]
fn front_matter_that_is_not_a_mapping_is_rejected() {
    let error = Document::parse("---\n- a\n- b\n---\n\nbody\n").unwrap_err();
    assert!(matches!(error, ParseError::FrontMatterNotMapping));
}

#[test]
fn a_scalar_front_matter_value_expands_to_a_one_element_list() {
    let doc = parse("---\nkind: procedure\napplies_to: calibration\ntags: 5\n---\n\nx\n");
    assert_eq!(
        doc.front.string_list("applies_to"),
        vec!["calibration".to_owned()]
    );
    assert!(doc.front.string_list_is_well_formed("applies_to"));
    assert!(
        !doc.front.string_list_is_well_formed("tags"),
        "a number is not a list"
    );
}

// --------------------------------------------------------------------- fences

#[test]
fn only_fences_at_column_zero_are_recognised() {
    let doc = parse(&procedure("    ```yaml step\n    id: indented\n    ```\n"));
    assert!(
        doc.steps.is_empty(),
        "an indented fence is prose, not a block"
    );
}

#[test]
fn a_longer_fence_may_contain_a_shorter_one() {
    let doc = parse(&procedure(
        "````yaml step\nid: outer\nkind: check\ncaptures:\n  - key: k\n    label: L\n    type: text\n````\n",
    ));
    assert_eq!(doc.steps.len(), 1);
    assert_eq!(doc.steps[0].id.as_deref(), Some("outer"));
}

#[test]
fn an_unclosed_fence_is_an_error() {
    let error = Document::parse(&procedure("## A\n\n```yaml step\nid: x\n")).unwrap_err();
    assert!(matches!(error, ParseError::UnclosedFence { line: 12 }));
}

#[test]
fn step_blocks_report_their_own_line_span() {
    let doc = parse(&procedure(
        "## A\n\n```yaml step\nid: a\nkind: check\n```\n\nafter\n",
    ));
    let step = &doc.steps[0];
    assert_eq!(step.line, 12, "the opening fence");
    assert_eq!(step.block_span, (12, 16), "the fence, half-open");
    assert_eq!(step.span.0, 10, "the step starts at its heading");
}

#[test]
fn a_step_takes_the_nearest_preceding_second_level_heading() {
    let doc = parse(&procedure(
        "### Not a step title\n\n```yaml step\nid: a\nkind: check\n```\n\n\
         ## Real title\n\nprose\n\n```yaml step\nid: b\nkind: check\n```\n",
    ));
    assert_eq!(doc.steps[0].title, None);
    assert_eq!(doc.steps[1].title.as_deref(), Some("Real title"));
    assert!(!check::steps(&doc, true).has_errors());
}

// --------------------------------------------------------------- step contents

#[test]
fn a_capture_distinguishes_a_null_value_from_a_missing_key() {
    let doc = parse(&procedure(
        "## A\n\n```yaml step\nid: a\nkind: check\ncaptures:\n  - key: k\n    label: L\n    \
         type: select\n    options: [x]\n  - key: j\n    label:\n    type: text\n```\n",
    ));
    let first = &doc.steps[0].captures[0];
    let second = &doc.steps[0].captures[1];
    assert!(!first.declares("unit"));
    assert_eq!(second.capture_type, Some(CaptureType::Text));
    assert!(second.declares("label") && second.label.is_none());
}

#[test]
fn expected_shapes_are_parsed_into_something_renderable() {
    let cases = [
        ("expected: true", Expected::Bool(true)),
        ("expected: ok", Expected::Select("ok".to_owned())),
        (
            "expected: {min: 1.5}",
            Expected::Range {
                min: Some(1.5),
                max: None,
            },
        ),
        (
            "expected: {min: 1, max: 2}",
            Expected::Range {
                min: Some(1.0),
                max: Some(2.0),
            },
        ),
    ];
    for (text, want) in cases {
        let body = format!(
            "## A\n\n```yaml step\nid: a\nkind: check\ncaptures:\n  - key: k\n    \
                            label: L\n    type: number\n    {text}\n```\n"
        );
        let doc = parse(&procedure(&body));
        assert_eq!(
            doc.steps[0].captures[0].expected.as_ref(),
            Some(&want),
            "{text}"
        );
    }
}

#[test]
fn an_unusable_expected_shape_carries_its_own_reason() {
    let body = "## A\n\n```yaml step\nid: a\nkind: check\ncaptures:\n  - key: k\n    label: L\n    \
                type: number\n    expected: 12\n```\n";
    let doc = parse(&procedure(body));
    match doc.steps[0].captures[0].expected.as_ref() {
        Some(Expected::Malformed(reason)) => assert!(reason.contains("min")),
        other => panic!("expected a malformed range, got {other:?}"),
    }
}

#[test]
fn expected_is_checked_against_its_own_capture_type() {
    let cases = [
        ("type: bool", "expected: nope", "must be true or false"),
        ("type: bool", "expected: true", ""),
        (
            "type: select\n    options: [a, b]",
            "expected: true",
            "must be one of its own options",
        ),
        ("type: select\n    options: [a, b]", "expected: b", ""),
        ("type: number", "expected: high", "must be a mapping"),
    ];
    for (capture_type, expected, want) in cases {
        let body = format!(
            "## A\n\n```yaml step\nid: a\nkind: check\ncaptures:\n  - key: k\n    label: L\n    \
             {capture_type}\n    {expected}\n```\n"
        );
        let doc = parse(&procedure(&body));
        let found = check::steps(&doc, true);
        let messages: Vec<&str> = found.items().iter().map(|d| d.message.as_str()).collect();
        if want.is_empty() {
            assert!(
                messages.is_empty(),
                "{capture_type} / {expected} should be fine: {messages:?}"
            );
        } else {
            assert!(
                messages.iter().any(|m| m.contains(want)),
                "{capture_type} / {expected} should report {want:?}, got {messages:?}"
            );
        }
    }
}

#[test]
fn a_capture_is_located_on_its_own_line_when_it_can_be() {
    let doc = parse(&procedure(
        "## A\n\n```yaml step\nid: a\nkind: check\ncaptures:\n  - key: found\n    label: L\n    type: text\n```\n",
    ));
    assert_eq!(doc.steps[0].captures[0].line, Some(16));
}

#[test]
fn a_step_block_that_is_not_a_mapping_is_rejected() {
    let error = Document::parse(&procedure("## A\n\n```yaml step\n- a\n- b\n```\n")).unwrap_err();
    assert!(matches!(error, ParseError::BlockNotMapping { .. }));
}

// -------------------------------------------------------- include markers

#[test]
fn include_markers_are_collected_in_document_order_and_only_outside_fences() {
    let doc = parse(&procedure(
        "<!-- include: procedures/a.md -->\n\n```yaml step\nid: local\nkind: check\n```\n\n\
         <!-- include: procedures/b.md -->\n",
    ));
    let targets: Vec<&str> = doc.includes.iter().map(|i| i.target.as_str()).collect();
    assert_eq!(targets, ["procedures/a.md", "procedures/b.md"]);
}

#[test]
fn an_include_marker_inside_a_fence_is_not_an_include() {
    let doc = parse(&procedure(
        "```yaml step\nid: a\nkind: check\n```\n\n```\n<!-- include: procedures/a.md -->\n```\n",
    ));
    assert!(doc.includes.is_empty());
}

// --------------------------------------------------------------------- links

#[test]
fn link_scanning_matches_the_reference_pattern() {
    let body = "see [a doc](docs/x.md) and [anchored](docs/y.md#top)\n\
                [external](https://example.com/z) [anchor](#places)\n\
                [spaced](docs/a b.md) [mail](mailto:x@example.com)\n";
    let doc = parse(&procedure(body));
    let targets: Vec<String> = check::link_targets(&doc)
        .iter()
        .map(|l| l.target.clone())
        .collect();
    assert_eq!(targets, ["docs/x.md", "docs/y.md#top"]);
}

#[test]
fn a_link_fragment_is_separated_from_its_path() {
    let found = link::scan("x [t](docs/y.md#top)\n", 1);
    assert_eq!(found[0].target, "docs/y.md#top");
    assert_eq!(found[0].path(), "docs/y.md");
}

// --------------------------------------------------------------- checkboxes

#[test]
fn checkbox_state_recognises_the_three_bullets_and_both_markers() {
    assert_eq!(check::checkbox_state("- [ ] todo"), Some(' '));
    assert_eq!(check::checkbox_state("* [x] done"), Some('x'));
    assert_eq!(check::checkbox_state("+ [X] done"), Some('X'));
    assert_eq!(check::checkbox_state("  - [ ] indented"), Some(' '));
    assert_eq!(check::checkbox_state("- [q] not a checkbox"), None);
    assert_eq!(check::checkbox_state("- plain item"), None);
}

// -------------------------------------------------------------- formatting

#[test]
fn formatting_rules_from_the_spec_are_each_reported() {
    assert!(check::formatting("a\n").is_empty());
    assert!(!check::formatting("a").is_empty(), "no final newline");
    assert!(
        !check::formatting("a\n\n").is_empty(),
        "too many final newlines"
    );
    assert!(!check::formatting("a\r\n").is_empty(), "CR characters");
    let tabbed = check::formatting("a\n\tindented\n");
    assert_eq!(tabbed.items()[0].line, Some(2));
    assert_eq!(check::formatting("  \tindented\n").items().len(), 1);
    assert!(check::formatting("  spaces are fine\n").is_empty());
}

// ------------------------------------------------------------- vocabularies

#[test]
fn identifiers_respect_their_two_alphabets() {
    use sop_core::vocab::{is_iso_date, is_valid_capture_key, is_valid_id};
    assert!(is_valid_id("walk-line") && is_valid_id("a1"));
    assert!(!is_valid_id("Walk-line") && !is_valid_id("-a") && !is_valid_id("a_b"));
    assert!(is_valid_capture_key("walk_line") && !is_valid_capture_key("walk-line"));
    assert!(is_iso_date("2026-09-24") && !is_iso_date("2026-13-01") && !is_iso_date("26-09-24"));
}

// ------------------------------------------------------- editing front matter

#[test]
fn setting_a_field_changes_only_that_line() {
    let text = "---\nkind: project\n# a comment worth keeping\ntitle: Old Name\nproject_id: sample\n---\n\nBody text.\n";
    let updated = front::set_field(text, "title", "New Name").unwrap();
    assert_eq!(
        updated,
        "---\nkind: project\n# a comment worth keeping\ntitle: New Name\nproject_id: sample\n---\n\nBody text.\n"
    );
}

#[test]
fn setting_a_missing_field_adds_it_before_the_closing_delimiter() {
    let updated = front::set_field(
        "---\nkind: project\ntitle: A\n---\n\nBody.\n",
        "lead",
        "leo",
    )
    .expect("front matter is present");
    let doc = parse(&updated);
    assert_eq!(doc.front.str("lead"), Some(Some("leo")));
    assert!(
        updated.ends_with("---\n\nBody.\n"),
        "the body must survive: {updated:?}"
    );
    assert!(
        !updated.contains("lead: leo\nkind"),
        "the new key belongs in the front matter, not at the top"
    );
}

#[test]
fn a_nested_key_is_not_mistaken_for_a_top_level_one() {
    let text = "---\nkind: run\ncaptures:\n  title: inner\ntitle: outer\n---\n\nBody.\n";
    let updated = front::set_field(text, "title", "changed").unwrap();
    assert!(updated.contains("  title: inner"), "{updated:?}");
    assert!(updated.contains("title: changed"), "{updated:?}");
}

#[test]
fn a_value_a_yaml_reader_would_misread_is_quoted() {
    // YAML 1.1 reads a bare `no` as false. A survey answer really can be `no`, and the
    // record must mean the same thing to every reader.
    for value in [
        "no", "yes", "true", "null", "1", "1.5", "2026", "on", "off", "~",
    ] {
        assert_eq!(
            front::format_scalar(value),
            format!("\"{value}\""),
            "{value} must be quoted"
        );
    }
    let updated = front::set_field("---\nkind: help\ntitle: x\n---\n", "title", "no").unwrap();
    assert_eq!(
        parse(&updated).front.str("title"),
        Some(Some("no")),
        "a quoted value must read back as the text that was typed"
    );
}

#[test]
fn values_that_read_the_same_either_way_are_left_alone() {
    for value in [
        "Geomagnetic Survey Field Work",
        "uvic-geomag-survey",
        "2026-09-25",
        "Renfrew, BC",
    ] {
        assert_eq!(
            front::format_scalar(value),
            value,
            "{value} needs no quotes"
        );
    }
    assert_eq!(front::format_scalar(""), "\"\"");
    assert_eq!(front::format_scalar("trailing "), "\"trailing \"");
    // These would change meaning or lose text if they were written bare.
    assert_eq!(
        front::format_scalar("Survey: Renfrew"),
        "\"Survey: Renfrew\""
    );
    assert_eq!(front::format_scalar("North #2"), "\"North #2\"");
    assert_eq!(front::format_scalar("- walk east"), "\"- walk east\"");
    assert_eq!(front::format_scalar("-5"), "\"-5\"");
}

#[test]
fn a_document_without_front_matter_cannot_be_edited() {
    assert_eq!(
        front::set_field("no front matter here\n", "title", "x"),
        None
    );
    assert_eq!(
        front::set_field("---\nunclosed: true\n", "title", "x"),
        None
    );
}
