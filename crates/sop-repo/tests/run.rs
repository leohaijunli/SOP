//! A run's life: start it, record events, recover it, end it.
//!
//! These exercise `sop-repo::run` against a scratch copy of the real repository, and
//! check the promises the format makes: a snapshot is frozen at start, a crash is
//! recovered by replaying the log, the rendered record agrees with the events, and a
//! skipped/deviated step cannot be recorded without a reason.

use std::fs;

use sop_core::RunEvent;
use sop_repo::{Repo, manifest, run};

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
fn a_run_that_has_not_ended_is_not_a_validation_error() {
    let scratch = Scratch::new("run-in-progress");
    let repo = scratch.repo();
    start(&repo, RUN);

    // The record is written at start and rewritten as the run goes, so the file the
    // operator's own header validates is an unfinished one. That must not read as a
    // broken record; `status`/`ended` only appear once the run ends.
    let (failed, report) = scratch.validate();
    assert!(!failed, "an in-progress run must not be an error:\n{report}");
    assert!(
        report.ends_with("0 error(s), 0 warning(s)\n"),
        "a run that has only started is not a finding yet:\n{report}"
    );
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
        plan: Some("uas-mag-calibration".to_owned()),
        case: Some("heading-error".to_owned()),
    };
    run::start_with_meta(&repo, SOP, "2026-09-25-meta", "leo", "Renfrew 395", Some("test"), &meta)
        .unwrap();
    run::end(&repo, SOP, "2026-09-25-meta", "partial").unwrap();

    let record = scratch.read("runs/ground-walk-survey/2026-09-25-meta.md");
    assert!(record.contains("sensor:\n  model: GEM GSM-19\n  serial: \"4451233\"\n  firmware: \"7.0\""), "{record}");
    assert!(record.contains("hardware:\n  - mag_gcs v0.3.1\n  - tripod"), "{record}");
    assert!(record.contains("conditions:\n  temp_c: \"12\"\n  weather: clear"), "{record}");
    assert!(record.contains("plan: uas-mag-calibration\ncase: heading-error"), "{record}");
    // The head restates the instrument for a reader who only opens the record.
    assert!(record.contains("sensor: GEM GSM-19, serial 4451233, firmware 7.0"), "{record}");
    // The manifest carries the same provenance, so the coverage board can group a run
    // under its plan and case without opening the record.
    let manifest = manifest::build(&repo);
    let entry = manifest
        .runs
        .iter()
        .find(|entry| entry.run_id.as_ref().and_then(|id| id.as_str()) == Some("2026-09-25-meta"))
        .expect("the started run is in the manifest");
    assert_eq!(entry.plan.as_ref().and_then(|p| p.as_str()), Some("uas-mag-calibration"));
    assert_eq!(entry.case.as_ref().and_then(|c| c.as_str()), Some("heading-error"));
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
fn a_log_truncated_anywhere_replays_as_a_prefix() {
    // The append-only promise: an interrupted write can only ever cut the final line, so
    // replay from any byte length must yield the events written before the cut - never a
    // half-applied event and never a hard failure.
    let scratch = Scratch::new("run-truncate-prefix");
    let repo = scratch.repo();
    let run_id = "2026-09-25-truncate";
    start(&repo, run_id);
    for (key, value) in [("a", "1"), ("b", "2")] {
        run::record(
            &repo,
            SOP,
            run_id,
            &RunEvent::CaptureRecorded {
                at: "t".into(),
                step: "cond-location".into(),
                key: format!("cap_{key}"),
                value: value.into(),
                unit: None,
            },
        )
        .unwrap();
    }

    let rel = "runs/ground-walk-survey/2026-09-25-truncate/events.jsonl";
    let full = scratch.read(rel);
    // Cut inside the last event's JSON, leaving a partial line with no newline.
    let cut = full.trim_end().len() - 5;
    scratch.write(rel, &full[..cut]);

    let loaded = run::load(&repo, SOP, run_id).unwrap();
    let captures = &loaded.state.step("cond-location").unwrap().captures;
    assert_eq!(
        captures["cap_a"].value, "1",
        "an event written before the cut survives"
    );
    assert!(
        !captures.contains_key("cap_b"),
        "the event the cut landed inside is dropped whole, never half-applied"
    );
}

