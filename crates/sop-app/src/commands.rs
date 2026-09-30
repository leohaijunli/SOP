//! The operations the window can ask for.
//!
//! Every edit goes through `sop_repo::authoring`, which checks the result before writing
//! it and undoes it if the repository as a whole stops validating. Nothing here decides
//! whether an edit is allowed; it only carries the answer back, including the report
//! when the answer is no.
//!
//! Every command is `#[tauri::command(async)]`. Tauri runs a synchronous command on the
//! window's main thread, so a slow one - a `git pull` with no network, a directory walk
//! over a big repository, a copy of a large attachment - freezes the window while it runs.
//! The `async` flag moves the same body onto Tauri's blocking thread pool, which is the
//! `spawn_blocking` this code would otherwise have to spell out by hand. The functions
//! stay synchronous so they can still be called directly from Rust.

use std::path::{Path, PathBuf};

use sop_core::authoring::{ItemRef, Move};
use sop_core::RunEvent;
use sop_repo::{Repo, authoring, git, manifest, project, run, validate};
use tauri::State;

use crate::api::{
    CaptureInput, ChecklistItemView, ContentCounts, GitInfo, ProjectField, ProjectView,
    RunAddedStepView, RunAttachmentView, RunCaptureView, RunClockView, RunMarkerView,
    RunMetaInput, RunStepView, RunView, SensorView, SettingRow, Status, StepInput, StepPatch,
    ValidationCounts,
};
use crate::state::AppState;

type Reply<T> = Result<T, String>;

// ------------------------------------------------------------------- reading

#[tauri::command(async)]
pub fn manifest_json(state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    Ok(manifest::build(&repo).to_json())
}

/// The test plans and their cases, from the `testplan/` tree of the testcase repository.
#[tauri::command(async)]
pub fn test_plans(state: State<'_, AppState>) -> Reply<Vec<sop_repo::testplan::TestPlan>> {
    let repo = Repo::open(state.testcase_repo_root()?);
    Ok(sop_repo::testplan::plans(&repo))
}

/// Pull the working copy and the testcase repository, fast-forward only, and report.
#[tauri::command(async)]
pub fn sync_pull(state: State<'_, AppState>) -> Reply<String> {
    let mut lines: Vec<String> = Vec::new();

    let work_root = state.root()?;
    let work_remote = state.settings()?.remote;
    match git::pull(&work_root, &work_remote) {
        Ok(log) => lines.push(format!("working copy: {}", log.join("; "))),
        Err(error) => lines.push(format!("working copy: {error}")),
    }

    let tc_root = state.testcase_repo_root()?;
    let tc_remote = state.settings()?.remote;
    if tc_root != work_root {
        match git::pull(&tc_root, &tc_remote) {
            Ok(log) => lines.push(format!("test cases: {}", log.join("; "))),
            Err(error) => lines.push(format!("test cases: {error}")),
        }
    }

    Ok(lines.join("\n"))
}

/// Commit and push the testcase repository after the operator changed a case or plan.
#[tauri::command(async)]
pub fn testcase_push(message: String, state: State<'_, AppState>) -> Reply<String> {
    let root = state.testcase_repo_root()?;
    let remote = state.settings()?.remote;
    let report = git::commit_and_push(&root, &remote, &message).map_err(text)?;
    Ok(report.log.join("; "))
}

/// Duplicate a test case (a plan folder's md) so a repeat or variant experiment has its
/// own case. The copy gets a new id and title from its new filename.
#[tauri::command(async)]
pub fn duplicate_test_case(path: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = Repo::open(state.testcase_repo_root()?);
    let src = resolve_file(&repo, &path)?;
    if !src.is_file() {
        return Err(format!("{path}: no such test case"));
    }
    let parent = src.parent().unwrap_or_else(|| repo.root());
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("case")
        .to_owned();

    let mut n = 2;
    let dest = loop {
        let candidate = parent.join(format!("{stem}-{n}.md"));
        if !candidate.exists() {
            break candidate;
        }
        n += 1;
    };

    let text = std::fs::read_to_string(&src)
        .map_err(|error| format!("{}: {error}", src.display()))?;
    let new_id = dest
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&stem)
        .to_owned();
    let text = sop_core::front::set_field(&text, "sop_id", &new_id).unwrap_or(text);
    let text = sop_core::front::set_field(&text, "title", &new_id.replace('-', " ")).unwrap_or(text);
    std::fs::write(&dest, text)
        .map_err(|error| format!("{}: {error}", dest.display()))?;
    Ok(repo.relpath(&dest))
}

/// Delete a test case (remove its md from the plan folder).
#[tauri::command(async)]
pub fn delete_test_case(path: String, state: State<'_, AppState>) -> Reply<()> {
    let repo = Repo::open(state.testcase_repo_root()?);
    let file = resolve_file(&repo, &path)?;
    std::fs::remove_file(&file)
        .map_err(|error| format!("{}: {error}", file.display()))?;
    Ok(())
}

