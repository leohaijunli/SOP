//! The operations the window can ask for.
//!
//! Every edit goes through `sop_repo::authoring`, which checks the result before writing
//! it and undoes it if the repository as a whole stops validating. Nothing here decides
//! whether an edit is allowed; it only carries the answer back, including the report
//! when the answer is no.

use sop_core::authoring::{ItemRef, Move};
use sop_repo::{Repo, authoring, git, manifest, project, validate};
use tauri::State;

use crate::api::{
    CaptureInput, ContentCounts, GitInfo, ProjectField, ProjectView, SettingRow, Status,
    StepInput, StepPatch,
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
