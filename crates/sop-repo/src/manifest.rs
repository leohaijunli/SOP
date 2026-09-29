//! `dist/manifest.json`: everything the field app needs, in one generated file.
//!
//! The app loads one file instead of walking the repository, so it can work from a
//! single fetch or a local directory copy. `dist/` is generated, never edited by hand.

use serde::Serialize;
use serde_json::{Value as Json, json};

use sop_core::{DEFAULT_HELP_ORDER, Mapping, Number, Step, Value};

use crate::{Repo, resolve_checklist};

#[derive(Debug, Serialize)]
pub struct Manifest {
    pub schema: i64,
    /// The project this repository is for, when it declares one.
    pub project: Option<ProjectEntry>,
    pub procedures: Vec<ProcedureEntry>,
    pub checklists: Vec<ChecklistEntry>,
    /// Test plans and their cases, from the `testplan/` directory tree.
    pub testplans: Vec<crate::testplan::TestPlan>,
    pub runs: Vec<RunEntry>,
    /// Help pages for the side panel, in panel order.
    pub help: Vec<HelpEntry>,
    /// The panel's section headings, in panel order.
    pub help_sections: Vec<HelpSection>,
}

#[derive(Debug, Serialize)]
pub struct ProjectEntry {
    pub project_id: Option<Json>,
    pub title: Option<Json>,
    pub institution: Option<Json>,
    pub lead: Option<Json>,
    pub started: Option<Json>,
    pub updated: Option<Json>,
    pub summary: Option<Json>,
    pub applies_to: Vec<Json>,
    pub tags: Vec<Json>,
    /// Sites named in `project.md`, offered in the start-a-run form's site field.
    pub sites: Vec<Json>,
    pub path: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct HelpEntry {
    pub help_id: Option<Json>,
    pub title: Option<Json>,
    pub section: Option<Json>,
    pub order: Json,
    pub audience: Option<Json>,
    pub summary: Option<Json>,
    pub updated: Option<Json>,
    pub tags: Vec<Json>,
    pub path: String,
    /// The Markdown body, so the panel does not need a second read.
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct HelpSection {
    pub title: String,
    pub order: i64,
}

impl Manifest {
    /// Pretty JSON with a trailing newline, so the file is stable under `git diff`.
    pub fn to_json(&self) -> String {
        let mut out = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_owned());
        out.push('\n');
        out
    }
}

#[derive(Debug, Serialize)]
pub struct ProcedureEntry {
    pub procedure_id: Option<Json>,
    pub title: Option<Json>,
    pub version: Option<Json>,
    pub updated: Option<Json>,
    pub applies_to: Vec<Json>,
    pub tags: Vec<Json>,
    pub path: String,
    pub steps: Vec<StepEntry>,
}

#[derive(Debug, Serialize)]
pub struct ChecklistEntry {
    pub sop_id: Option<Json>,
    pub title: Option<Json>,
    pub version: Option<Json>,
    pub updated: Option<Json>,
    pub status: Option<Json>,
    pub applies_to: Vec<Json>,
    pub equipment: Vec<Json>,
    /// The `conditions:` the checklist declares, for the start-a-run form.
    pub conditions: Vec<sop_core::front::ConditionDecl>,
    pub path: String,
    pub step_count: usize,
    pub unresolved_includes: Vec<String>,
    pub steps: Vec<StepEntry>,
}

#[derive(Debug, Serialize)]
pub struct RunEntry {
    pub run_id: Option<Json>,
    pub sop: Option<Json>,
    pub sop_version: Option<Json>,
    pub operator: Option<Json>,
    pub site: Option<Json>,
    pub plan: Option<Json>,
    pub case: Option<Json>,
    pub started: Option<Json>,
    pub status: Option<Json>,
    pub conclusion: Option<Json>,
    pub deviations_count: Option<Json>,
    /// The instrument recorded at start, so a new run can be seeded from the last one.
    pub sensor: Option<Json>,
    /// The conditions recorded at start, for the same reason.
    pub conditions: Option<Json>,
    pub path: String,
    pub step_count: usize,
}

#[derive(Debug, Serialize)]
pub struct StepEntry {
    pub id: Option<Json>,
    pub title: Option<Json>,
    pub kind: Option<Json>,
    pub severity: Option<Json>,
    pub deprecated: bool,
    pub captures: Vec<Json>,
    /// The prose the operator reads, with the `yaml step` block removed.
    pub body: String,
    pub source: Option<Json>,
}

/// Build the manifest for a repository.
pub fn build(repo: &Repo) -> Manifest {
    let found = repo.discover();

    let project = found.project.as_ref().and_then(|path| {
        let loaded = repo.load(path).ok()?;
        let front = &loaded.doc.front;
        Some(ProjectEntry {
            project_id: front.get("project_id").map(yaml_to_json),
            title: front.get("title").map(yaml_to_json),
            institution: front.get("institution").map(yaml_to_json),
            lead: front.get("lead").map(yaml_to_json),
            started: front.get("started").map(yaml_to_json),
            updated: front.get("updated").map(yaml_to_json),
            summary: front.get("summary").map(yaml_to_json),
            applies_to: list_of(front.get("applies_to")),
            tags: list_of(front.get("tags")),
            sites: list_of(front.get("sites")),
            body: loaded.doc.body.trim_start().to_owned(),
            path: loaded.relpath.clone(),
        })
    });

    let mut procedures = Vec::new();
    for path in &found.procedures {
        let Ok(loaded) = repo.load(path) else {
            continue;
        };
        let front = &loaded.doc.front;
        procedures.push(ProcedureEntry {
            procedure_id: front.get("procedure_id").map(yaml_to_json),
            title: front.get("title").map(yaml_to_json),
            version: front.get("version").map(yaml_to_json),
            updated: front.get("updated").map(yaml_to_json),
            applies_to: list_of(front.get("applies_to")),
            tags: list_of(front.get("tags")),
            path: loaded.relpath.clone(),
            steps: loaded.doc.steps.iter().map(step_entry).collect(),
        });
    }
    procedures.sort_by_key(|entry| py_str(&entry.procedure_id));

    let mut checklists = Vec::new();
    for path in &found.checklists {
        let Ok(loaded) = repo.load(path) else {
            continue;
        };
        let resolved = resolve_checklist(repo, &loaded);
        let front = &loaded.doc.front;
        checklists.push(ChecklistEntry {
            sop_id: front.get("sop_id").map(yaml_to_json),
            title: front.get("title").map(yaml_to_json),
            version: front.get("version").map(yaml_to_json),
            updated: front.get("updated").map(yaml_to_json),
            status: front.get("status").map(yaml_to_json),
            applies_to: list_of(front.get("applies_to")),
            equipment: list_of(front.get("equipment")),
            conditions: front.conditions(),
            path: loaded.relpath.clone(),
            step_count: resolved.steps.len(),
            unresolved_includes: resolved
                .problems
                .iter()
                .map(|(_, message)| message.clone())
                .collect(),
            steps: resolved.steps.iter().map(step_entry).collect(),
        });
    }
    checklists.sort_by_key(|entry| py_str(&entry.sop_id));

    let mut runs = Vec::new();
    for path in &found.runs {
        let Ok(loaded) = repo.load(path) else {
            continue;
        };
        let front = &loaded.doc.front;
        runs.push(RunEntry {
            run_id: front.get("run_id").map(yaml_to_json),
            sop: front.get("sop").map(yaml_to_json),
            sop_version: front.get("sop_version").map(yaml_to_json),
            operator: front.get("operator").map(yaml_to_json),
            site: front.get("site").map(yaml_to_json),
            plan: front.get("plan").map(yaml_to_json),
            case: front.get("case").map(yaml_to_json),
            started: front.get("started").map(yaml_to_json),
            status: front.get("status").map(yaml_to_json),
            conclusion: front.get("conclusion").map(yaml_to_json),
            deviations_count: if front.contains("deviations_count") {
                front.get("deviations_count").map(yaml_to_json)
            } else {
                Some(json!(0))
            },
            sensor: front.get("sensor").map(yaml_to_json),
            conditions: front.get("conditions").map(yaml_to_json),
            path: loaded.relpath.clone(),
            step_count: loaded.doc.results.len(),
        });
    }
    runs.sort_by_key(|entry| py_str(&entry.run_id));

    let mut help: Vec<HelpEntry> = found
        .help
        .iter()
        .filter_map(|path| {
            let loaded = repo.load(path).ok()?;
            let front = &loaded.doc.front;
            Some(HelpEntry {
                help_id: front.get("help_id").map(yaml_to_json),
                title: front.get("title").map(yaml_to_json),
                section: front.get("section").map(yaml_to_json),
                order: front
                    .get("order")
                    .map(yaml_to_json)
                    .unwrap_or_else(|| json!(DEFAULT_HELP_ORDER)),
                audience: front.get("audience").map(yaml_to_json),
                summary: front.get("summary").map(yaml_to_json),
                updated: front.get("updated").map(yaml_to_json),
                tags: list_of(front.get("tags")),
                body: loaded.doc.body.trim_start().to_owned(),
                path: loaded.relpath.clone(),
            })
        })
        .collect();
    // Panel order is the contract: `order` is global across help pages, so a section
    // appears wherever its first page appears. Section and id break ties, so the result
    // never depends on directory iteration order.
    help.sort_by(|a, b| {
        let key = |entry: &HelpEntry| {
            (
                entry.order.as_i64().unwrap_or(DEFAULT_HELP_ORDER),
                py_str(&entry.section),
                py_str(&entry.help_id),
            )
        };
        key(a).cmp(&key(b))
    });

    let mut section_order: Vec<(String, i64)> = Vec::new();
    for entry in &help {
        let title = py_str(&entry.section);
        if title == "None" {
            continue;
        }
        let order = entry.order.as_i64().unwrap_or(DEFAULT_HELP_ORDER);
        match section_order
            .iter_mut()
            .find(|(existing, _)| *existing == title)
        {
            Some((_, existing)) => *existing = (*existing).min(order),
            None => section_order.push((title, order)),
        }
    }
    // Not sorted again here: `help` is already in panel order, so the sections come out
    // in the order their first page appears.
    let help_sections = section_order
        .into_iter()
        .map(|(title, order)| HelpSection { title, order })
        .collect();

    Manifest {
        schema: 1,
        project,
        procedures,
        checklists,
        testplans: crate::testplan::plans(&repo),
        runs,
        help,
        help_sections,
    }
}

fn step_entry(step: &Step) -> StepEntry {
    StepEntry {
        id: step.id.clone().map(Json::String),
        title: step.title.clone().map(Json::String),
        kind: step
            .key
            .as_ref()
            .map(|key| Json::String(key.as_str().to_owned())),
        severity: Some(Json::String(
            step.severity.clone().unwrap_or_else(|| "normal".to_owned()),
        )),
        deprecated: step.deprecated,
        captures: step
            .captures
            .iter()
            .map(|capture| json_safe(&Value::Mapping(capture.raw.clone())))
            .collect(),
        body: step.prose.clone(),
        source: step.source.clone().map(Json::String),
    }
}

/// Python's `str()` for the shapes that end up in an id column, used only for sorting.
fn py_str(value: &Option<Json>) -> String {
    match value {
        None | Some(Json::Null) => "None".to_owned(),
        Some(Json::String(text)) => text.clone(),
        Some(Json::Bool(flag)) => if *flag { "True" } else { "False" }.to_owned(),
        Some(other) => other.to_string(),
    }
}

/// A front matter value read as a list, with scalars treated as one-element lists.
fn list_of(value: Option<&Value>) -> Vec<Json> {
    match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Sequence(items)) => items.iter().map(yaml_to_json).collect(),
        Some(single) => vec![yaml_to_json(single)],
    }
}