/// Import a markdown file into a plan folder as a new test case. The source file is
/// copied in under a unique name and given a fresh id/title so it is a runnable case.
#[tauri::command(async)]
pub fn import_test_case(plan_path: String, source: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = Repo::open(state.testcase_repo_root()?);
    let plan_dir = resolve_file(&repo, &plan_path)?;
    if !plan_dir.is_dir() {
        return Err(format!("{plan_path}: no such plan folder"));
    }
    let src = resolve_file(&repo, &source)?;
    let text = std::fs::read_to_string(&src)
        .map_err(|error| format!("{}: {error}", src.display()))?;

    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| "case".to_owned());

    let mut candidate = plan_dir.join(format!("{stem}.md"));
    let mut n = 2;
    while candidate.exists() {
        candidate = plan_dir.join(format!("{stem}-{n}.md"));
        n += 1;
    }

    let new_id = candidate
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&stem)
        .to_owned();
    let text = sop_core::front::set_field(&text, "sop_id", &new_id).unwrap_or(text);
    let text = sop_core::front::set_field(&text, "title", &new_id.replace('-', " ")).unwrap_or(text);
    std::fs::write(&candidate, text)
        .map_err(|error| format!("{}: {error}", candidate.display()))?;
    Ok(repo.relpath(&candidate))
}

/// Create a new test plan: a folder under `testplan/` with a `plan.md` carrying its title.
#[tauri::command(async)]
pub fn create_test_plan(name: String, title: String, state: State<'_, AppState>) -> Reply<String> {
    let name = name.trim().to_lowercase();
    if !sop_core::vocab::is_valid_id(&name) {
        return Err("plan name must be a lowercase id with hyphens".to_owned());
    }
    let root = state.testcase_root()?;
    let dir = root.join(&name);
    if dir.exists() {
        return Err(format!("plan '{name}' already exists"));
    }
    let title = title.trim();
    let title = if title.is_empty() { name.clone() } else { title.to_owned() };
    std::fs::create_dir_all(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    let plan_file = dir.join("plan.md");
    let text = format!(
        "---\ntitle: {}\norder: 100\nversion: 1\n---\n\n{}\n",
        sop_core::front::format_scalar(&title),
        title
    );
    std::fs::write(&plan_file, text).map_err(|error| format!("{}: {error}", plan_file.display()))?;
    Ok(dir.display().to_string())
}

/// A compact test summary of every run, in test-plan / test-case order, plus a coverage
/// line for every case whether it has been run or not. Written to `out` (via the save
/// dialog the frontend opens) and returned as text so the window can show it.
#[tauri::command(async)]
pub fn run_summary(out: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    let testcases = Repo::open(state.testcase_repo_root()?);
    let text = sop_repo::summary::markdown(&repo, &testcases);
    let path = PathBuf::from(&out);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, text.as_bytes())
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(path.display().to_string())
}

/// Remove one run: its record file, run directory, and log directory.
///
/// Both ids end up in a path, so both are checked as ids before anything is removed. The
/// operator is asked first by the window; this only does what it is told.
#[tauri::command(async)]
pub fn run_delete(sop: String, run_id: String, state: State<'_, AppState>) -> Reply<usize> {
    let repo = state.repo()?;
    if !sop_core::vocab::is_valid_id(&sop) {
        return Err(format!("'{sop}' is not a checklist id"));
    }
    if !sop_core::vocab::is_valid_id(&run_id) {
        return Err(format!("'{run_id}' is not a run id"));
    }
    let removed = run::delete(&repo, &sop, &run_id).map_err(text)?;
    Ok(removed.len())
}

/// Remove every run in the working copy and report how many there were.
///
/// `runs/_inbox/` is left alone: it holds field observations, not run history.
#[tauri::command(async)]
pub fn run_delete_all(state: State<'_, AppState>) -> Reply<usize> {
    let repo = state.repo()?;
    let runs = run::all_runs(&repo).len();
    run::delete_all(&repo).map_err(text)?;
    Ok(runs)
}

/// Resolve a case path: absolute as-is, otherwise against the repository root.
fn resolve_file(repo: &Repo, path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if p.is_absolute() {
        Ok(p.to_path_buf())
    } else {
        Ok(repo.resolve(path))
    }
}

#[tauri::command(async)]
pub fn status(state: State<'_, AppState>) -> Reply<Status> {
    build_status(&state)
}

/// Render Markdown to HTML. The renderer lives in `sop_core::md`, so the app, the
/// preview server, and (later) the record export all read a note the same way.
#[tauri::command(async)]
pub fn render_markdown(text: String) -> Reply<String> {
    Ok(sop_core::md::render(&text))
}

