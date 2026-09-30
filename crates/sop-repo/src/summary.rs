//! A summary of every test, for exporting to a file.
//!
//! Two tables: coverage of every test case (run or not), then one row per run with the
//! notes, instrument, site, time, and attached file names. The runs come from the working
//! copy; the plan and case names come from the testcase repository, which is where
//! `testplan/` lives when it is kept separate.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use sop_core::run::RunState;

use crate::{Repo, run, testplan};

/// Every test, as a markdown document.
pub fn markdown(repo: &Repo, testcases: &Repo) -> String {
    let mut runs = runs_by_checklist(repo);
    for rows in runs.values_mut() {
        rows.sort_by(|a, b| a.time.cmp(&b.time));
    }
    let plans = testplan::plans(testcases);

    // Coverage first: every test case gets a line, run or not, so a case nobody has
    // executed shows up as a gap instead of being absent from the summary.
    let mut out = String::from("# Test summary\n\n## Test cases\n\n");
    out.push_str("| Test plan | Test case | Runs | Complete | Partial | Aborted | Last run |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    let mut written: BTreeSet<String> = BTreeSet::new();
    for plan in &plans {
        let plan_name = plan.title.clone().unwrap_or_else(|| plan.id.clone());
        for case in &plan.cases {
            let case_name = case.title.clone().unwrap_or_else(|| case.id.clone());
            coverage_line(&mut out, &plan_name, &case_name, rows_for(&runs, &case.sop_id));
            written.insert(case.sop_id.clone());
        }
    }
    // Runs that match no case - a checklist opened by path, or content this testcase
    // repository does not describe - still get a line rather than disappearing.
    for (sop, rows) in &runs {
        if written.contains(sop) {
            continue;
        }
        coverage_line(&mut out, "", sop, Some(rows.as_slice()));
    }

    // Then the detail: one row per run, in plan / case order.
    out.push_str("\n## Runs\n\n");
    out.push_str("| Test plan | Test case | Notes | Sensor | Site | Time | Log file |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for plan in &plans {
        let plan_name = plan.title.clone().unwrap_or_else(|| plan.id.clone());
        for case in &plan.cases {
            let Some(rows) = runs.get(&case.sop_id) else { continue };
            let case_name = case.title.clone().unwrap_or_else(|| case.id.clone());
            for row in rows {
                detail_line(&mut out, &plan_name, &case_name, row);
            }
        }
    }
    for (sop, rows) in &runs {
        if written.contains(sop) {
            continue;
        }
        for row in rows {
            detail_line(&mut out, "", sop, row);
        }
    }
    out
}

/// Every run in the working copy, keyed by the checklist id it recorded.
fn runs_by_checklist(repo: &Repo) -> BTreeMap<String, Vec<Row>> {
    let mut out: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    for (sop, run_id) in run::all_runs(repo) {
        let Ok(loaded) = run::load(repo, &sop, &run_id) else { continue };
        out.entry(sop).or_default().push(Row::of(&loaded.state));
    }
    out
}

fn rows_for<'a>(runs: &'a BTreeMap<String, Vec<Row>>, sop: &str) -> Option<&'a [Row]> {
    runs.get(sop).map(Vec::as_slice)
}

/// One coverage row: how many runs a test case has, how they ended, and when it last ran.
/// A case with no runs still gets the row, with zeroes.
fn coverage_line(out: &mut String, plan: &str, case: &str, rows: Option<&[Row]>) {
    let rows = rows.unwrap_or(&[]);
    let ended =
        |status: &str| rows.iter().filter(|row| row.status.as_deref() == Some(status)).count();
    let last = rows.last().map(|row| row.time.as_str()).unwrap_or("");
    let _ = writeln!(
        out,
        "| {} | {} | {} | {} | {} | {} | {} |",
        esc(plan),
        esc(case),
        rows.len(),
        ended("complete"),
        ended("partial"),
        ended("aborted"),
        esc(last)
    );
}

/// One run's row in the detail table.
fn detail_line(out: &mut String, plan: &str, case: &str, row: &Row) {
    let _ = writeln!(
        out,
        "| {} | {} | {} | {} | {} | {} | {} |",
        esc(plan),
        esc(case),
        esc(&row.notes),
        esc(&row.sensor),
        esc(&row.site),
        esc(&row.time),
        esc(&row.logs)
    );
}

/// The parts of one run the summary prints.
struct Row {
    /// The run's outcome, absent while it is still in progress.
    status: Option<String>,
    notes: String,
    sensor: String,
    site: String,
    time: String,
    logs: String,
}

impl Row {
    fn of(state: &RunState) -> Self {
        let mut notes: Vec<String> = state.run_notes.iter().map(|note| note.text.clone()).collect();
        for step in state.steps.values() {
            notes.extend(step.notes.iter().map(|note| note.text.clone()));
        }
        let sensor = state
            .sensor
            .as_ref()
            .map(|sensor| sensor.model.as_str().to_owned())
            .unwrap_or_default();
        let logs = state
            .steps
            .values()
            .flat_map(|step| step.attachments.iter())
            .chain(state.run_attachments.iter())
            .filter_map(|attachment| {
                std::path::Path::new(&attachment.path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect::<Vec<_>>()
            .join(", ");
        Self {
            status: state.run_status.clone(),
            notes: notes.join("; "),
            sensor,
            site: state.site.clone().unwrap_or_default(),
            time: state.started.clone().unwrap_or_default(),
            logs,
        }
    }
}

fn esc(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}