/// Drop the private `_`-prefixed keys that the reference tooling adds while parsing.
pub fn json_safe(value: &Value) -> Json {
    match value {
        Value::Null => Json::Null,
        Value::Bool(flag) => Json::Bool(*flag),
        Value::Number(number) => number_to_json(number),
        Value::String(text) => Json::String(text.clone()),
        Value::Sequence(items) => Json::Array(items.iter().map(json_safe).collect()),
        Value::Mapping(map) => Json::Object(public_entries(map)),
        Value::Tagged(tagged) => json_safe(&tagged.value),
    }
}

fn public_entries(map: &Mapping) -> serde_json::Map<String, Json> {
    let mut out = serde_json::Map::new();
    for (key, value) in map {
        let Some(key) = key.as_str() else { continue };
        if key.starts_with('_') {
            continue;
        }
        out.insert(key.to_owned(), json_safe(value));
    }
    out
}

fn yaml_to_json(value: &Value) -> Json {
    json_safe(value)
}

fn number_to_json(number: &Number) -> Json {
    if let Some(value) = number.as_i64() {
        return Json::from(value);
    }
    if let Some(value) = number.as_u64() {
        return Json::from(value);
    }
    match number.as_f64().and_then(serde_json::Number::from_f64) {
        Some(value) => Json::Number(value),
        None => Json::Null,
    }
}