#[tauri::command(async)]
pub fn validation_report(state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    Ok(validate::validate_repository(&repo).report.render(repo.root()))
}

/// The procedures that can be included in a checklist, and whether it already has them.
#[tauri::command(async)]
pub fn procedure_choices(file: Option<String>, state: State<'_, AppState>) -> Reply<Vec<ProcedureChoice>> {
    let repo = state.repo()?;
    let included = match file.as_deref() {
        Some(file) => {
            let path = authoring::editable(&repo, file).map_err(text)?;
            let loaded = repo.load(&path).map_err(text)?;
            loaded
                .doc
                .includes
                .iter()
                .map(|include| include.target.clone())
                .collect::<Vec<String>>()
        }
        None => Vec::new(),
    };

    let mut out = Vec::new();
    for path in repo.discover().procedures {
        let relative = repo.relpath(&path);
        let (id, title) = match repo.load(&path) {
            Ok(loaded) => (
                loaded.doc.front.str("procedure_id").flatten().map(str::to_owned),
                loaded.doc.front.str("title").flatten().map(str::to_owned),
            ),
            Err(_) => (None, None),
        };
        out.push(ProcedureChoice {
            included: included.contains(&relative),
            id,
            title,
            path: relative,
        });
    }
    Ok(out)
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcedureChoice {
    pub path: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub included: bool,
}

// ------------------------------------------------------------------ settings

#[tauri::command(async)]
pub fn settings_rows(state: State<'_, AppState>) -> Reply<Vec<SettingRow>> {
    rows(&state)
}

#[tauri::command(async)]
pub fn settings_path(state: State<'_, AppState>) -> Reply<String> {
    Ok(state.settings_path().display().to_string())
}

#[tauri::command(async)]
pub fn settings_set(key: String, value: String, state: State<'_, AppState>) -> Reply<Vec<SettingRow>> {
    state.set_setting(&key, &value)?;
    rows(&state)
}

#[tauri::command(async)]
pub fn settings_unset(key: String, state: State<'_, AppState>) -> Reply<Vec<SettingRow>> {
    state.unset_setting(&key)?;
    rows(&state)
}

#[tauri::command(async)]
pub fn settings_repo_path(state: State<'_, AppState>) -> Reply<String> {
    Ok(state.settings_repo_path()?.display().to_string())
}

/// The combined Project + Settings view, as a single JSON document.
///
/// This is what the Project and Settings pages load on open; the operator can then edit
/// either page and press Confirm, which writes the same JSON back to `configure.json`
/// and applies the values to `project.md` and the settings files.
#[tauri::command(async)]
pub fn config_load(state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    let config_path = repo.resolve("configure.json");
    if config_path.is_file() {
        return std::fs::read_to_string(&config_path)
            .map_err(|error| format!("{}: {error}", config_path.display()));
    }
    // No config file yet: build a default from the current project.md and settings.
    let proj_path = repo.resolve(sop_core::vocab::PROJECT_FILE);
    let mut project_map = serde_json::Map::new();
    if let Ok(loaded) = repo.load(&proj_path) {
        for (key, _desc) in project::FIELDS {
            if let Some(value) = loaded.doc.front.str(key).flatten() {
                project_map.insert(key.to_string(), serde_json::Value::String(value.to_owned()));
            }
        }
    }
    let settings = state.settings()?;
    let mut settings_map = serde_json::Map::new();
    for (key, _desc) in sop_core::settings::KEYS {
        if let Some(value) = settings.get(key) {
            settings_map.insert(key.to_string(), serde_json::Value::String(value));
        }
    }
    let config = serde_json::json!({ "project": project_map, "settings": settings_map });
    Ok(serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_owned()))
}

/// Write the Project + Settings JSON back to `configure.json` and apply its values.
///
/// The Confirm button on the Project and Settings pages sends the whole edited document;
/// this applies the project fields and settings so the rest of the app sees them, then
/// stores the document for the next launch to load.
#[tauri::command(async)]
pub fn config_save(json: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    let parsed: serde_json::Value =
        serde_json::from_str(&json).map_err(|error| format!("config is not valid JSON: {error}"))?;

    // Write the config file first so it always lands, even if applying a field fails.
    let config_path = repo.resolve("configure.json");
    std::fs::write(&config_path, serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| "{}".to_owned()))
        .map_err(|error| format!("{}: {error}", config_path.display()))?;

    if let Some(project_obj) = parsed.get("project").and_then(|value| value.as_object()) {
        for (key, value) in project_obj {
            let value = value.as_str().unwrap_or_default();
            // Required project fields cannot be set to empty; leave a blank box alone so
            // the save does not fail on the first unedited field.
            if value.trim().is_empty() {
                continue;
            }
            project::set(&repo, key, value).map_err(text)?;
        }
    }

    if let Some(settings_obj) = parsed.get("settings").and_then(|value| value.as_object()) {
        for (key, value) in settings_obj {
            // `recent-repositories` is maintained by the app itself and cannot be set by
            // hand; applying it would fail the whole save.
            if key == "recent-repositories" {
                continue;
            }
            if let Some(value) = value.as_str() {
                state.set_setting(key, value)?;
            }
        }
    }

    Ok(config_path.display().to_string())
}

