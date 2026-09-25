//! Writing an edit back: refusing it, or undoing it.
//!
//! `sop-core`'s tests cover the text. These cover the promises made around it: that only
//! content is editable, that nothing is written when the file itself would not validate,
//! and that an edit which breaks *another* file is put back.

use sop_core::authoring::{ItemRef, Move, StepChanges, StepDraft};
use sop_repo::{authoring, manifest, validate};

mod common;
use common::Scratch;

/// A manifest field, which is JSON because the manifest is what the app loads.
fn json_str(value: Option<&serde_json::Value>) -> Option<&str> {
    value.and_then(serde_json::Value::as_str)
}

const WALK: &str = "checklists/ground-walk-survey.md";
const WALK_RUN: &str = "runs/ground-walk-survey/2026-09-24-renfrew-walk01.md";

fn draft(id: &str, title: &str) -> StepDraft {
    StepDraft {
        id: id.to_owned(),
        title: title.to_owned(),
        kind: "note".to_owned(),
        severity: "normal".to_owned(),
        prose: "Added by the editor.".to_owned(),
        ..StepDraft::default()
    }
}

#[test]
fn a_good_edit_is_written_and_the_repository_still_validates() {
    let scratch = Scratch::new("authoring-good");
    let repo = scratch.repo();

    let changes = StepChanges {
        prose: Some("Rewritten by the editor.\n\n- [ ] Checked".to_owned()),
        ..StepChanges::default()
    };
    let path = authoring::update_step(&repo, WALK, "walk-briefing", &changes).unwrap();
    assert!(path.is_file());

    let text = scratch.read(WALK);
    assert!(text.contains("Rewritten by the editor."), "{text}");
    assert!(
        text.contains("## Team briefing"),
        "the heading and its neighbours survive: {text}"
    );

    let (failed, report) = scratch.validate();
    assert!(!failed, "the repository must still validate:\n{report}");
    assert!(report.ends_with("0 error(s), 0 warning(s)\n"), "{report}");
}

#[test]
fn a_new_step_reaches_the_manifest() {
    let scratch = Scratch::new("authoring-manifest");
    let repo = scratch.repo();
    let mut draft = draft("walk-extra-check", "Extra check");
    draft.kind = "check".to_owned();
    draft.severity = "critical".to_owned();

    authoring::add_step(&repo, WALK, &draft, None).unwrap();

    let built = manifest::build(&repo);
    let checklist = built
        .checklists
        .iter()
        .find(|checklist| json_str(checklist.sop_id.as_ref()) == Some("ground-walk-survey"))
        .expect("the checklist is in the manifest");
    assert!(
        checklist
            .steps
            .iter()
            .any(|step| json_str(step.id.as_ref()) == Some("walk-extra-check")),
        "the app reads the manifest, so a step that is not in it is not in the app"
    );
}

#[test]
fn an_edit_that_breaks_another_file_is_undone() {
    let scratch = Scratch::new("authoring-undone");
    let repo = scratch.repo();
    let before = scratch.read(WALK);
    assert!(
        scratch.read(WALK_RUN).contains("step: walk-line-plan"),
        "fixture drifted: the run record no longer cites this step"
    );

    let error = authoring::remove_step(&repo, WALK, "walk-line-plan").unwrap_err();
    let message = error.to_string();
    assert!(message.contains("undone"), "{message}");
    assert!(
        message.contains("walk-line-plan"),
        "the message must name what broke: {message}"
    );
    assert_eq!(
        scratch.read(WALK),
        before,
        "the file must be exactly as it was"
    );
    let (failed, report) = scratch.validate();
    assert!(!failed, "and the repository must be usable:\n{report}");
}

