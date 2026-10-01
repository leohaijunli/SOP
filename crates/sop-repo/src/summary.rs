//! A summary of every test, for exporting to a file.
//!
//! Two tables: coverage of every test case (run or not), then one row per run. The run
//! rows come from the manifest's `RunEntry` - the same data the History view reads, so
//! the exported table and the on-screen table agree - with the file counts and notes
//! added. The plan and case names come from the testcase repository, which is where
//! `testplan/` lives when it is kept separate.
//!
//! The local time is not computed here: `sop-repo` has no clock. The caller passes a
//! `fmt_time` that turns a stored UTC timestamp into local text, so the crate stays free
//! of `chrono` (only `sop-app` depends on it).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::Value as Json;

use crate::{Repo, manifest, run, testplan};

/// The document header: which project this is, when it was exported, and the revision.
#[derive(Debug, Clone, Default)]
pub struct SummaryMeta {
    pub project: String,
    /// The application version.
    pub version: String,
    /// The working copy's short git hash, when it is a repository.
    pub git_commit: Option<String>,
    /// True when the working copy has uncommitted changes.
    pub git_dirty: bool,
    /// The export time, already formatted in the operator's local time.
    pub exported: String,
}

/// A stored UTC timestamp as local text. Injected so `sop-repo` needs no time zone.
pub type FmtTime<'a> = &'a dyn Fn(&str) -> String;

/// Every test, as a markdown document.
pub fn markdown(
    repo: &Repo,
    testcases: &Repo,
    fmt_time: FmtTime<'_>,
    meta: &SummaryMeta,
) -> String {
    let gathered = gather(repo, testcases, fmt_time);
    let mut out = String::from("# Test summary\n\n");
    out.push_str(&format!("- project: {}\n", meta.project));
    out.push_str(&format!("- exported: {}\n", meta.exported));
    out.push_str(&format!("- version: {}\n", meta.version));
    match &meta.git_commit {
        Some(commit) => out.push_str(&format!(
            "- revision: {commit}{}\n",
            if meta.git_dirty {
                " (uncommitted changes)"
            } else {
                ""
            }
        )),
        None => out.push_str("- revision: unknown\n"),
    }

    out.push_str("\n## Test cases\n\n");
    out.push_str("| Test plan | Test case | Runs | Complete | Partial | Aborted | Last run |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for line in &gathered.coverage {
        out.push_str(line);
        out.push('\n');
    }

    out.push_str("\n## Runs\n\n");
    out.push_str(
        "| Test plan | Test case | Run ID | Started | Site | Operator | Sensor | Outcome | \
         Conclusion | Deviations | Files (L/P/F) | Notes | Folder |\n",
    );
    out.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for row in &gathered.rows {
        out.push_str(&row.markdown(fmt_time));
        out.push('\n');
    }
    out
}

/// The same summary as CSV, so a spreadsheet can open it. A UTF-8 BOM is written first so
/// Excel reads the Chinese and the `|`-free text as UTF-8 rather than in the system code
/// page.
pub fn csv(repo: &Repo, testcases: &Repo, fmt_time: FmtTime<'_>, meta: &SummaryMeta) -> String {
    let gathered = gather(repo, testcases, fmt_time);
    let mut out = String::from("\u{feff}");
    // The same header the markdown carries, as `key,value` rows before the tables.
    let _ = writeln!(out, "project,{}", csv_field(&meta.project));
    let _ = writeln!(out, "exported,{}", csv_field(&meta.exported));
    let _ = writeln!(out, "version,{}", csv_field(&meta.version));
    let _ = writeln!(out, "revision,{}", csv_field(&revision(meta)));
    out.push('\n');
    let _ = writeln!(
        out,
        "Test plan,Test case,Runs,Complete,Partial,Aborted,Last run"
    );
    for line in &gathered.coverage_csv {
        out.push_str(line);
        out.push('\n');
    }
    let _ = writeln!(
        out,
        "Test plan,Test case,Run ID,Started,Site,Operator,Sensor,Outcome,Conclusion,Deviations,\
         Files (L/P/F),Notes,Folder"
    );
    for row in &gathered.rows {
        out.push_str(&row.csv(fmt_time));
        out.push('\n');
    }
    out
}