#[tauri::command(async)]
pub fn open_repository(repo: String, state: State<'_, AppState>) -> Reply<Status> {
    state.open(&repo)?;
    build_status(&state)
}

#[tauri::command(async)]
pub fn remote_url(state: State<'_, AppState>) -> Reply<Option<String>> {
    let repo = state.repo()?;
    let settings = state.settings()?;
    Ok(git::state(repo.root(), &settings.remote).url)
}

#[tauri::command(async)]
pub fn remote_set(url: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    let settings = state.settings()?;
    let url = url.trim();
    if url.is_empty() {
        return Err("the URL is empty".to_owned());
    }
    if !git::state(repo.root(), &settings.remote).is_repository {
        return Err(format!(
            "{} is not a git working copy; run `git init` there first",
            repo.root().display()
        ));
    }
    git::set_remote_url(repo.root(), &settings.remote, url).map_err(text)?;
    Ok(url.to_owned())
}

// ------------------------------------------------------------------- project

#[tauri::command(async)]
pub fn project_fields(state: State<'_, AppState>) -> Reply<ProjectView> {
    build_project(&state)
}

#[tauri::command(async)]
pub fn project_set(key: String, value: String, state: State<'_, AppState>) -> Reply<ProjectView> {
    let repo = state.repo()?;
    project::set(&repo, &key, &value).map_err(text)?;
    build_project(&state)
}

// ------------------------------------------------------------------- editing

#[tauri::command(async)]
pub fn step_add(
    file: String,
    step: StepInput,
    after: Option<String>,
    state: State<'_, AppState>,
) -> Reply<String> {
    let repo = state.repo()?;
    let after = after.map(ItemRef::Step);
    let path = authoring::add_step(&repo, &file, &step.into(), after).map_err(text)?;
    Ok(repo.relpath(&path))
}

#[tauri::command(async)]
pub fn step_update(
    file: String,
    id: String,
    patch: StepPatch,
    state: State<'_, AppState>,
) -> Reply<String> {
    let repo = state.repo()?;
    let path = authoring::update_step(&repo, &file, &id, &patch.into()).map_err(text)?;
    Ok(repo.relpath(&path))
}

#[tauri::command(async)]
pub fn step_remove(file: String, id: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    let path = authoring::remove_step(&repo, &file, &id).map_err(text)?;
    Ok(repo.relpath(&path))
}

/// Move a step or an include marker past its neighbour. `up` is false for down.
#[tauri::command(async)]
pub fn item_move(
    file: String,
    kind: String,
    id: String,
    up: bool,
    state: State<'_, AppState>,
) -> Reply<String> {
    let repo = state.repo()?;
    let item = if kind == "include" {
        ItemRef::Include(id)
    } else {
        ItemRef::Step(id)
    };
    let direction = if up { Move::Up } else { Move::Down };
    let path = authoring::move_item(&repo, &file, &item, direction).map_err(text)?;
    Ok(repo.relpath(&path))
}

#[tauri::command(async)]
pub fn capture_set(
    file: String,
    step: String,
    capture: CaptureInput,
    state: State<'_, AppState>,
) -> Reply<String> {
    let repo = state.repo()?;
    let path = authoring::set_capture(&repo, &file, &step, &capture.into()).map_err(text)?;
    Ok(repo.relpath(&path))
}

#[tauri::command(async)]
pub fn capture_remove(
    file: String,
    step: String,
    key: String,
    state: State<'_, AppState>,
) -> Reply<String> {
    let repo = state.repo()?;
    let path = authoring::remove_capture(&repo, &file, &step, &key).map_err(text)?;
    Ok(repo.relpath(&path))
}

#[tauri::command(async)]
pub fn include_add(file: String, target: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    let path = authoring::add_include(&repo, &file, &target, None).map_err(text)?;
    Ok(repo.relpath(&path))
}

#[tauri::command(async)]
pub fn include_remove(file: String, target: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    let path = authoring::remove_include(&repo, &file, &target).map_err(text)?;
    Ok(repo.relpath(&path))
}

// ------------------------------------------------------------------ running

