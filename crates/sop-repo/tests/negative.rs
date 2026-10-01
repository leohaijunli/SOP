//! Negative tests for the validator: one broken thing at a time.
//!
//! Each case copies the repository, breaks exactly one thing, and requires that
//! validation fails with a recognisable message. A clean copy must still pass, so the
//! suite cannot pass by rejecting everything.
//!
//! It replaced the transitional `tools/selftest.py`, which is now removed.

use std::fs;

use sop_repo::Repo;
use sop_repo::validate;

mod common;
use common::Scratch;

const PROC: &str = "procedures/power-on.md";
const CAL_SOP: &str = "checklists/mag-sensor-calibration.md";
const WALK_SOP: &str = "checklists/ground-walk-survey.md";
const CAL_RUN: &str = "runs/uvic-geomag-survey/mag-sensor-calibration/2026-09-22-bench-cal01.md";
const WALK_RUN: &str = "runs/uvic-geomag-survey/ground-walk-survey/2026-09-24-renfrew-walk01.md";
const TESTLINE: &str = "procedures/test-line.md";
const HELP: &str = "help/px4-operations.md";
const HELP_2: &str = "help/ubuntu-operations.md";
const PROJECT: &str = "project.md";

/// One case: a name, a way to break the repository, and the message that must appear.
type Case = (&'static str, fn(&Scratch), &'static str);

