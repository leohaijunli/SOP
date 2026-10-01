//! The exported test summary: every test case, run or not, and every run's detail.

use std::fs;

use sop_core::RunEvent;
use sop_repo::run;
use sop_repo::summary::{self, SummaryMeta};

mod common;
use common::Scratch;

/// The summary takes a UTC -> local formatter and a document header; the tests keep the
/// timestamps as stored and use a fixed header so the output is deterministic.
fn ident(utc: &str) -> String {
    utc.to_owned()
}

fn meta() -> SummaryMeta {
    SummaryMeta {
        project: "Renfrew".to_owned(),
        version: "0.1.0".to_owned(),
        git_commit: Some("abc1234".to_owned()),
        git_dirty: false,
        exported: "2026-09-29 14:32".to_owned(),
    }
}

/// A run of `sop_id`, ended as `status`, with an optional run-level note.
fn make_run(repo: &sop_repo::Repo, sop: &str, run_id: &str, status: &str, note: Option<&str>) {
    run::start(repo, sop, run_id, "leo", "cfar", Some("test")).unwrap();
    if let Some(text) = note {
        run::record(
            repo,
            sop,
            run_id,
            &RunEvent::NoteAdded {
                at: String::new(),
                step: None,
                text: text.to_owned(),
            },
        )
        .unwrap();
    }
    run::end(repo, sop, run_id, status, None).unwrap();
}

/// A testcase repository with one plan and two cases: `walk.md` and `mount.md`.
fn cases_repo(cases: &Scratch) {
    let plan = cases.path("testplan/preflight");
    fs::create_dir_all(&plan).unwrap();
    fs::write(
        plan.join("plan.md"),
        "---\ntitle: Preflight\norder: 1\n---\n\n",
    )
    .unwrap();
    fs::write(
        plan.join("walk.md"),
        "---\nkind: checklist\nsop_id: ground-walk-survey\ntitle: Walk Survey\n---\n\n## Step\n\n- [ ] x\n",
    )
    .unwrap();
    fs::write(
        plan.join("mount.md"),
        "---\nkind: checklist\nsop_id: mount-check\ntitle: Mount\n---\n\n## Step\n\n- [ ] x\n",
    )
    .unwrap();
}

#[test]
fn the_summary_covers_every_test_case_and_every_run() {
    let work = Scratch::new("summary-work");
    let repo = work.repo();
    make_run(
        &repo,
        "ground-walk-survey",
        "2026-09-28-site-01",
        "partial",
        None,
    );

    let cases = Scratch::empty("summary-cases");
    cases_repo(&cases);
    let cases_repo = cases.repo();

    let text = summary::markdown(&repo, &cases_repo, &ident, &meta());

    // Header: project, export time, version, revision.
    assert!(text.contains("- project: Renfrew"), "{text}");
    assert!(text.contains("- exported: 2026-09-29 14:32"), "{text}");
    assert!(text.contains("- revision: abc1234"), "{text}");

    // Coverage: the run case reports its run and its partial outcome; the other case
    // still appears, with zeroes, because a test nobody ran is a gap worth seeing.
    // The fixtures ship a legacy run (a record file with no run directory); the summary
    // counts it too, the same way History does, so the two rows agree.
    assert!(
        text.contains("| Preflight | Walk Survey | 2 | 0 | 2 | 0 |"),
        "the run case's coverage row:\n{text}"
    );
    assert!(
        text.contains("| Preflight | Mount | 0 | 0 | 0 | 0 |"),
        "an un-run case still gets a row:\n{text}"
    );

    // Detail: the run's own row, in the plan / case it belongs to, with the file counts
    // and a folder link the whole package keeps working when it is moved.
    assert!(
        text.contains("| Preflight | Walk Survey | 2026-09-28-site-01 |"),
        "{text}"
    );
    assert!(text.contains("| cfar |"), "{text}");
    assert!(
        text.contains("runs/uvic-geomag-survey/ground-walk-survey/2026-09-28-site-01"),
        "the folder column links to the run:\n{text}"
    );

    // The Runs table has the same number of columns as its header.
    let header = text
        .lines()
        .find(|line| line.starts_with("| Test plan | Test case | Run ID |"))
        .unwrap();
    assert_eq!(
        header.matches('|').count(),
        14,
        "13 columns, 14 pipes:\n{header}"
    );
}

#[test]
fn notes_with_markdown_and_csv_metacharacters_survive_both_formats() {
    let work = Scratch::new("summary-notes");
    let repo = work.repo();
    // A note with a pipe, a newline, a comma, a quote, and Chinese: everything that can
    // break a markdown cell or a CSV field.
    make_run(
        &repo,
        "ground-walk-survey",
        "2026-09-28-site-01",
        "partial",
        Some("磁场, \"bad\" | then\nsecond line"),
    );
    let cases = Scratch::empty("summary-cases");
    cases_repo(&cases);
    let cases_repo = cases.repo();

    let md = summary::markdown(&repo, &cases_repo, &ident, &meta());
    assert!(md.contains("磁场"), "the Chinese text is kept:\n{md}");
    assert!(md.contains("\\|"), "a pipe in a cell is escaped:\n{md}");
    assert!(
        !md.contains("| then\n"),
        "a newline does not end the row:\n{md}"
    );

    let csv = summary::csv(&repo, &cases_repo, &ident, &meta());
    assert!(
        csv.starts_with('\u{feff}'),
        "the CSV starts with a UTF-8 BOM"
    );
    assert!(csv.contains("磁场"), "{csv}");
    assert!(
        csv.contains("\"\"bad\"\""),
        "quotes are doubled inside a quoted field:\n{csv}"
    );
    assert!(csv.contains("project,Renfrew"), "{csv}");
    // The Runs header is present and its column count matches the rows.
    let header = csv
        .lines()
        .find(|line| line.starts_with("Test plan,Test case,Run ID,"))
        .unwrap();
    assert_eq!(header.split(',').count(), 13, "{header}");
}

#[test]
fn a_case_no_one_ran_still_appears_in_both_tables() {
    let work = Scratch::new("summary-csv-empty");
    let repo = work.repo();
    let cases = Scratch::empty("summary-cases");
    cases_repo(&cases);
    let cases_repo = cases.repo();

    let csv = summary::csv(&repo, &cases_repo, &ident, &meta());
    assert!(
        csv.contains("Preflight,Mount,0,0,0,0,"),
        "the un-run case is a zero row:\n{csv}"
    );
}