#[tauri::command(async)]
pub fn run_start(
    sop: String,
    run_id: String,
    operator: String,
    site: String,
    r#override: Option<String>,
    meta: Option<RunMetaInput>,
    state: State<'_, AppState>,
) -> Reply<RunView> {
    let repo = state.repo()?;
    let meta = meta.unwrap_or_default();
    let sensor = sop_core::run::SensorIdentity {
        model: meta.sensor_model.unwrap_or_default().trim().to_owned(),
        serial: meta.sensor_serial.unwrap_or_default().trim().to_owned(),
        firmware: meta.sensor_firmware.unwrap_or_default().trim().to_owned(),
    };
    let meta = run::RunMeta {
        sensor: (!sensor.is_empty()).then_some(sensor),
        hardware: meta
            .hardware
            .into_iter()
            .map(|item| item.trim().to_owned())
            .filter(|item| !item.is_empty())
            .collect(),
        conditions: meta
            .conditions
            .into_iter()
            .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
            .filter(|(key, value)| !key.is_empty() && !value.is_empty())
            .collect(),
        plan: meta.plan.map(|value| value.trim().to_owned()).filter(|value| !value.is_empty()),
        case: meta.case.map(|value| value.trim().to_owned()).filter(|value| !value.is_empty()),
        clock: match (meta.clock_instrument_time, meta.clock_basis) {
            (Some(instrument_time), Some(basis))
                if !instrument_time.trim().is_empty() && !basis.trim().is_empty() =>
            {
                let instrument_time = instrument_time.trim().to_owned();
                let now_epoch = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                let offset_secs = sop_core::timestamp::epoch_seconds(&instrument_time)
                    .map(|instrument| instrument - now_epoch)
                    .unwrap_or(0);
                Some(sop_core::run::ClockInfo {
                    basis: basis.trim().to_owned(),
                    instrument_time,
                    offset_secs,
                })
            }
            _ => None,
        },
    };
    let loaded =
        run::start_with_meta(&repo, &sop, &run_id, &operator, &site, r#override.as_deref(), &meta)
            .map_err(text)?;
    Ok(build_run_view(&loaded, &run_id))
}

#[tauri::command(async)]
pub fn run_record(
    sop: String,
    run_id: String,
    event: RunEvent,
    state: State<'_, AppState>,
) -> Reply<RunView> {
    let repo = state.repo()?;
    let loaded = run::record(&repo, &sop, &run_id, &event).map_err(text)?;
    Ok(build_run_view(&loaded, &run_id))
}

#[tauri::command(async)]
pub fn run_state(sop: String, run_id: String, state: State<'_, AppState>) -> Reply<RunView> {
    let repo = state.repo()?;
    let loaded = run::load(&repo, &sop, &run_id).map_err(text)?;
    Ok(build_run_view(&loaded, &run_id))
}

/// Add a step to a run after it started, so the record can carry a measurement the
/// checklist did not foresee. The id is generated here (`adhoc-NNN`), never by the
/// operator (`DESIGN.md` 6.6); the snapshot stays frozen, and the step lives in the event
/// log, to be merged back into the runtime step list at replay.
#[tauri::command(async)]
pub fn run_add_step(
    sop: String,
    run_id: String,
    title: String,
    after: Option<String>,
    state: State<'_, AppState>,
) -> Reply<RunView> {
    let repo = state.repo()?;
    let loaded = run::load(&repo, &sop, &run_id).map_err(text)?;
    let title = title.trim().to_owned();
    if title.is_empty() {
        return Err("a step title is required".to_owned());
    }
    let event = RunEvent::StepAdded {
        at: String::new(), // stamped by the tool
        id: next_adhoc_id(&loaded.state),
        title,
        after: after.filter(|id| !id.trim().is_empty()).map(|id| id.trim().to_owned()),
    };
    let loaded = run::record(&repo, &sop, &run_id, &event).map_err(text)?;
    Ok(build_run_view(&loaded, &run_id))
}

/// The next `adhoc-NNN` id: one past the largest id already added to this run.
fn next_adhoc_id(state: &sop_core::RunState) -> String {
    let max = state
        .added_steps
        .iter()
        .filter_map(|step| step.id.strip_prefix("adhoc-"))
        .filter_map(|number| number.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("adhoc-{max:03}")
}

/// Drop a tagged field marker on the run: a timestamped, labelled moment for aligning a
/// log to a physical feature when the instrument clock is not the notebook clock.
#[tauri::command(async)]
pub fn run_marker(
    sop: String,
    run_id: String,
    label: String,
    state: State<'_, AppState>,
) -> Reply<RunView> {
    let repo = state.repo()?;
    let loaded = run::marker(&repo, &sop, &run_id, &label).map_err(text)?;
    Ok(build_run_view(&loaded, &run_id))
}

/// Whether the checklist this run was started from has changed since the snapshot.
#[tauri::command(async)]
pub fn run_drift(sop: String, run_id: String, state: State<'_, AppState>) -> Reply<run::Drift> {
    let repo = state.repo()?;
    run::drift(&repo, &sop, &run_id).map_err(text)
}

#[tauri::command(async)]
pub fn run_end(
    sop: String,
    run_id: String,
    status: String,
    conclusion: Option<String>,
    conclusion_note: Option<String>,
    state: State<'_, AppState>,
) -> Reply<RunView> {
let repo = state.repo()?;
    let conclusion = match (conclusion, conclusion_note) {
        (None, None) => None,
        (Some(outcome), Some(note)) => Some((outcome, note)),
        (Some(_), None) => return Err("conclusion requires a conclusion_note".to_owned()),
        (None, Some(_)) => return Err("conclusion_note requires a conclusion".to_owned()),
    };
    let loaded = run::end(&repo, &sop, &run_id, &status, conclusion).map_err(text)?;
    Ok(build_run_view(&loaded, &run_id))
}

#[tauri::command(async)]
pub fn run_attach(
    sop: String,
    run_id: String,
    step: Option<String>,
    path: String,
    state: State<'_, AppState>,
) -> Reply<RunView> {
    let repo = state.repo()?;
    run::attach(&repo, &sop, &run_id, step.as_deref(), std::path::Path::new(&path))
        .map_err(text)?;
    let loaded = run::load(&repo, &sop, &run_id).map_err(text)?;
    Ok(build_run_view(&loaded, &run_id))
}

/// `Some(text)` for a field with content, `None` for a blank one, so the view leaves a
/// label out rather than rendering an empty value.
fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
}

fn build_run_view(loaded: &run::LoadedRun, run_id: &str) -> RunView {
    use sop_core::run::StepStatus;
    let state = &loaded.state;
    let mut steps = Vec::new();
    for def in &loaded.defs {
        let step_state = state.steps.get(&def.id);
        let status = step_state.map(|s| s.status.clone()).unwrap_or(StepStatus::Open);
        let captures = def
            .captures
            .iter()
            .map(|def| {
                let value = step_state
                    .and_then(|s| s.captures.get(&def.key))
                    .map(|value| match &value.unit {
                        Some(unit) => format!("{} {}", value.value, unit),
                        None => value.value.clone(),
                    });
                RunCaptureView {
                    key: def.key.clone(),
                    label: def.label.clone(),
                    capture_type: def.capture_type.clone(),
                    unit: def.unit.clone(),
                    required: def.required,
                    options: def.options.clone(),
                    expected: def.expected.clone(),
                    value,
                    acknowledged: step_state
                        .is_some_and(|state| state.acknowledged.contains(&def.key)),
                }
            })
            .collect();
        // The frozen body carries the item text; the run state carries only the ticks.
        // Pairing them is what lets the operator tick the words they read.
        let ticks = step_state.map(|s| s.checkboxes.as_slice()).unwrap_or(&[]);
        let checklist = sop_core::run::checklist_state(&def.prose, ticks)
            .into_iter()
            .map(|(text, checked)| ChecklistItemView { text, checked })
            .collect();
        steps.push(RunStepView {
            id: def.id.clone(),
            title: def.title.clone(),
            prose: sop_core::run::body_without_checklist(&def.prose),
            severity: def.severity.clone(),
            kind: def.kind.clone(),
            status: status.as_str().to_owned(),
            reason: step_state.and_then(|s| s.reason.clone()),
            checklist,
            captures,
            outputs: def.outputs.clone(),
            notes: step_state
                .map(|s| s.notes.iter().map(|note| note.text.clone()).collect())
                .unwrap_or_default(),
            attachments: step_state
                .map(|state| state.attachments.iter().map(attachment_view).collect())
                .unwrap_or_default(),
        });
    }
    RunView {
        sop: state.sop.clone().unwrap_or_default(),
        run_id: run_id.to_owned(),
        operator: state.operator.clone(),
        site: state.site.clone(),
        plan: state.plan.clone(),
        case: state.case.clone(),
        started: state.started.clone(),
        ended: state.ended.clone(),
        run_status: state.run_status.clone(),
        conclusion: state.conclusion.clone(),
        conclusion_note: state.conclusion_note.clone(),
        snapshot_sha256: state.snapshot_sha256.clone(),
        sop_version: state.sop_version.clone(),
        sop_commit: state.sop_commit.clone(),
        deviations_count: state.deviations(),
        sensor: state.sensor.as_ref().map(|sensor| SensorView {
            model: non_empty(&sensor.model),
            serial: non_empty(&sensor.serial),
            firmware: non_empty(&sensor.firmware),
        }),
        hardware: state.hardware.clone(),
        conditions: state.conditions.clone(),
        clock: state.clock.as_ref().map(|clock| RunClockView {
            basis: clock.basis.clone(),
            instrument_time: clock.instrument_time.clone(),
            offset_secs: clock.offset_secs,
        }),
        steps,
        added_steps: state
            .added_steps
            .iter()
            .map(|added| RunAddedStepView {
                id: added.id.clone(),
                title: added.title.clone(),
                after: added.after.clone(),
            })
            .collect(),
        markers: state
            .markers
            .iter()
            .map(|marker| RunMarkerView {
                at: marker.at.clone(),
                label: marker.label.clone(),
            })
            .collect(),
        run_notes: state.run_notes.iter().map(|note| note.text.clone()).collect(),
        run_attachments: state
            .run_attachments
            .iter()
            .map(attachment_view)
            .collect(),
        record_path: loaded.record_path.display().to_string(),
        suggested_log_name: Some(run::suggest_log_name(&loaded, "")),
    }
}

fn attachment_view(attachment: &sop_core::run::Attachment) -> RunAttachmentView {
    RunAttachmentView {
        path: attachment.path.clone(),
        sha256: attachment.sha256.clone(),
        size: attachment.size,
        t_min: attachment.t_min.clone(),
        t_max: attachment.t_max.clone(),
        row_count: attachment.row_count,
    }
}

// --------------------------------------------------------------- publishing

/// One run's record, as the document `run end` commits.
///
/// The text is returned as well as written, so the window can show what the file says
/// without reading it back through a second command.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub bytes: usize,
    pub text: String,
}

