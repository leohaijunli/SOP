//! Applying an edit to a content file: check it, write it, and take it back if the
//! repository as a whole objects.
//!
//! This is the only path by which the editor writes content, so the guarantees live here
//! rather than in each caller:
//!
//! 1. Only `procedures/` and `checklists/` are editable. A run record is evidence of what
//!    happened and is never edited afterwards; everything else in the repository is
//!    tooling.
//! 2. The candidate text is checked as the file it will become *before* it is written, so
//!    an edit that is wrong is refused with a message about the file the operator was
//!    editing.
//! 3. A full repository pass runs afterwards, and the edit is undone if it fails. Editing
//!    one file can break another: renaming a step id in a procedure orphans every run
//!    record that cites it, and the file being edited is the one that cannot see that.

use std::path::PathBuf;

use sop_core::authoring as edit_text;
use sop_core::authoring::{AuthoringError, CaptureDraft, ItemRef, Move, StepChanges, StepDraft};
use thiserror::Error;

use crate::{Repo, RepoError, atomic, validate};

#[derive(Debug, Error)]
pub enum EditError {
    #[error("{0}")]
    Authoring(#[from] AuthoringError),

    #[error("{0}")]
    Read(#[from] RepoError),

    #[error("{0}")]
    Write(#[from] atomic::WriteError),

    #[error("'{0}' is not a file the editor changes; only procedures/ and checklists/ are")]
    NotEditable(String),

    #[error("the change is not consistent, so nothing was written:\n{0}")]
    Rejected(String),

    #[error("the change broke something else in the repository and was undone:\n{0}")]
    Undone(String),
}

/// Resolve a repository-relative path the editor may change.
///
/// The prefix is checked on the resolved path rather than on what was typed, because
/// `procedures/../project.md` starts with the right directory name and does not end up
/// in it.
pub fn editable(repo: &Repo, relative: &str) -> Result<PathBuf, EditError> {
    let trimmed = relative.trim_start_matches("./");
    let path = repo.resolve(trimmed);
    let editable = ["procedures", "checklists"].iter().any(|directory| {
        let directory = repo.root().join(directory);
        path != directory && path.starts_with(directory)
    });
    if !editable || path.extension().is_none_or(|extension| extension != "md") {
        return Err(EditError::NotEditable(relative.to_owned()));
    }
    Ok(path)
}

/// Apply a change to one file, and keep it only if it stands on its own and the
/// repository still validates.
pub fn edit<F>(repo: &Repo, relative: &str, change: F) -> Result<PathBuf, EditError>
where
    F: FnOnce(&str) -> Result<String, AuthoringError>,
{
    let path = editable(repo, relative)?;
    let before = repo.read_text(&path)?;
    let after = change(&before)?;

    let single = validate::check_text(repo, &path, &after);
    if single.has_errors() {
        return Err(EditError::Rejected(single.render(repo.root())));
    }
    if after == before {
        return Ok(path);
    }

    atomic::write(&path, &after)?;
    let full = validate::validate_repository(repo);
    if full.report.has_errors() {
        atomic::write(&path, &before)?;
        return Err(EditError::Undone(full.report.render(repo.root())));
    }
    Ok(path)
}

/// Add a step, after another entry or at the end of the file.
pub fn add_step(
    repo: &Repo,
    relative: &str,
    draft: &StepDraft,
    after: Option<ItemRef>,
) -> Result<PathBuf, EditError> {
    edit(repo, relative, |text| {
        edit_text::add_step(text, draft, after.as_ref())
    })
}

/// Change the fields of a step.
pub fn update_step(
    repo: &Repo,
    relative: &str,
    id: &str,
    changes: &StepChanges,
) -> Result<PathBuf, EditError> {
    edit(repo, relative, |text| {
        edit_text::set_step(text, id, changes)
    })
}

/// Remove a step.
pub fn remove_step(repo: &Repo, relative: &str, id: &str) -> Result<PathBuf, EditError> {
    edit(repo, relative, |text| edit_text::remove_step(text, id))
}

/// Move a step or an include marker past its neighbour.
pub fn move_item(
    repo: &Repo,
    relative: &str,
    item: &ItemRef,
    direction: Move,
) -> Result<PathBuf, EditError> {
    edit(repo, relative, |text| {
        edit_text::move_item(text, item, direction)
    })
}

/// Add a capture to a step, or replace the capture with the same key.
pub fn set_capture(
    repo: &Repo,
    relative: &str,
    step_id: &str,
    draft: &CaptureDraft,
) -> Result<PathBuf, EditError> {
    edit(repo, relative, |text| {
        edit_text::set_capture(text, step_id, draft)
    })
}

/// Remove a capture from a step.
pub fn remove_capture(
    repo: &Repo,
    relative: &str,
    step_id: &str,
    key: &str,
) -> Result<PathBuf, EditError> {
    edit(repo, relative, |text| {
        edit_text::remove_capture(text, step_id, key)
    })
}

/// Add an include marker, which is how a checklist reuses a procedure's steps.
pub fn add_include(
    repo: &Repo,
    relative: &str,
    target: &str,
    after: Option<ItemRef>,
) -> Result<PathBuf, EditError> {
    edit(repo, relative, |text| {
        edit_text::add_include(text, target, after.as_ref())
    })
}

/// Remove an include marker.
pub fn remove_include(repo: &Repo, relative: &str, target: &str) -> Result<PathBuf, EditError> {
    edit(repo, relative, |text| {
        edit_text::remove_include(text, target)
    })
}
