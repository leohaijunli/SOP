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
        "Name of the git remote to use, for example origin. Not a URL.",
    ),
    (
        "branch",
        "Branch runs are recorded on. Absent means the current branch.",
    ),
    (
        "help-open",
        "Whether the help panel starts open: true or false.",
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