/// Everything the two renderers need: the coverage lines (markdown and CSV) and the run
/// rows. Gathered once so `markdown` and `csv` cannot disagree.
struct Gathered {
    coverage: Vec<String>,
    coverage_csv: Vec<String>,
    rows: Vec<RunRow>,
}

fn gather(repo: &Repo, testcases: &Repo, fmt_time: FmtTime<'_>) -> Gathered {
    let plans = testplan::plans(testcases);
    let runs = manifest::build(repo).runs;

    // Runs keyed by the checklist id they recorded, newest last, so a case can report
    // its own runs and the last time it ran.
    let mut by_sop: BTreeMap<String, Vec<&manifest::RunEntry>> = BTreeMap::new();
    for entry in &runs {
        by_sop.entry(str_of(&entry.sop)).or_default().push(entry);
    }
    for rows in by_sop.values_mut() {
        rows.sort_by_key(|entry| str_of(&entry.started));
    }

    let mut coverage = Vec::new();
    let mut coverage_csv = Vec::new();
    let mut rows = Vec::new();
    let mut written: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for plan in &plans {
        let plan_name = plan.title.clone().unwrap_or_else(|| plan.id.clone());
        for case in &plan.cases {
            let case_name = case.title.clone().unwrap_or_else(|| case.id.clone());
            let case_runs = by_sop.get(&case.sop_id).map(Vec::as_slice).unwrap_or(&[]);
            coverage.push(coverage_line(&plan_name, &case_name, case_runs));
            coverage_csv.push(coverage_line_csv(&plan_name, &case_name, case_runs));
            for entry in case_runs {
                rows.push(RunRow::of(repo, &plan_name, &case_name, entry));
            }
            written.insert(case.sop_id.clone());
        }
    }
    // Runs that match no case - a checklist opened by path, or content this testcase
    // repository does not describe - still get a row rather than disappearing.
    for (sop, case_runs) in &by_sop {
        if written.contains(sop) {
            continue;
        }
        coverage.push(coverage_line("", sop, case_runs));
        coverage_csv.push(coverage_line_csv("", sop, case_runs));
        for entry in case_runs {
            rows.push(RunRow::of(repo, "", sop, entry));
        }
    }
    let _ = fmt_time;
    Gathered {
        coverage,
        coverage_csv,
        rows,
    }
}

/// The revision text for the header: the short hash, marked when the copy is dirty.
fn revision(meta: &SummaryMeta) -> String {
    match &meta.git_commit {
        Some(commit) if meta.git_dirty => format!("{commit} (uncommitted changes)"),
        Some(commit) => commit.clone(),
        None => "unknown".to_owned(),
    }
}

/// One coverage row: how many runs a test case has, how they ended, and when it last ran.
/// A case with no runs still gets the row, with zeroes.
fn coverage_line(plan: &str, case: &str, runs: &[&manifest::RunEntry]) -> String {
    let ended = |status: &str| {
        runs.iter()
            .filter(|entry| str_of(&entry.status) == status)
            .count()
    };
    let last = runs
        .last()
        .map(|entry| str_of(&entry.started))
        .unwrap_or_default();
    format!(
        "| {} | {} | {} | {} | {} | {} | {} |",
        esc(plan),
        esc(case),
        runs.len(),
        ended("complete"),
        ended("partial"),
        ended("aborted"),
        esc(&last)
    )
}

fn coverage_line_csv(plan: &str, case: &str, runs: &[&manifest::RunEntry]) -> String {
    let ended = |status: &str| {
        runs.iter()
            .filter(|entry| str_of(&entry.status) == status)
            .count()
    };
    let last = runs
        .last()
        .map(|entry| str_of(&entry.started))
        .unwrap_or_default();
    [
        plan.to_owned(),
        case.to_owned(),
        runs.len().to_string(),
        ended("complete").to_string(),
        ended("partial").to_string(),
        ended("aborted").to_string(),
        last,
    ]
    .iter()
    .map(|field| csv_field(field))
    .collect::<Vec<_>>()
    .join(",")
}

