//! Application settings.
//!
//! These are the app's own preferences: which working copy to open, which git remote to
//! talk to, and how the window should look. They are deliberately *not* content, and
//! deliberately not stored in the repository - a preference that gets committed is a
//! preference that conflicts.
//!
//! There is a hard rule in this module: **no settings value holds a credential or a
//! remote URL.** The URL lives in the repository's `.git/config`, where `git` put it and
//! where `git` can update it. The app stores the *name* of the remote and asks git for
//! the rest. An app that does not store a token cannot leak one, and a repository that
//! is copied to another machine carries no secret with it.
//!
//! This crate performs no I/O, so the file is read and written by `sop-repo`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use thiserror::Error;

/// The remote name used when settings do not say otherwise.
pub const DEFAULT_REMOTE: &str = "origin";

/// Settings that describe the repository rather than the machine.
///
/// These travel with the working copy in `.field-sop/settings.json`, so a clone on a
/// second laptop already knows which remote and branch to use - which is the whole point
/// of keeping them: an operator should not have to be told them, or remember them, on a
/// machine they have just set up.
///
/// Everything else is machine-local. An absolute working-copy path is wrong on every
/// machine but one, and the list of recently opened copies is a record of this laptop's
/// history, not the repository's.
pub const REPO_KEYS: &[&str] = &["remote", "branch"];

