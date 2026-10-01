//! The repository's identity, read from and written to `project.md`.
//!
//! The project's name is content, not a preference. It is committed, so every operator
//! and every old run record agrees about which campaign the work belongs to; if it lived
//! in the app's settings, two laptops would disagree and the disagreement would never be
//! reviewed. The settings file holds the *path* to the working copy and the name of the
//! remote, and nothing else about the project.
//!
//! Editing is done field by field rather than by regenerating the file, so a person's
//! prose, key order, and comments survive. See `sop_core::front::set_field`.

use std::path::PathBuf;

use sop_core::{Document, check, front, vocab};
use thiserror::Error;

use crate::{Repo, RepoError, atomic, report::Report};

/// The fields `sop project set` accepts, with what each one is for.
///
/// The names are the front-matter keys themselves, so nothing has to be translated
/// between what a person types and what the file says.
pub const FIELDS: &[(&str, &str)] = &[
    (
        "title",
        "The name the app shows, and the name a reader of an old record sees.",
    ),
    (
        "project_id",
        "Stable id. Changing it is a rename, not a new project; prefer not to.",
    ),
    ("institution", "Who is responsible for the work."),
    ("lead", "Who to ask about the content."),
    ("started", "When the work started, as YYYY-MM-DD."),
    (
        "updated",
        "When the project file last changed, as YYYY-MM-DD.",
    ),
    (
        "summary",
        "One sentence describing the work. Carried into the manifest.",
    ),
    ("contact", "Where to write about the project."),
];

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("{0}")]
    Read(#[from] RepoError),

    #[error("there is no project.md in {root}; copy templates/project.md into place and edit it")]
    Missing { root: PathBuf },

    #[error("'{0}' is not a project field; run `sop project` to list them")]
    UnknownField(String),

    #[error("'{key}' {message}")]
    Invalid { key: String, message: String },

    #[error("{0}")]
    Write(#[from] atomic::WriteError),

    #[error("refused: the result would not pass validation, so nothing was written\n{0}")]
    Refused(String),
}

/// The project file, when the repository has one.
pub fn load(repo: &Repo) -> Result<Option<crate::Loaded>, ProjectError> {
    let path = repo.root().join(vocab::PROJECT_FILE);
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(repo.load(&path)?))
}

/// The project's title, for `sop status` and the window title.
///
/// A file that is not a project file, or has no title, yields `None` rather than a guess:
/// the validator's job is to complain about that, and a viewer's job is not to invent one.
pub fn title(repo: &Repo) -> Option<String> {
    let loaded = load(repo).ok().flatten()?;
    if loaded.doc.kind.as_deref() != Some("project") {
        return None;
    }
    let title = loaded.doc.front.str("title").flatten()?;
    (!title.trim().is_empty()).then(|| title.to_owned())
}

/// The project's stable id, or an empty string when it has none.
///
/// Run records are filed under it (`runs/<project_id>/...`), so an empty id means the
/// records sit directly under `runs/` as before.
pub fn id(repo: &Repo) -> String {
    let loaded = load(repo).ok().flatten();
    let Some(loaded) = loaded else {
        return String::new();
    };
    if loaded.doc.kind.as_deref() != Some("project") {
        return String::new();
    }
    loaded
        .doc
        .front
        .str("project_id")
        .flatten()
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

/// Rewrite one field of the project file.
///
/// The new text is parsed and checked *before* it is written, so a command that reports
/// success cannot have left behind a project file the validator rejects. Nothing else in
/// the file is touched.
pub fn set(repo: &Repo, key: &str, value: &str) -> Result<PathBuf, ProjectError> {
    let key =
        canonical_field(key).ok_or_else(|| ProjectError::UnknownField(key.trim().to_owned()))?;
    let value = value.trim();
    if value.is_empty() {
        return Err(ProjectError::Invalid {
            key: key.to_owned(),
            message: "must not be empty".to_owned(),
        });
    }
    if value.contains('\n') {
        return Err(ProjectError::Invalid {
            key: key.to_owned(),
            message: "must fit on one line".to_owned(),
        });
    }

    let path = repo.root().join(vocab::PROJECT_FILE);
    if !path.is_file() {
        return Err(ProjectError::Missing {
            root: repo.root().to_path_buf(),
        });
    }

    let loaded = repo.load(&path)?;
    let updated =
        front::set_field(&loaded.text, key, value).ok_or_else(|| ProjectError::Invalid {
            key: key.to_owned(),
            message: "could not be set: the file has no front matter block".to_owned(),
        })?;

    let doc = Document::parse(&updated).map_err(|source| RepoError::Parse {
        path: path.clone(),
        source,
    })?;
    let diagnostics = check::project_front_matter(&doc);
    if diagnostics.has_errors() {
        let mut report = Report::new();
        report.extend(&path, diagnostics);
        return Err(ProjectError::Refused(report.render(repo.root())));
    }

    atomic::write(&path, &updated)?;
    Ok(path)
}

/// Match a field name as typed, accepting the hyphenated spelling as a convenience.
fn canonical_field(key: &str) -> Option<&'static str> {
    let key = key.trim().replace('-', "_");
    FIELDS
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(name, _)| *name)
}
