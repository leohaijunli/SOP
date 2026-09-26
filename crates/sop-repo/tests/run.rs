//! A run's life: start it, record events, recover it, end it.
//!
//! These exercise `sop-repo::run` against a scratch copy of the real repository, and
//! check the promises the format makes: a snapshot is frozen at start, a crash is
//! recovered by replaying the log, the rendered record agrees with the events, and a
//! skipped/deviated step cannot be recorded without a reason.

use sop_core::RunEvent;
use sop_repo::{Repo, run};

mod common;
use common::Scratch;

const SOP: &str = "ground-walk-survey";
const RUN: &str = "2026-09-25-test-run";

/// `ground-walk-survey` is a draft, so these tests always start it over an override.
fn start(repo: &Repo, run_id: &str) {
    run::start(repo, SOP, run_id, "leo", "Renfrew 395", Some("automated test"))
        .unwrap_or_else(|error| panic!("start failed: {error}"));
}

#[test]
fn a_run_starts_opens_and_records_a_result() {
    let scratch = Scratch::new("run-lifecycle");
    let repo = scratch.repo();
    start(&repo, RUN);

    let loaded = run::load(&repo, SOP, RUN).unwrap();
    assert_eq!(loaded.state.sop.as_deref(), Some(SOP));
    assert_eq!(loaded.state.operator.as_deref(), Some("leo"));
    assert!(loaded.state.snapshot_sha256.is_some());
    assert!(
        scratch
            .path("runs/ground-walk-survey/2026-09-25-test-run/snapshot.md")
            .is_file()
    );

    run::record(
        &repo,
        SOP,
        RUN,
        &RunEvent::CaptureRecorded {
            at: "t".into(),
            step: "cond-location".into(),
            key: "session_location".into(),
            value: "hill top".into(),
            unit: None,
        },
    )
    .unwrap();
    run::record(
        &repo,
        SOP,
        RUN,
        &RunEvent::CheckboxToggled {
            at: "t".into(),
            step: "cond-location".into(),
            index: 0,
            checked: true,
        },
    )
    .unwrap();
    run::record(
        &repo,
        SOP,
        RUN,
        &RunEvent::StepStatusChanged {
            at: "t".into(),
            step: "cond-location".into(),
            status: "done".into(),
            reason: None,
        },
    )
    .unwrap();

    let loaded = run::load(&repo, SOP, RUN).unwrap();
    let step = loaded.state.step("cond-location").unwrap();
    assert_eq!(step.status.as_str(), "done");
    assert!(step.checkboxes[0]);
    assert_eq!(step.captures["session_location"].value, "hill top");

    run::end(&repo, SOP, RUN, "complete").unwrap();
    assert!(
        scratch
            .path("runs/ground-walk-survey/2026-09-25-test-run.md")
            .is_file(),
        "the human record is written next to the run directory"
    );
    let record = scratch.read("runs/ground-walk-survey/2026-09-25-test-run/record.md");
    assert!(record.contains("step: cond-location"), "{record}");
    assert!(record.contains("session_location: hill top"), "{record}");
    assert!(record.contains("status: done"), "{record}");
    assert!(record.ends_with("deviations_count: 0\n"), "{record}");
}

#[test]
fn a_draft_checklist_will_not_start_without_an_override() {
    let scratch = Scratch::new("run-draft");
    let repo = scratch.repo();
    let err = run::start(&repo, SOP, "2026-09-25-draft-block", "leo", "site", None).unwrap_err();
    assert!(
        format!("{err}").contains("is a draft"),
        "draft checklists are blocked: {err}"
    );
}

#[test]
fn a_skip_without_a_reason_is_rejected() {
    let scratch = Scratch::new("run-skip");
    let repo = scratch.repo();
    let run_id = "2026-09-25-skip";
    start(&repo, run_id);
    let err = run::record(
        &repo,
        SOP,
        run_id,
        &RunEvent::StepStatusChanged {
            at: "t".into(),
            step: "cond-location".into(),
            status: "skipped".into(),
            reason: None,
        },
    )
    .unwrap_err();
    assert!(
        format!("{err}").contains("reason"),
        "a skipped step must carry a reason: {err}"
    );
}

#[test]
fn a_torn_final_log_line_is_recovered() {
    let scratch = Scratch::new("run-torn");
    let repo = scratch.repo();
    let run_id = "2026-09-25-torn";
    start(&repo, run_id);
    run::record(
        &repo,
        SOP,
        run_id,
        &RunEvent::CaptureRecorded {
            at: "t".into(),
            step: "cond-location".into(),
            key: "session_location".into(),
            value: "hill top".into(),
            unit: None,
        },
    )
    .unwrap();

    // Simulate a crash that left half a line behind.
    let rel = "runs/ground-walk-survey/2026-09-25-torn/events.jsonl";
    let text = scratch.read(rel);
    scratch.write(rel, &format!("{text}{{\"type\":\"RunEnded\""));

    let loaded = run::load(&repo, SOP, run_id).unwrap();
    assert!(
        loaded.state.ended.is_none(),
        "the torn trailing line is ignored on recovery"
    );
    assert_eq!(
        loaded.state.step("cond-location").unwrap().captures["session_location"].value,
        "hill top"
    );
}

#[test]
fn the_rendered_record_matches_the_events() {
    let scratch = Scratch::new("run-crosscheck");
    let repo = scratch.repo();
    let run_id = "2026-09-25-cc";
    start(&repo, run_id);
    run::record(
        &repo,
        SOP,
        run_id,
        &RunEvent::StepStatusChanged {
            at: "t".into(),
            step: "walk-briefing".into(),
            status: "deviated".into(),
            reason: Some("wind shifted the plan".into()),
        },
    )
    .unwrap();
    run::end(&repo, SOP, run_id, "partial").unwrap();

    let record = scratch.read("runs/ground-walk-survey/2026-09-25-cc/record.md");
    assert!(record.contains("status: deviated"), "{record}");
    assert!(record.contains("reason: wind shifted the plan"), "{record}");
    assert!(record.contains("deviations_count: 1"), "{record}");
}