/// Whether `key` belongs to the repository rather than to this machine.
pub fn is_repo_key(key: &str) -> bool {
    REPO_KEYS.contains(&key)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SettingsError {
    #[error("'{0}' is not a setting; run `sop settings` to list them")]
    UnknownKey(String),

    #[error("'{0}' is maintained by the app and cannot be set by hand")]
    NotSettable(String),

    #[error("'{key}' {message}")]
    Invalid { key: String, message: String },

    #[error("settings are not valid JSON: {0}")]
    Json(String),
}

/// The settings a running app needs, with defaults for everything unset.
///
/// Unknown keys are kept, because a newer build may have written them and silently
/// dropping them on save would lose somebody's configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The working copy to open. An opaque string: this crate does not interpret paths.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    /// The name of the git remote to use. A name like `origin`, never a URL.
    pub remote: String,

    /// The branch a run is recorded on. Absent means "whatever the checkout is on".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// A separate repository that holds the `testplan/` tree. Absent means the working
    /// copy's own `testplan/`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub testcase_repo: Option<String>,

    /// Whether the help panel starts open.
    pub help_open: bool,

    /// Working copies opened before, most recent first.
    pub recent_repositories: Vec<String>,

    /// Keys written by another build. Kept verbatim.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Json>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            repository: None,
            remote: DEFAULT_REMOTE.to_owned(),
            branch: None,
            testcase_repo: None,
            help_open: true,
            recent_repositories: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

/// Every setting name, with a description, for `sop settings` and the settings page.
pub const KEYS: &[(&str, &str)] = &[
    (
        "repository",
        "Working copy to open. A path, or a path to create.",
    ),
    (
        "remote",
        "Name of the git remote to use, for example origin. Not a URL. Kept in the \
         repository, so a clone on another machine already has it.",
    ),
    (
        "branch",
        "Branch runs are recorded on. Absent means the current branch. Kept in the \
         repository, so a clone on another machine already has it.",
    ),
    (
        "help-open",
        "Whether the help panel starts open: true or false.",
    ),
    (
        "testcase-repo",
        "Separate repository holding the testplan/ tree. Absent uses the working copy.",
    ),
    (
        "recent-repositories",
        "Working copies opened before, most recent first.",
    ),
];

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_json(text: &str) -> Result<Self, SettingsError> {
        if text.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(text).map_err(|error| SettingsError::Json(error.to_string()))
    }

    pub fn to_json(&self) -> String {
        let mut out = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_owned());
        out.push('\n');
        out
    }

    /// Read one setting as text, for display.
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "repository" => self.repository.clone(),
            "remote" => Some(self.remote.clone()),
            "branch" => self.branch.clone(),
            "testcase-repo" => self.testcase_repo.clone(),
            "help-open" => Some(self.help_open.to_string()),
            "recent-repositories" => Some(self.recent_repositories.join(", ")),
            _ => self.extra.get(key).map(|value| value.to_string()),
        }
    }

    /// Set one setting from text, rejecting values that would put a URL or a secret in
    /// the app's configuration.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), SettingsError> {
        let value = value.trim();
        match key {
            "repository" => {
                if value.is_empty() {
                    return Err(SettingsError::Invalid {
                        key: key.to_owned(),
                        message: "must not be empty".to_owned(),
                    });
                }
                self.repository = Some(value.to_owned());
            }
            "remote" => {
                check_remote_name(value)?;
                self.remote = value.to_owned();
            }
            "branch" => {
                if value.is_empty() || value.chars().any(char::is_whitespace) {
                    return Err(SettingsError::Invalid {
                        key: key.to_owned(),
                        message: "must be a branch name with no spaces".to_owned(),
                    });
                }
                self.branch = Some(value.to_owned());
            }
            "testcase-repo" => {
                if value.is_empty() {
                    return Err(SettingsError::Invalid {
                        key: key.to_owned(),
                        message: "must not be empty".to_owned(),
                    });
                }
                self.testcase_repo = Some(value.to_owned());
            }
            "help-open" => {
                self.help_open = match value.to_ascii_lowercase().as_str() {
                    "true" | "yes" | "on" | "1" => true,
                    "false" | "no" | "off" | "0" => false,
                    _ => {
                        return Err(SettingsError::Invalid {
                            key: key.to_owned(),
                            message: "must be true or false".to_owned(),
                        });
                    }
                };
            }
            "recent-repositories" => return Err(SettingsError::NotSettable(key.to_owned())),
            other if self.extra.contains_key(other) => {
                self.extra
                    .insert(other.to_owned(), Json::String(value.to_owned()));
            }
            other => return Err(SettingsError::UnknownKey(other.to_owned())),
        }
        Ok(())
    }

    /// Remove a setting, returning it to its default.
    pub fn unset(&mut self, key: &str) -> Result<(), SettingsError> {
        match key {
            "repository" => self.repository = None,
            "remote" => self.remote = DEFAULT_REMOTE.to_owned(),
            "branch" => self.branch = None,
            "testcase-repo" => self.testcase_repo = None,
            "help-open" => self.help_open = true,
            "recent-repositories" => self.recent_repositories.clear(),
            other => {
                self.extra.remove(other);
            }
        }
        Ok(())
    }

    /// Record a working copy as the most recently opened one.
    pub fn remember(&mut self, repository: &str) {
        self.recent_repositories.retain(|known| known != repository);
        self.recent_repositories.insert(0, repository.to_owned());
        self.recent_repositories.truncate(10);
    }

    /// The repository-scoped settings as JSON, for `.field-sop/settings.json`.
    ///
    /// `branch` is written even when it is unset, as `null`, so that clearing a branch in
    /// the window is a change `git diff` shows rather than a key that silently vanishes.
    pub fn repo_json(&self) -> String {
        let mut map = serde_json::Map::new();
        map.insert("remote".to_owned(), Json::String(self.remote.clone()));
        map.insert(
            "branch".to_owned(),
            match &self.branch {
                Some(branch) => Json::String(branch.clone()),
                None => Json::Null,
            },
        );
        let mut out =
            serde_json::to_string_pretty(&Json::Object(map)).unwrap_or_else(|_| "{}".to_owned());
        out.push('\n');
        out
    }

    /// Apply a repository's own settings over this machine's.
    ///
    /// Only the keys the repository declares are touched, so the file is a set of
    /// overrides rather than a second complete copy. `"branch": null` clears the
    /// machine's branch, which is how a repository says "whatever the checkout is on".
    pub fn apply_repo_json(&mut self, text: &str) -> Result<(), SettingsError> {
        if text.trim().is_empty() {
            return Ok(());
        }
        let value: Json =
            serde_json::from_str(text).map_err(|error| SettingsError::Json(error.to_string()))?;
        let Some(map) = value.as_object() else {
            return Err(SettingsError::Json(
                "the repository settings must be a JSON object".to_owned(),
            ));
        };
        if let Some(remote) = map.get("remote").and_then(Json::as_str)
            && check_remote_name(remote).is_ok()
        {
            self.remote = remote.to_owned();
        }
        if let Some(branch) = map.get("branch") {
            match branch.as_str() {
                Some(name) if !name.is_empty() => self.branch = Some(name.to_owned()),
                Some(_) => {}
                None if branch.is_null() => self.branch = None,
                None => {}
            }
        }
        Ok(())
    }
}

/// A remote name is a name, not a location.
///
/// This is the guard that keeps a token out of the app's configuration: the moment a URL
/// is accepted here, somebody pastes `https://user:token@host/repo` into it.
fn check_remote_name(value: &str) -> Result<(), SettingsError> {
    let invalid = |message: &str| SettingsError::Invalid {
        key: "remote".to_owned(),
        message: message.to_owned(),
    };
    if value.is_empty() {
        return Err(invalid("must not be empty"));
    }
    if value.chars().any(char::is_whitespace) {
        return Err(invalid("must not contain spaces"));
    }
    if value.contains("://")
        || value.contains('@')
        || value.starts_with("git") && value.contains(':')
    {
        return Err(invalid(
            "looks like a URL, and the app does not store remote URLs or credentials; use \
             `sop remote <url>` so the URL lives in the repository's git config",
        ));
    }
    Ok(())
}