/// Write the record for one run.
///
/// With no destination the document lands in `exports/<sop_id>-<run_id>.md` inside the
/// working copy. `exports/` is not one of the directories discovery reads, so an export
/// never changes what the content means.
#[tauri::command(async)]
pub fn run_export(
    sop: String,
    run_id: String,
    out: Option<String>,
    state: State<'_, AppState>,
) -> Reply<ExportResult> {
    let repo = state.repo()?;
    // Both ids end up in a path, so both are checked as ids before anything is written.
    if !sop_core::vocab::is_valid_id(&sop) {
        return Err(format!("'{sop}' is not a checklist id"));
    }
    if !sop_core::vocab::is_valid_id(&run_id) {
        return Err(format!("'{run_id}' is not a run id"));
    }
    let text = run::record_document(&repo, &sop, &run_id).map_err(text)?;

    let path = match out.as_deref().map(str::trim).filter(|out| !out.is_empty()) {
        Some(out) => repo.resolve(out),
        None => repo.resolve(&format!("exports/{sop}-{run_id}.md")),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    std::fs::write(&path, text.as_bytes())
        .map_err(|error| format!("{}: {error}", path.display()))?;

    Ok(ExportResult {
        path: path.display().to_string(),
        bytes: text.len(),
        text,
    })
}

/// Export a run record to a specific path.
///
/// The frontend opens the native save dialog (through `tauri-plugin-dialog`, which runs
/// it on the correct thread) and passes the chosen path here. A command thread must not
/// open a native dialog itself, which is why the dialog never lives in this file.
#[tauri::command(async)]
pub fn run_export_to(
    sop: String,
    run_id: String,
    out: String,
    state: State<'_, AppState>,
) -> Reply<ExportResult> {
    let repo = state.repo()?;
    if !sop_core::vocab::is_valid_id(&sop) {
        return Err(format!("'{sop}' is not a checklist id"));
    }
    if !sop_core::vocab::is_valid_id(&run_id) {
        return Err(format!("'{run_id}' is not a run id"));
    }
    let text = run::record_document(&repo, &sop, &run_id).map_err(text)?;

    let path = PathBuf::from(&out);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    std::fs::write(&path, text.as_bytes())
        .map_err(|error| format!("{}: {error}", path.display()))?;

    Ok(ExportResult {
        path: path.display().to_string(),
        bytes: text.len(),
        text,
    })
}

/// Load an external local markdown file and return it as a checklist entry.
///
/// The frontend opens the native file picker (via `tauri-plugin-dialog`) and passes the
/// chosen path here. The file can live anywhere: no `checklists/` or `procedures/`
/// folder is required, and a plain markdown file is parsed for `##`-heading steps.
#[tauri::command(async)]
pub fn load_external_md(path: String, state: State<'_, AppState>) -> Reply<Option<String>> {
    let _ = state;
    let path = PathBuf::from(&path);
    let text_content = std::fs::read_to_string(&path)
        .map_err(|error| format!("{}: {error}", path.display()))?;

    let doc = sop_core::Document::parse_standalone(&text_content)
        .map_err(|error| error.to_string())?;

    let filename = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let sop_id = doc
        .front
        .str("sop_id")
        .flatten()
        .map(str::to_owned)
        .unwrap_or_else(|| filename.clone());

    let title = doc
        .front
        .str("title")
        .flatten()
        .map(str::to_owned)
        .unwrap_or_else(|| filename.clone());

    let version = doc.front.str("version").flatten().map(str::to_owned);
    let status = doc.front.str("status").flatten().map(str::to_owned);
    let applies_to = doc.front.string_list("applies_to");
    let equipment = doc.front.string_list("equipment");
    let conditions = doc.front.conditions();

    let steps: Vec<serde_json::Value> = doc
        .steps
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "title": s.title,
                "kind": s.key.as_ref().map(|k| k.as_str()),
                "severity": s.severity.as_deref().unwrap_or("normal"),
                "deprecated": s.deprecated,
                "captures": s.captures.iter().map(|c| manifest::json_safe(&sop_core::Value::Mapping(c.raw.clone()))).collect::<Vec<_>>(),
                "body": s.prose,
                "source": s.source,
            })
        })
        .collect();

    let entry = serde_json::json!({
        "sop_id": sop_id,
        "title": title,
        "version": version,
        "updated": doc.front.str("updated").flatten(),
        "status": status,
        "applies_to": applies_to,
        "equipment": equipment,
        "conditions": conditions,
        "path": path.display().to_string(),
        "step_count": steps.len(),
        "unresolved_includes": [],
        "steps": steps,
    });

    Ok(Some(entry.to_string()))
}