/// Give a flat run record the snapshot it would have had if it had been started with
/// `sop run start`: the checklist resolved as it is now, one step block per id.
///
/// A record is read against the revision frozen at start, so a case that wants to test
/// "this record cites a step that does not exist" has to give the record a revision of
/// its own first. Without one the check is advisory by design.
fn freeze_run_snapshot(root: &Scratch, record: &str) {
    let path = std::path::Path::new(record);
    let sop = path
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let run = path.file_stem().unwrap().to_str().unwrap();
    let ids = root
        .repo()
        .checklist_step_ids(sop)
        .unwrap_or_else(|| panic!("fixture drifted: '{sop}' does not resolve"));

    let mut text = String::from("---\nkind: snapshot\n---\n\n");
    for id in ids {
        text.push_str(&format!(
            "## {id}\n\n```yaml step\nid: {id}\nkind: check\nseverity: normal\n```\n\n"
        ));
    }
    let dir = root.path(&format!("runs/uvic-geomag-survey/{sop}/{run}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("snapshot.md"), text).unwrap();
}

fn cases() -> Vec<Case> {
    vec![
        (
            "a run with a status but no end time",
            |root| root.replace_once(WALK_RUN, "ended: 2026-09-24T20:05:00Z\n", ""),
            "'status' and 'ended' must be written together",
        ),
        (
            "missing required front matter key",
            |root| root.replace_once(PROC, "\nversion: 1\n", "\n"),
            "missing required key 'version'",
        ),
        (
            "a chapter with no yaml step block",
            |root| root.append(PROC, "\n## A chapter that is not a step\n\nJust prose.\n"),
            "has no `yaml step` block",
        ),
        (
            "duplicate step id",
            |root| root.replace_once(PROC, "id: poweron-clock", "id: poweron-supply"),
            "duplicate step id 'poweron-supply'",
        ),
        (
            "an all-digit step id written without quotes",
            |root| root.replace_once(PROC, "id: poweron-clock", "id: 7"),
            "'id' must be a string",
        ),
        (
            "invalid step id",
            |root| root.replace_once(PROC, "id: poweron-clock", "id: PowerOn-Clock"),
            "is not a valid id",
        ),
        (
            "dangling include target",
            |root| {
                root.replace_once(
                    WALK_SOP,
                    "<!-- include: procedures/walk-line.md -->",
                    "<!-- include: procedures/walk-lines.md -->",
                )
            },
            "include target does not exist",
        ),
        (
            "colliding step ids across includes",
            |root| root.replace_once(TESTLINE, "id: testline-select", "id: recon-access"),
            "collides with",
        ),
        (
            "unknown capture type",
            |root| {
                root.replace_once(
                    PROC,
                    "    type: number\n    unit: V",
                    "    type: numeric\n    unit: V",
                )
            },
            "is not one of",
        ),
        (
            "expected on a text capture",
            |root| {
                root.replace_once(
                    TESTLINE,
                    "    label: Test line identifier\n    type: text\n",
                    "    label: Test line identifier\n    type: text\n    expected: L1\n",
                )
            },
            "valid only on",
        ),
        (
            "expected not among its own options",
            |root| root.replace_once(PROC, "    expected: ok", "    expected: passed"),
            "not one of its own options",
        ),
        (
            "inverted expected range",
            |root| root.replace_once("procedures/warm-up.md", "min: -1.0", "min: 5.0"),
            "greater than 'expected.max'",
        ),
        (
            "pre-checked item in a template",
            |root| root.append(PROC, "\n- [x] this should not be pre-checked\n"),
            "pre-checked item",
        ),
        (
            "schema from the future",
            |root| root.replace_once(PROC, "kind: procedure", "kind: procedure\nschema: 99"),
            "supports at most",
        ),
        (
            "attachment hash mismatch",
            |root| {
                let text = root.read(CAL_RUN);
                let old = text
                    .split('\n')
                    .find(|line| line.trim_start().starts_with("sha256:"))
                    .expect("fixture drifted: no sha256 line")
                    .to_owned();
                root.write(
                    CAL_RUN,
                    &text.replacen(&old, &format!("    sha256: {}", "0".repeat(64)), 1),
                );
            },
            "does not match the file",
        ),
        (
            "deviations_count disagrees",
            |root| root.replace_once(CAL_RUN, "deviations_count: 1", "deviations_count: 0"),
            "result(s) are deviated",
        ),
        (
            "deviated without a reason",
            |root| {
                root.replace_once(
                    CAL_RUN,
                    "reason: Spread of 12.0 nT",
                    "note: Spread of 12.0 nT",
                )
            },
            "has no 'reason'",
        ),
        (
            "result cites a step outside its snapshot",
            |root| {
                freeze_run_snapshot(root, CAL_RUN);
                root.replace_once(CAL_RUN, "step: noise-verdict", "step: noise-summary");
            },
            "not in checklist",
        ),
        (
            "complete run with a skipped step",
            |root| root.replace_once(WALK_RUN, "status: partial", "status: complete"),
            "was skipped",
        ),
        (
            "complete run missing a result",
            |root| {
                freeze_run_snapshot(root, CAL_RUN);
                let text = root.read(CAL_RUN);
                let block = "## hygiene-person\n\n```yaml result\nstep: hygiene-person\nstatus: done\nopened_at: 2026-09-24T16:53:00Z\nended_at: 2026-09-24T16:56:00Z\n```\n\n";
                assert!(
                    text.contains(block),
                    "fixture drifted: hygiene-person result block"
                );
                root.write(CAL_RUN, &text.replacen(block, "", 1));
            },
            "has no result",
        ),
        (
            "missing attached log",
            |root| {
                fs::remove_file(root.path("logs/2026-09-22-bench-cal01/heading_sweep.csv")).unwrap()
            },
            "attached log does not exist",
        ),
        (
            "orphan log directory",
            |root| fs::create_dir_all(root.path("logs/2026-09-30-nowhere-walk01")).unwrap(),
            "has no matching run record",
        ),
        (
            "unresolved relative link",
            |root| root.append(PROC, "\nSee [the missing doc](../docs/nope.md).\n"),
            "link target does not resolve",
        ),
        (
            "CRLF line endings",
            |root| {
                let text = root.read(PROC).replace('\n', "\r\n");
                root.write(PROC, &text);
            },
            "CR characters",
        ),
        (
            "help page missing a section",
            |root| root.replace_once(HELP, "section: PX4 Operations\n", ""),
            "missing required key 'section'",
        ),
        (
            "help page with an unknown audience",
            |root| root.replace_once(HELP, "audience: operator", "audience: wizard"),
            "must be one of",
        ),
        (
            "help page id that does not match its filename",
            |root| root.replace_once(HELP, "help_id: px4-operations", "help_id: px4-Ops"),
            "is not a valid id",
        ),
        (
            "help page with a dangling link",
            |root| root.append(HELP, "\nSee [the missing page](nope.md).\n"),
            "link target does not resolve",
        ),
        (
            "project file whose kind is not 'project'",
            |root| root.replace_once(PROJECT, "kind: project", "kind: procedure"),
            "must be 'project'",
        ),
        (
            "project file with a malformed project id",
            |root| {
                root.replace_once(
                    PROJECT,
                    "project_id: uvic-geomag-survey",
                    "project_id: Uvic Survey",
                )
            },
            "is not a valid id",
        ),
        (
            "project file with a date that is not a date",
            |root| root.replace_once(PROJECT, "updated: 2026-09-25", "updated: yesterday"),
            "must be a YYYY-MM-DD date",
        ),
    ]
}

/// Cases that must warn without failing the run.
type Warning = (&'static str, fn(&Scratch), &'static str);

fn warning_cases() -> Vec<Warning> {
    vec![
        (
            "two help pages claiming the same panel position",
            |root| root.replace_once(HELP_2, "order: 20", "order: 10"),
            "same 'order'",
        ),
        (
            "checklist with no packing list",
            |root| root.clear_front_matter_block(CAL_SOP, "equipment"),
            "'equipment' is empty",
        ),
        (
            "unknown front matter key",
            |root| root.replace_once(PROC, "kind: procedure", "kind: procedure\nfuture_key: 1"),
            "unknown front matter key 'future_key'",
        ),
    ]
}

#[test]
fn a_clean_copy_validates() {
    let scratch = Scratch::new("clean");
    let (failed, report) = scratch.validate();
    assert!(!failed, "a clean copy must validate:\n{report}");
    assert!(
        report.ends_with("0 error(s), 0 warning(s)\n"),
        "a clean copy must have no findings at all:\n{report}"
    );
    drop(scratch);
    let _ = CAL_SOP;
}

#[test]
fn every_negative_case_fails_for_the_right_reason() {
    let mut failures = Vec::new();
    for (name, mutate, expected) in cases() {
        let scratch = Scratch::new(name);
        // Opening the repo upgrades a flat `runs/<sop>/` layout to the project-nested one,
        // which is where the fixture paths below expect the records to be.
        scratch.repo();
        mutate(&scratch);
        let (failed, report) = scratch.validate();
        if !failed {
            failures.push(format!("{name}: validator accepted the broken content"));
        } else if !report.contains(expected) {
            failures.push(format!(
                "{name}: failed for the wrong reason, no {expected:?} in:\n{report}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases failed:\n{}",
        failures.len(),
        cases().len(),
        failures.join("\n")
    );
}

#[test]
fn every_warning_case_warns_for_the_right_reason() {
    let mut failures = Vec::new();
    for (name, mutate, expected) in warning_cases() {
        let scratch = Scratch::new(name);
        mutate(&scratch);
        let (failed, report) = scratch.validate();
        if failed {
            failures.push(format!(
                "{name}: this is a warning, not an error:\n{report}"
            ));
        } else if !report.contains(expected) {
            failures.push(format!("{name}: no {expected:?} in:\n{report}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} warning cases failed:\n{}",
        failures.len(),
        warning_cases().len(),
        failures.join("\n")
    );
}

#[test]
fn a_repository_root_with_no_content_is_reported_as_such() {
    let empty = Scratch::new("empty");
    for entry in ["procedures", "checklists", "runs", "logs", "help", PROJECT] {
        let path = empty.path(entry);
        if path.is_dir() {
            fs::remove_dir_all(path).unwrap();
        } else {
            fs::remove_file(path).unwrap();
        }
    }
    let repo = Repo::open(&empty.root);
    let outcome = validate::validate_repository(&repo);
    assert_eq!(outcome.files_checked, 0);
    assert!(!outcome.report.has_errors());
}
