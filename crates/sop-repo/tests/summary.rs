//! The exported test summary: every test case, run or not, and every run's detail.

use std::fs;

use sop_repo::{run, summary};

mod common;
use common::Scratch;

#[test]
fn the_summary_covers_every_test_case_and_every_run() {
    let work = Scratch::new("summary-work");
    let repo = work.repo();
    // A run whose checklist id a test case claims.
    run::start(&repo, "ground-walk-survey", "2026-09-28-site-01", "leo", "cfar", Some("test"))
        .unwrap();
    run::end(&repo, "ground-walk-survey", "2026-09-28-site-01", "partial").unwrap();

    // The testcase repository: one plan, one case that has been run and one that has not.
    let cases = Scratch::empty("summary-cases");
    let cases_repo = cases.repo();
    let plan = cases.path("testplan/preflight");
    fs::create_dir_all(&plan).unwrap();
    fs::write(plan.join("plan.md"), "---\ntitle: Preflight\norder: 1\n---\n\n").unwrap();
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

    let text = summary::markdown(&repo, &cases_repo);

    // Coverage: the run case reports its run and its partial outcome; the other case
    // still appears, with zeroes, because a test nobody ran is a gap worth seeing.
    assert!(
        text.contains("| Preflight | Walk Survey | 1 | 0 | 1 | 0 |"),
        "the run case's coverage row:\n{text}"
    );
    assert!(
        text.contains("| Preflight | Mount | 0 | 0 | 0 | 0 |"),
        "an un-run case still gets a row:\n{text}"
    );

    // Detail: the run's own row, in the plan / case it belongs to.
    assert!(
        text.contains("| Preflight | Walk Survey |")
            && text.contains("| cfar |"),
        "the run's detail row:\n{text}"
    );
}
