//! Test plans: one folder per plan, one markdown case per file inside it.

use std::fs;

use sop_repo::{manifest, run, testplan};

mod common;
use common::Scratch;

#[test]
fn a_case_carries_the_checklist_id_a_run_of_it_records() {
    let scratch = Scratch::new("testplan-sop-id");
    let repo = scratch.repo();
    let plan = scratch.path("testplan/preflight");
    fs::create_dir_all(&plan).unwrap();
    fs::write(
        plan.join("plan.md"),
        "---\ntitle: Preflight\norder: 1\n---\n\n",
    )
    .unwrap();
    // A declared id that differs from the file stem: runs record the declared one, so
    // this is what the History view matches runs against.
    fs::write(
        plan.join("power-on.md"),
        "---\nkind: checklist\nsop_id: uas-mag-preflight-power-on\ntitle: Power On\n---\n\n## Supply\n\n- [ ] ok\n",
    )
    .unwrap();
    // No `sop_id`: the file stem is the checklist id, the same rule the app runs by.
    fs::write(
        plan.join("mount.md"),
        "---\nkind: checklist\ntitle: Mount\n---\n\n## Mount\n\n- [ ] ok\n",
    )
    .unwrap();

    let plans = testplan::plans(&repo);
    let plan = plans
        .iter()
        .find(|plan| plan.id == "preflight")
        .expect("the plan is discovered");
    let cases: Vec<(&str, &str)> = plan
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case.sop_id.as_str()))
        .collect();
    assert!(
        cases.contains(&("power-on", "uas-mag-preflight-power-on")),
        "{cases:?}"
    );
    assert!(cases.contains(&("mount", "mount")), "{cases:?}");
}

#[test]
fn plain_markdown_plans_are_auto_loaded_and_their_cases_can_record_logs() {
    let scratch = Scratch::empty("testplan-plain-md");
    let folder = scratch.path("testplan/quick");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("plan.md"), "# Field test plan\n").unwrap();
    fs::write(folder.join("quick-check.md"), "# Quick check\n\n## Preparation\n\n- [ ] Connect the instrument\n\n## Measurement\n\nCollect a short log.\n").unwrap();
    let repo = scratch.repo();
    let plans = testplan::plans(&repo);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].cases[0].sop_id, "quick-check");
    assert_eq!(plans[0].cases[0].step_count, 2);
    assert_eq!(
        manifest::build(&repo).testplans.len(),
        1,
        "opening the workspace automatically indexes local MD plans"
    );
    let loaded = run::start(
        &repo,
        "quick-check",
        "2026-11-24-quick",
        "operator",
        "lab",
        None,
    )
    .unwrap();
    assert_eq!(loaded.defs.len(), 2);
    let source = scratch.path("measurement.log");
    fs::write(&source, "measured data").unwrap();
    run::attach(&repo, "quick-check", "2026-11-24-quick", None, &source).unwrap();
    assert_eq!(
        run::files(&repo, "quick-check", "2026-11-24-quick")
            .unwrap()
            .log_count,
        1
    );
    let (failed, report) = scratch.validate();
    assert!(!failed, "{report}");
}