#[test]
fn concurrent_writers_do_not_lose_events() {
    // The app and the CLI can both be pointed at one run. The per-run lock plus O_APPEND
    // has to make every event land exactly once, with no interleaved line.
    let scratch = Scratch::new("run-concurrent-append");
    let repo = scratch.repo();
    let run_id = "2026-09-25-concurrent";
    start(&repo, run_id);

    const WRITERS: usize = 8;
    std::thread::scope(|scope| {
        for writer in 0..WRITERS {
            let repo = &repo;
            scope.spawn(move || {
                run::record(
                    repo,
                    SOP,
                    run_id,
                    &RunEvent::CaptureRecorded {
                        at: "t".into(),
                        step: "cond-location".into(),
                        key: format!("cap_{writer}"),
                        value: writer.to_string(),
                        unit: None,
                    },
                )
                .unwrap();
            });
        }
    });

    let rel = "runs/ground-walk-survey/2026-09-25-concurrent/events.jsonl";
    let text = scratch.read(rel);
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        lines.len(),
        WRITERS + 2,
        "RunStarted + the draft-override note + one event per writer, no torn or dropped line:\n{text}"
    );
    for line in &lines {
        serde_json::from_str::<RunEvent>(line).expect("every line is one whole event");
    }

    let loaded = run::load(&repo, SOP, run_id).unwrap();
    let captures = &loaded.state.step("cond-location").unwrap().captures;
    for writer in 0..WRITERS {
        assert_eq!(captures[&format!("cap_{writer}")].value, writer.to_string());
    }
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

#[test]
fn a_run_started_from_a_file_path_is_filed_under_the_checklists_sop_id() {
    let scratch = Scratch::new("run-external-file");
    let repo = scratch.repo();

    // How the app runs an "Open file" checklist: `start` is handed the path, and the
    // file stem ("power-on") is not the checklist's id. The run directory, the committed
    // record, and the manifest's grouping all have to use the id, or History shows the
    // same run twice - once under the stem and once under the id.
    let external = scratch.path("external/power-on.md");
    fs::create_dir_all(external.parent().unwrap()).unwrap();
    fs::write(
        &external,
        "---\nkind: checklist\nsop_id: uas-mag-preflight-power-on\ntitle: Power On\nversion: 1\nstatus: active\n---\n\n## Supply\n\n- [ ] Recorded\n",
    )
    .unwrap();

    let run_id = "2026-09-28-site-01";
    run::start(&repo, external.to_str().unwrap(), run_id, "leo", "cfar", None).unwrap();
    run::end(&repo, "uas-mag-preflight-power-on", run_id, "complete").unwrap();

    assert!(scratch.path("runs/uas-mag-preflight-power-on").is_dir());
    assert!(
        scratch.path("runs/uas-mag-preflight-power-on/2026-09-28-site-01.md").is_file(),
        "the committed record is filed under the checklist id"
    );
    assert!(
        !scratch.path("runs/power-on").exists(),
        "the in-progress record must not be filed under the file stem"
    );

    let manifest = manifest::build(&repo);
    let matching: Vec<_> = manifest
        .runs
        .iter()
        .filter(|entry| {
            entry.run_id.as_ref().and_then(|id| id.as_str()) == Some(run_id)
        })
        .collect();
    assert_eq!(matching.len(), 1, "History must list the run once, not twice");
}

#[test]
fn an_attachment_is_saved_under_its_own_name_and_listed_in_the_record() {
    let scratch = Scratch::new("run-attach");
    let repo = scratch.repo();
    start(&repo, RUN);

    // `incoming/` is not one of the directories discovery or validation reads, so it
    // stands in for a file picked from anywhere on the operator's machine.
    let source = scratch.path("incoming/mag_raw.csv");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, b"a,b\n1,2\n").unwrap();

    let attachment = run::attach(&repo, SOP, RUN, Some("cond-location"), &source).unwrap();
    assert_eq!(
        attachment.path,
        format!("runs/{SOP}/{RUN}/logs/mag_raw.csv"),
        "the record names the file relative to the repository, next to the run"
    );
    assert!(scratch.path(&attachment.path).is_file());

    // A different file with the same name must not overwrite the first.
    fs::write(&source, b"other\n").unwrap();
    let second = run::attach(&repo, SOP, RUN, Some("cond-location"), &source).unwrap();
    assert_eq!(second.path, format!("runs/{SOP}/{RUN}/logs/mag_raw-2.csv"));
    assert_eq!(scratch.read(&attachment.path), "a,b\n1,2\n");

    // Picking the same file again is the same attachment, not a second copy.
    fs::write(&source, b"a,b\n1,2\n").unwrap();
    let again = run::attach(&repo, SOP, RUN, Some("cond-location"), &source).unwrap();
    assert_eq!(again.path, attachment.path);
    let held = run::load(&repo, SOP, RUN).unwrap().state.step("cond-location").unwrap().attachments.len();
    assert_eq!(held, 2, "two files were attached, so two are recorded");

    run::end(&repo, SOP, RUN, "partial").unwrap();
    let record = scratch.read(&format!("runs/{SOP}/{RUN}.md"));
    assert!(record.contains("logs:\n"), "the data files are listed in the record:\n{record}");
    assert!(record.contains(&format!("path: {}\n", attachment.path)), "{record}");
    assert!(record.contains(&attachment.sha256), "{record}");

    // The listed path resolves, and the size and hash match, so the record verifies.
    let (failed, report) = scratch.validate();
    assert!(!failed, "a run carrying its data must validate:\n{report}");
    assert!(
        !report.contains("not under"),
        "the run directory's logs/ is the app's own location:\n{report}"
    );
}