/// One run's row in the Runs table.
struct RunRow {
    plan: String,
    case: String,
    run_id: String,
    /// `runs/<project_id>/<sop_id>` or `runs/<sop_id>` when the repo has no project, so the
    /// package's links point at the run wherever the record actually lives.
    subdir: String,
    started: String,
    site: String,
    operator: String,
    sensor: String,
    outcome: String,
    conclusion: String,
    deviations: String,
    log_count: usize,
    photo_count: usize,
    file_count: usize,
    notes: String,
}

impl RunRow {
    fn of(repo: &Repo, plan: &str, case: &str, entry: &manifest::RunEntry) -> Self {
        let sop = str_of(&entry.sop);
        let run_id = str_of(&entry.run_id);
        Self {
            plan: plan.to_owned(),
            case: case.to_owned(),
            sensor: sensor_of(entry),
            started: str_of(&entry.started),
            site: str_of(&entry.site),
            operator: str_of(&entry.operator),
            outcome: str_of(&entry.status),
            conclusion: str_of(&entry.conclusion),
            deviations: entry
                .deviations_count
                .as_ref()
                .and_then(Json::as_i64)
                .unwrap_or(0)
                .to_string(),
            log_count: entry.log_count,
            photo_count: entry.photo_count,
            file_count: entry.file_count,
            notes: notes_of(repo, &sop, &run_id),
            run_id,
            subdir: crate::run::run_subdir(repo, &sop),
        }
    }

    fn folder(&self) -> String {
        format!("runs/{}/{}", self.subdir, self.run_id)
    }

    fn files(&self) -> String {
        format!(
            "{}/{}/{}",
            self.log_count, self.photo_count, self.file_count
        )
    }

    fn markdown(&self, fmt_time: FmtTime<'_>) -> String {
        format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            esc(&self.plan),
            esc(&self.case),
            esc(&self.run_id),
            esc(&fmt_time(&self.started)),
            esc(&self.site),
            esc(&self.operator),
            esc(&self.sensor),
            esc(&self.outcome),
            esc(&self.conclusion),
            esc(&self.deviations),
            esc(&self.files()),
            esc(&self.notes),
            esc(&self.folder()),
        )
    }

    fn csv(&self, fmt_time: FmtTime<'_>) -> String {
        [
            self.plan.clone(),
            self.case.clone(),
            self.run_id.clone(),
            fmt_time(&self.started),
            self.site.clone(),
            self.operator.clone(),
            self.sensor.clone(),
            self.outcome.clone(),
            self.conclusion.clone(),
            self.deviations.clone(),
            self.files(),
            self.notes.clone(),
            self.folder(),
        ]
        .iter()
        .map(|field| csv_field(field))
        .collect::<Vec<_>>()
        .join(",")
    }
}

/// A run's notes, run-level first, then each step's, joined for a table cell.
fn notes_of(repo: &Repo, sop: &str, run_id: &str) -> String {
    let Ok(loaded) = run::load(repo, sop, run_id) else {
        return String::new();
    };
    let mut notes: Vec<String> = loaded
        .state
        .run_notes
        .iter()
        .map(|note| note.text.clone())
        .collect();
    for step in loaded.state.steps.values() {
        notes.extend(step.notes.iter().map(|note| note.text.clone()));
    }
    notes.join("; ")
}

/// The instrument as History shows it: the serial when there is one, else the model.
fn sensor_of(entry: &manifest::RunEntry) -> String {
    let Some(sensor) = entry.sensor.as_ref().and_then(Json::as_object) else {
        return String::new();
    };
    let pick = |key: &str| {
        sensor
            .get(key)
            .and_then(Json::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    match (pick("serial"), pick("model")) {
        (Some(serial), Some(model)) => format!("{model} {serial}"),
        (Some(serial), None) => serial,
        (None, Some(model)) => model,
        (None, None) => String::new(),
    }
}

/// A JSON value read as text; anything that is not a string reads as empty.
fn str_of(value: &Option<Json>) -> String {
    value
        .as_ref()
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_owned()
}

/// Escape a value for a markdown table cell: `|` cannot appear inside a cell, and a
/// newline would end the row.
fn esc(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

/// Escape a value for a CSV field per RFC 4180: quote when it contains a comma, a quote,
/// or a line break, and double any quote inside.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}
