//! Test plans: one folder per plan, one markdown case per file inside it.

use std::fs;

use sop_repo::testplan;

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