#[test]
fn deleting_every_run_removes_the_history_and_leaves_the_inbox_alone() {
    let scratch = Scratch::new("run-delete-all");
    let repo = scratch.repo();
    start(&repo, RUN);
    run::end(&repo, SOP, RUN, "partial").unwrap();

    let found = run::all_runs(&repo);
    assert!(
        found.iter().any(|(sop, id)| sop == SOP && id == RUN),
        "the new run is listed: {found:?}"
    );
    assert!(
        found.iter().any(|(sop, _)| sop == "mag-sensor-calibration"),
        "the fixture run is listed: {found:?}"
    );

    let removed = run::delete_all(&repo).unwrap();
    assert!(!removed.is_empty());
    assert!(run::all_runs(&repo).is_empty(), "every run record is gone");
    assert!(!scratch.path(&format!("runs/{SOP}/{RUN}.md")).exists());
    assert!(!scratch.path(&format!("runs/{SOP}/{RUN}")).exists());
    assert!(
        scratch.path("runs/_inbox/2026-09-24-wind-oscillation.md").is_file(),
        "an observation in the inbox is not run history"
    );
}

#[test]
fn acknowledging_an_out_of_range_capture_is_recorded() {
    let scratch = Scratch::new("run-acknowledge");
    let repo = scratch.repo();
    start(&repo, "2026-09-25-acknowledge");
    run::record(
        &repo,
        SOP,
        "2026-09-25-acknowledge",
        &RunEvent::CaptureRecorded {
            at: "t1".into(),
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
        "2026-09-25-acknowledge",
        &RunEvent::CaptureAcknowledged {
            at: "t2".into(),
            step: "cond-location".into(),
            key: "session_location".into(),
            reason: Some("known offset".into()),
        },
    )
    .unwrap();

    let loaded = run::load(&repo, SOP, "2026-09-25-acknowledge").unwrap();
    assert!(
        loaded.state.steps["cond-location"]
            .acknowledged
            .contains("session_location"),
        "the acknowledgement replay reaches the state"
    );
    run::end(&repo, SOP, "2026-09-25-acknowledge", "partial").unwrap();
    let record = scratch.read("runs/ground-walk-survey/2026-09-25-acknowledge.md");
    assert!(
        record.contains("acknowledged:\n  - session_location"),
        "{record}"
    );
}

#[test]
fn drift_reports_whether_the_checklist_changed_since_the_snapshot() {
    let scratch = Scratch::new("run-drift");
    let repo = scratch.repo();
    start(&repo, "2026-09-25-drift");

    let before = run::drift(&repo, SOP, "2026-09-25-drift").unwrap();
    assert!(before.current_sha256.is_some(), "the checklist resolves today");
    assert!(!before.drifted, "nothing changed yet");

    // Editing the checklist body makes it resolve to a different snapshot.
    scratch.append(
        "checklists/ground-walk-survey.md",
        "\n\nAn extra paragraph the run did not freeze.\n",
    );
    let after = run::drift(&repo, SOP, "2026-09-25-drift").unwrap();
    assert!(after.drifted, "a changed checklist is reported as drifted");
}

#[test]
fn a_run_from_a_testplan_case_resolves_its_checklist_for_validation() {
    // The app starts cases from `testplan/` by path, and the run records the case's
    // `sop_id`. The validator has to find that id again - in `testplan/`, not only in
    // `checklists/` - or every test-case run is reported as an error.
    let scratch = Scratch::new("run-testplan-validate");
    let repo = scratch.repo();
    let case = scratch.path("testplan/preflight/power-on.md");
    fs::create_dir_all(case.parent().unwrap()).unwrap();
    fs::write(
        &case,
        "---\nkind: checklist\nsop_id: uas-mag-preflight-power-on\ntitle: Power On\nversion: 1\nstatus: active\n---\n\n## Supply\n\n- [ ] Recorded\n",
    )
    .unwrap();

    let run_id = "2026-09-28-site-01";
    run::start(&repo, case.to_str().unwrap(), run_id, "leo", "cfar", None).unwrap();
    run::end(&repo, "uas-mag-preflight-power-on", run_id, "partial").unwrap();

    let (_, report) = scratch.validate();
    assert!(
        !report.contains("does not name an existing checklist"),
        "a testplan case resolves for validation:\n{report}"
    );
}
