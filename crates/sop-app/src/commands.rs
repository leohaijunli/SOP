//! The operations the window can ask for.
//!
//! Every edit goes through `sop_repo::authoring`, which checks the result before writing
//! it and undoes it if the repository as a whole stops validating. Nothing here decides
//! whether an edit is allowed; it only carries the answer back, including the report
//! when the answer is no.

use sop_core::authoring::{ItemRef, Move};
use sop_core::RunEvent;
use sop_repo::{Repo, authoring, git, manifest, project, run, validate};
use tauri::State;

use crate::api::{
    CaptureInput, ContentCounts, GitInfo, ProjectField, ProjectView, RunAttachmentView,
    RunCaptureView, RunStepView, RunView, SettingRow, Status, StepInput, StepPatch,
};
use crate::state::AppState;

type Reply<T> = Result<T, String>;

// ------------------------------------------------------------------- reading

#[tauri::command]
pub fn manifest_json(state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    Ok(manifest::build(&repo).to_json())
}

#[tauri::command]
pub fn status(state: State<'_, AppState>) -> Reply<Status> {
    build_status(&state)
}

#[tauri::command]
pub fn validation_report(state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    Ok(validate::validate_repository(&repo).report.render(repo.root()))
}

/// The procedures that can be included in a checklist, and whether it already has them.
#[tauri::command]
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

#[tauri::command]
pub fn settings_rows(state: State<'_, AppState>) -> Reply<Vec<SettingRow>> {
    rows(&state)
}

#[tauri::command]
pub fn settings_path(state: State<'_, AppState>) -> Reply<String> {
    Ok(state.settings_path().display().to_string())
}

#[tauri::command]
pub fn settings_set(key: String, value: String, state: State<'_, AppState>) -> Reply<Vec<SettingRow>> {
    let mut settings = state.settings()?;
    settings.set(&key, &value).map_err(text)?;
    state.save(&settings)?;
    rows(&state)
}

#[tauri::command]
pub fn settings_unset(key: String, state: State<'_, AppState>) -> Reply<Vec<SettingRow>> {
    let mut settings = state.settings()?;
    settings.unset(&key).map_err(text)?;
    state.save(&settings)?;
    rows(&state)
}

#[tauri::command]
pub fn open_repository(repo: String, state: State<'_, AppState>) -> Reply<Status> {
    state.open(&repo)?;
    build_status(&state)
}

#[tauri::command]
pub fn remote_url(state: State<'_, AppState>) -> Reply<Option<String>> {
    let repo = state.repo()?;
    let settings = state.settings()?;
    Ok(git::state(repo.root(), &settings.remote).url)
}

#[tauri::command]
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

#[tauri::command]
pub fn project_fields(state: State<'_, AppState>) -> Reply<ProjectView> {
    build_project(&state)
}

#[tauri::command]
pub fn project_set(key: String, value: String, state: State<'_, AppState>) -> Reply<ProjectView> {
    let repo = state.repo()?;
    project::set(&repo, &key, &value).map_err(text)?;
    build_project(&state)
}

// ------------------------------------------------------------------- editing

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
pub fn step_remove(file: String, id: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    let path = authoring::remove_step(&repo, &file, &id).map_err(text)?;
    Ok(repo.relpath(&path))
}

/// Move a step or an include marker past its neighbour. `up` is false for down.
#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
pub fn include_add(file: String, target: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    let path = authoring::add_include(&repo, &file, &target, None).map_err(text)?;
    Ok(repo.relpath(&path))
}

#[tauri::command]
pub fn include_remove(file: String, target: String, state: State<'_, AppState>) -> Reply<String> {
    let repo = state.repo()?;
    let path = authoring::remove_include(&repo, &file, &target).map_err(text)?;
    Ok(repo.relpath(&path))
}

// ------------------------------------------------------------------ running

#[tauri::command]
pub fn run_start(
    sop: String,
    run_id: String,
    operator: String,
    site: String,
    r#override: Option<String>,
    state: State<'_, AppState>,
) -> Reply<RunView> {
    let repo = state.repo()?;
    let loaded = run::start(&repo, &sop, &run_id, &operator, &site, r#override.as_deref())
        .map_err(text)?;
    Ok(build_run_view(&loaded, &run_id))
}

#[tauri::command]
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

#[tauri::command]
pub fn run_state(sop: String, run_id: String, state: State<'_, AppState>) -> Reply<RunView> {
    let repo = state.repo()?;
    let loaded = run::load(&repo, &sop, &run_id).map_err(text)?;
    Ok(build_run_view(&loaded, &run_id))
}

#[tauri::command]
pub fn run_end(sop: String, run_id: String, status: String, state: State<'_, AppState>) -> Reply<RunView> {
    let repo = state.repo()?;
    let loaded = run::end(&repo, &sop, &run_id, &status).map_err(text)?;
    Ok(build_run_view(&loaded, &run_id))
}

#[tauri::command]
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
                }
            })
            .collect();
        steps.push(RunStepView {
            id: def.id.clone(),
            title: def.title.clone(),
            prose: def.prose.clone(),
            severity: def.severity.clone(),
            kind: def.kind.clone(),
            status: status.as_str().to_owned(),
            reason: step_state.and_then(|s| s.reason.clone()),
            checkboxes: step_state.map(|s| s.checkboxes.clone()).unwrap_or_default(),
            captures,
            notes: step_state.map(|s| s.notes.clone()).unwrap_or_default(),
        });
    }
    RunView {
        sop: state.sop.clone().unwrap_or_default(),
        run_id: run_id.to_owned(),
        operator: state.operator.clone(),
        site: state.site.clone(),
        started: state.started.clone(),
        ended: state.ended.clone(),
        run_status: state.run_status.clone(),
        snapshot_sha256: state.snapshot_sha256.clone(),
        sop_version: state.sop_version.clone(),
        sop_commit: state.sop_commit.clone(),
        deviations_count: state.deviations(),
        steps,
        run_notes: state.run_notes.clone(),
        run_attachments: state
            .run_attachments
            .iter()
            .map(|a| RunAttachmentView {
                path: a.path.clone(),
                sha256: a.sha256.clone(),
                size: a.size,
            })
            .collect(),
        record_path: loaded.record_path.display().to_string(),
    }
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
    })
}