/// Publish the working copy: `git add -A`, commit, push.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushResult {
    pub branch: Option<String>,
    pub remote: String,
    pub changed: usize,
    pub commit: Option<String>,
    pub up_to_date: bool,
    pub log: Vec<String>,
}

/// Stage everything, commit it when there is anything to commit, and push.
///
/// The app names the commit and the remote; `git` does the rest, with the operator's
/// identity and the operator's credentials. See `sop_repo::git`.
#[tauri::command(async)]
pub fn repo_push(message: String, state: State<'_, AppState>) -> Reply<PushResult> {
    let repo = state.repo()?;
    let settings = state.settings()?;
    let message = message.trim();
    let message = if message.is_empty() {
        format!("field-sop: record {} run(s)", repo.discover().runs.len())
    } else {
        message.to_owned()
    };
    let report = git::commit_and_push(repo.root(), &settings.remote, &message).map_err(text)?;
    Ok(PushResult {
        branch: report.branch,
        remote: report.remote,
        changed: report.changed,
        commit: report.commit,
        up_to_date: report.up_to_date,
        log: report.log,
    })
}

// ------------------------------------------------------------------ helpers

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn rows(state: &AppState) -> Reply<Vec<SettingRow>> {
    let settings = state.settings()?;
    Ok(sop_core::settings::KEYS
        .iter()
        .map(|(key, description)| SettingRow {
            key: (*key).to_owned(),
            value: settings.get(key),
            description: (*description).to_owned(),
            in_repository: sop_core::settings::is_repo_key(key),
        })
        .collect())
}

