//! Test plans: folders under `testplan/`, each holding the test cases for one plan.
//!
//! A test plan is a directory `testplan/<plan-id>/`; each markdown file directly inside
//! it is one test case. A case is parsed the same way any standalone checklist is, so the
//! headings become steps and it can be run with `run start` by its file path.
//!
//! Ordering is explicit, not filename-based: a plan's `plan.md` and each case's front
//! matter may declare an `order` integer stub, and both are sorted by `order` then id.
//! A case or plan without an `order` sorts after one that has it.

use std::path::Path;

use serde::Serialize;

use crate::{Repo, markdown_in};

/// Cases without an explicit `order` sort after the ones that have one.
const DEFAULT_ORDER: i64 = 10_000;

#[derive(Debug, Serialize)]
pub struct TestCase {
    /// File stem: the stable id of the case within its plan.
    pub id: String,
    /// The checklist id a run of this case records: the case's front-matter `sop_id`,
    /// or the file stem when it declares none. This is what links a run back to its
    /// case, because a run records the checklist id, not the case's file path.
    pub sop_id: String,
    pub title: Option<String>,
    /// Repository-relative path, e.g. `testplan/calib/power-on.md`.
    pub path: String,
    pub step_count: usize,
    /// Front-matter `order`, for running the cases of a plan in sequence.
    pub order: i64,
}

#[derive(Debug, Serialize)]
pub struct TestPlan {
    /// Directory name under `testplan/`.
    pub id: String,
    /// Directory name, or a `title` read from a `plan.md` in the folder when present.
    pub title: Option<String>,
    /// Repository-relative directory, e.g. `testplan/calib`.
    pub path: String,
    /// Front-matter `order` in `plan.md`, for ordering plans in the test-plans screen.
    pub order: i64,
    pub cases: Vec<TestCase>,
}

/// Every test plan, ordered by `order` then id, with its cases in `order` then id.
pub fn plans(repo: &Repo) -> Vec<TestPlan> {
    let mut out = Vec::new();
    for dir in repo.discover().testplan_dirs {
        let name = match dir.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_owned(),
            None => continue,
        };
        let title = plan_title(repo, &dir).or_else(|| Some(name.clone()));
        let order = plan_order(repo, &dir);

        let mut cases = Vec::new();
        for path in markdown_in(&dir) {
            // `plan.md` defines the plan itself, not a test case.
            if path.file_name().is_some_and(|n| n == "plan.md") {
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            let (sop_id, case_title, step_count, case_order) = case_meta(repo, &path, &id);
            cases.push(TestCase {
                id,
                sop_id,
                title: case_title,
                path: path.display().to_string(),
                step_count,
                order: case_order,
            });
        }
        cases.sort_by_key(|c| (c.order, c.id.clone()));

        out.push(TestPlan {
            id: name,
            title,
            path: dir.display().to_string(),
            order,
            cases,
        });
    }
    out.sort_by_key(|p| (p.order, p.id.clone()));
    out
}

/// A plan's display title: a `title` in `plan.md` front matter if there is one.
fn plan_title(repo: &Repo, dir: &Path) -> Option<String> {
    plan_doc(repo, dir)
        .and_then(|doc| doc.front.str("title").flatten().map(str::to_owned))
}

/// A plan's `order`, from `plan.md` front matter when present.
fn plan_order(repo: &Repo, dir: &Path) -> i64 {
    plan_doc(repo, dir)
        .and_then(|doc| doc.front.i64("order"))
        .unwrap_or(DEFAULT_ORDER)
}

fn plan_doc(repo: &Repo, dir: &Path) -> Option<sop_core::Document> {
    let plan_file = dir.join("plan.md");
    let text = repo.read_text(&plan_file).ok()?;
    sop_core::Document::parse_standalone(&text).ok()
}

/// A case's checklist id, title, step count, and `order`, parsed from its own markdown.
///
/// The id follows the same rule the app runs a case by: the front-matter `sop_id` when
/// there is one, the file stem otherwise.
fn case_meta(repo: &Repo, path: &Path, id: &str) -> (String, Option<String>, usize, i64) {
    let Ok(text) = repo.read_text(path) else {
        return (id.to_owned(), None, 0, DEFAULT_ORDER);
    };
    let Ok(doc) = sop_core::Document::parse_standalone(&text) else {
        return (id.to_owned(), None, 0, DEFAULT_ORDER);
    };
    let sop_id = doc
        .front
        .str("sop_id")
        .flatten()
        .map(str::to_owned)
        .unwrap_or_else(|| id.to_owned());
    let title = doc.front.str("title").flatten().map(str::to_owned);
    let order = doc.front.i64("order").unwrap_or(DEFAULT_ORDER);
    (sop_id, title, doc.steps.len(), order)
}
