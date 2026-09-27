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
fn a_run_records_the_instrument_and_conditions_it_was_started_with() {
    let scratch = Scratch::new("run-meta");
    let repo = scratch.repo();
    let mut conditions = std::collections::BTreeMap::new();
    conditions.insert("weather".to_owned(), "clear".to_owned());
    conditions.insert("temp_c".to_owned(), "12".to_owned());
    let meta = run::RunMeta {
        sensor: Some(sop_core::run::SensorIdentity {
            model: "GEM GSM-19".to_owned(),
            // A leading digit is what the quoting exists for: bare, YAML reads it as an
            // integer and the serial is lost to a `str()` read.
            serial: "4451233".to_owned(),
            firmware: "7.0".to_owned(),
        }),
        hardware: vec!["mag_gcs v0.3.1".to_owned(), "tripod".to_owned()],
        conditions,
    };
    run::start_with_meta(&repo, SOP, "2026-09-25-meta", "leo", "Renfrew 395", Some("test"), &meta)
        .unwrap();
    run::end(&repo, SOP, "2026-09-25-meta", "partial").unwrap();

    let record = scratch.read("runs/ground-walk-survey/2026-09-25-meta.md");
    assert!(record.contains("sensor:\n  model: GEM GSM-19\n  serial: \"4451233\"\n  firmware: \"7.0\""), "{record}");
    assert!(record.contains("hardware:\n  - mag_gcs v0.3.1\n  - tripod"), "{record}");
    assert!(record.contains("conditions:\n  temp_c: \"12\"\n  weather: clear"), "{record}");
    // The head restates the instrument for a reader who only opens the record.
    assert!(record.contains("sensor: GEM GSM-19, serial 4451233, firmware 7.0"), "{record}");
}

#[test]
fn an_open_step_keeps_its_captures_and_has_no_outcome() {
    let scratch = Scratch::new("run-open-capture");
    let repo = scratch.repo();
    start(&repo, "2026-09-25-open");
    run::record(
        &repo,
        SOP,
        "2026-09-25-open",
        &RunEvent::CaptureRecorded {
            at: "t".into(),
            step: "cond-location".into(),
            key: "session_location".into(),
            value: "hill top".into(),
            unit: None,
        },
    )
    .unwrap();

    let record = scratch.read("runs/ground-walk-survey/2026-09-25-open/record.md");
    assert!(record.contains("session_location: hill top"), "the data someone typed must survive: {record}");
    assert!(
        !record.contains("status: open"),
        "an open step has no outcome line rather than a placeholder: {record}"
    );
}

#[test]
fn a_complete_run_cannot_leave_a_step_with_no_outcome() {
    let scratch = Scratch::new("run-no-outcome");
    let repo = scratch.repo();
    start(&repo, "2026-09-25-no-outcome");
    run::record(
        &repo,
        SOP,
        "2026-09-25-no-outcome",
        &RunEvent::CaptureRecorded {
            at: "t".into(),
            step: "cond-location".into(),
            key: "session_location".into(),
            value: "hill top".into(),
            unit: None,
        },
    )
    .unwrap();
    run::end(&repo, SOP, "2026-09-25-no-outcome", "complete").unwrap();

    let (failed, report) = scratch.validate();
    assert!(failed, "a complete run with an unanswered step must not validate:\n{report}");
    assert!(report.contains("records no outcome"), "{report}");
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

#[test]
fn the_record_carries_the_checkbox_items_as_the_operator_left_them() {
    let scratch = Scratch::new("run-record-checkboxes");
    let repo = scratch.repo();
    start(&repo, RUN);
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
    run::end(&repo, SOP, RUN, "partial").unwrap();

    let text = run::record_text(&repo, SOP, RUN).unwrap();
    assert!(
        text.contains("- [x] Location described well enough to find again"),
        "{text}"
    );
    assert!(
        text.contains("- [ ] Nearby sources noted, or explicitly recorded as none known"),
        "an item nobody touched must stay unticked: {text}"
    );
    assert_eq!(
        text,
        scratch.read(&format!("runs/{SOP}/{RUN}.md")),
        "the exported document is the committed one"
    );
}

#[test]
fn a_record_without_a_run_directory_is_read_from_the_file() {
    let scratch = Scratch::new("run-record-document");
    let repo = scratch.repo();

    // The hand-written run in the fixture has a record file and no run directory, which
    // is the shape SPEC.md section 8 says to expect from before runs were self-contained.
    let existing = "2026-09-24-renfrew-walk01";
    assert!(!scratch.path(&format!("runs/{SOP}/{existing}")).exists());
    assert_eq!(
        run::record_document(&repo, SOP, existing).unwrap(),
        scratch.read(&format!("runs/{SOP}/{existing}.md")),
        "the file is the record when there is nothing to re-render"
    );

    // A run the app recorded still comes back re-rendered from its own log.
    start(&repo, RUN);
    run::end(&repo, SOP, RUN, "partial").unwrap();
    assert_eq!(
        run::record_document(&repo, SOP, RUN).unwrap(),
        run::record_text(&repo, SOP, RUN).unwrap()
    );

    assert!(run::record_document(&repo, SOP, "no-such-run").is_err());
}

#[test]
fn deleting_a_run_removes_its_paths_and_leaves_every_other_run_alone() {
    let scratch = Scratch::new("run-delete");
    let repo = scratch.repo();
    start(&repo, RUN);
    run::end(&repo, SOP, RUN, "partial").unwrap();
    let record = format!("runs/{SOP}/{RUN}.md");
    let dir = format!("runs/{SOP}/{RUN}");
    assert!(scratch.path(&record).is_file());
    assert!(scratch.path(&dir).is_dir());

    let removed = run::delete(&repo, SOP, RUN).unwrap();
    assert_eq!(removed.len(), 2, "{removed:?}");
    assert!(!scratch.path(&record).exists());
    assert!(!scratch.path(&dir).exists());
    assert!(
        scratch
            .path("runs/ground-walk-survey/2026-09-24-renfrew-walk01.md")
            .is_file(),
        "the other run in this checklist is untouched"
    );
    let (failed, report) = scratch.validate();
    assert!(!failed, "the repository must still validate:\n{report}");
}

#[test]
fn deleting_a_run_that_is_not_there_is_refused() {
    let scratch = Scratch::new("run-delete-missing");
    let repo = scratch.repo();
    assert!(matches!(
        run::delete(&repo, SOP, "no-such-run"),
        Err(run::RunError::NoRun(_, _))
    ));
}

#[test]
fn deleting_a_run_refuses_an_id_that_is_not_a_run_id() {
    let scratch = Scratch::new("run-delete-bad-id");
    let repo = scratch.repo();
    assert!(matches!(
        run::delete(&repo, SOP, "../escape"),
        Err(run::RunError::BadRunId(_))
    ));
}