fn build_project(state: &AppState) -> Reply<ProjectView> {
    let repo = state.repo()?;
    let loaded = project::load(&repo).map_err(text)?;
    let path = repo
        .root()
        .join(sop_core::vocab::PROJECT_FILE)
        .display()
        .to_string();
    let fields = project::FIELDS
        .iter()
        .map(|(key, description)| ProjectField {
            key: (*key).to_owned(),
            value: loaded
                .as_ref()
                .and_then(|loaded| loaded.doc.front.str(key).flatten())
                .map(str::to_owned),
            description: (*description).to_owned(),
        })
        .collect();
    Ok(ProjectView { path, fields })
}

fn build_status(state: &AppState) -> Reply<Status> {
    let repo: Repo = state.repo()?;
    let settings = state.settings()?;
    let loaded = project::load(&repo).map_err(text)?;
    let front = loaded.as_ref().map(|loaded| &loaded.doc.front);
    let state_git = git::state(repo.root(), &settings.remote);
    let found = repo.discover();
    let validation = validate::validate_repository(&repo).report;

    Ok(Status {
        working_copy: repo.root().display().to_string(),
        settings_path: state.settings_path().display().to_string(),
        project_title: front
            .and_then(|front| front.str("title").flatten())
            .map(str::to_owned),
        project_id: front
            .and_then(|front| front.str("project_id").flatten())
            .map(str::to_owned),
        institution: front
            .and_then(|front| front.str("institution").flatten())
            .map(str::to_owned),
        git: GitInfo {
            is_repository: state_git.is_repository,
            branch: state_git.branch,
            remote: state_git.remote,
            url: state_git.url,
            url_has_credentials: state_git.url_has_credentials,
            dirty: state_git.dirty,
            ahead: state_git.ahead,
            behind: state_git.behind,
        },
        content: ContentCounts {
            procedures: found.procedures.len(),
            checklists: found.checklists.len(),
            runs: found.runs.len(),
            help: found.help.len(),
            log_directories: found.log_dirs.len(),
        },
        validation: ValidationCounts {
            errors: validation.error_count(),
            warnings: validation.warning_count(),
        },
    })
}