#[test]
fn an_edit_whose_own_file_would_not_validate_is_never_written() {
    let scratch = Scratch::new("authoring-rejected");
    let repo = scratch.repo();
    let before = scratch.read(WALK);

    // A select with no options is refused by the file's own rules.
    let mut capture = sop_core::authoring::CaptureDraft {
        key: "empty_select".to_owned(),
        label: "A select with no options".to_owned(),
        capture_type: "select".to_owned(),
        required: true,
        ..Default::default()
    };
    capture.options.clear();
    let error = authoring::set_capture(&repo, WALK, "walk-briefing", &capture).unwrap_err();
    assert!(error.to_string().contains("at least one option"), "{error}");
    assert_eq!(scratch.read(WALK), before);

    // And a duplicate step id is refused by the editor before the file is checked.
    let error =
        authoring::add_step(&repo, WALK, &draft("walk-briefing", "Again"), None).unwrap_err();
    assert!(
        error.to_string().contains("already in this file"),
        "{error}"
    );
    assert_eq!(scratch.read(WALK), before);
}

#[test]
fn only_content_is_editable() {
    let scratch = Scratch::new("authoring-scope");
    let repo = scratch.repo();

    for relative in [
        "runs/ground-walk-survey/2026-09-24-renfrew-walk01.md",
        "project.md",
        "help/commands.md",
        "SPEC.md",
        "procedures/../project.md",
        "../outside.md",
        "checklists/../../etc/passwd",
    ] {
        let error = authoring::editable(&repo, relative).unwrap_err();
        assert!(
            error.to_string().contains("not a file the editor changes"),
            "{relative}: {error}"
        );
    }

    for relative in [
        "procedures/walk-line.md",
        "checklists/ground-walk-survey.md",
    ] {
        assert!(authoring::editable(&repo, relative).is_ok(), "{relative}");
    }
}

#[test]
fn moving_an_entry_is_written_and_nothing_breaks() {
    let scratch = Scratch::new("authoring-move");
    let repo = scratch.repo();
    let before: Vec<String> = manifest::build(&repo)
        .checklists
        .iter()
        .find(|c| json_str(c.sop_id.as_ref()) == Some("ground-walk-survey"))
        .unwrap()
        .steps
        .iter()
        .map(|step| json_str(step.id.as_ref()).unwrap_or_default().to_owned())
        .collect();

    authoring::move_item(
        &repo,
        WALK,
        &ItemRef::Step("walk-session-closeout".to_owned()),
        Move::Up,
    )
    .unwrap();

    let after: Vec<String> = manifest::build(&repo)
        .checklists
        .iter()
        .find(|c| json_str(c.sop_id.as_ref()) == Some("ground-walk-survey"))
        .unwrap()
        .steps
        .iter()
        .map(|step| json_str(step.id.as_ref()).unwrap_or_default().to_owned())
        .collect();

    assert_eq!(after.len(), before.len(), "a move changes order, not count");
    assert_ne!(after, before, "the order must actually have changed");
    let (failed, report) = scratch.validate();
    assert!(!failed, "{report}");
}

#[test]
fn an_include_can_be_added_to_a_checklist() {
    let scratch = Scratch::new("authoring-include");
    let repo = scratch.repo();
    // `static-noise-test` is a procedure that no checklist includes yet.
    authoring::add_include(&repo, WALK, "procedures/static-noise-test.md", None).unwrap();

    let text = scratch.read(WALK);
    assert!(
        text.contains("<!-- include: procedures/static-noise-test.md -->"),
        "{text}"
    );
    let built = manifest::build(&repo);
    let checklist = built
        .checklists
        .iter()
        .find(|c| json_str(c.sop_id.as_ref()) == Some("ground-walk-survey"))
        .unwrap();
    assert!(
        checklist
            .steps
            .iter()
            .any(|step| json_str(step.source.as_ref()) == Some("procedures/static-noise-test.md")),
        "the included procedure's steps must appear in the checklist"
    );
    let (failed, report) = scratch.validate();
    assert!(!failed, "{report}");
}

#[test]
fn check_text_agrees_with_a_full_pass_about_one_file() {
    let scratch = Scratch::new("authoring-check-text");
    let repo = scratch.repo();
    let path = scratch.path(WALK);

    let text = scratch.read(WALK);
    assert!(!validate::check_text(&repo, &path, &text).has_errors());

    let broken = text.replace("kind: checklist", "kind: procedure");
    let report = validate::check_text(&repo, &path, &broken);
    assert!(report.has_errors(), "a wrong kind must be an error");
    assert!(
        report.render(repo.root()).contains(WALK),
        "and it must name the file"
    );
}
