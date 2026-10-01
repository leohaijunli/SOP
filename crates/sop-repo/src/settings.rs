//! Reading and writing the application settings file.
//!
//! This crate is the only one that touches the disk, so the file lives here while the
//! model itself stays in `sop-core` where it can be tested without one.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sop_core::Settings;
use thiserror::Error;

use crate::atomic;

/// Where a working copy keeps the settings that belong to it rather than to a machine.
///
/// The file is ordinary content: it is committed, reviewed, and diffed like everything
/// else in the repository. It holds only [`sop_core::settings::REPO_KEYS`] - today the
/// git remote name and branch - and it is not a second home for the machine's settings.
pub fn repo_path(root: &Path) -> PathBuf {
    root.join(".field-sop").join("settings.json")
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{path}: {message}")]
    Invalid { path: PathBuf, message: String },
}

/// Where the settings file lives, following the XDG base directory specification.
///
/// The app's own preferences are not repository content: putting them in the working
/// copy would commit them, and every operator's window state would conflict with every
/// other's.
pub fn default_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("field-sop").join("settings.json")
}

/// Load settings, falling back to the defaults when the file does not exist yet.
pub fn load(path: &Path) -> Result<Settings, SettingsError> {
    match fs::read_to_string(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(source) => Err(SettingsError::Io {
            path: path.to_path_buf(),
            source,
        }),
        Ok(text) => Settings::from_json(&text).map_err(|error| SettingsError::Invalid {
            path: path.to_path_buf(),
            message: error.to_string(),
        }),
    }
}

/// Write settings, creating the directory and never leaving a half-written file behind.
pub fn save(path: &Path, settings: &Settings) -> Result<(), SettingsError> {
    atomic::write(path, &settings.to_json()).map_err(|error| SettingsError::Io {
        path: error.path,
        source: error.source,
    })
}

/// Overlay the repository's own settings on `settings`.
///
/// A missing file is not an error: a repository that has never had a git setting changed
/// simply inherits the machine's. This is why the app can be pointed at a fresh clone and
/// still start.
pub fn apply_repo(root: &Path, settings: &mut Settings) -> Result<(), SettingsError> {
    let path = repo_path(root);
    match fs::read_to_string(&path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(SettingsError::Io { path, source }),
        Ok(text) => settings
            .apply_repo_json(&text)
            .map_err(|error| SettingsError::Invalid {
                path,
                message: error.to_string(),
            }),
    }
}

/// Write the repository-scoped settings into the working copy, creating `.field-sop/`.
///
/// Only the keys that belong to the repository are written, so a repository never ends up
/// carrying a machine's absolute path.
pub fn save_repo(root: &Path, settings: &Settings) -> Result<(), SettingsError> {
    let path = repo_path(root);
    atomic::write(&path, &settings.repo_json()).map_err(|error| SettingsError::Io {
        path: error.path,
        source: error.source,
    })
}
