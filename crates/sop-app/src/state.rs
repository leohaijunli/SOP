//! The explicitly selected directory holding the user's logs, plans, and records.
//!
//! The window holds one working copy at a time. Opening another one is a setting the
//! app remembers, so the next launch lands where the last session was: a field laptop
//! that forgets which repository it was recording into is a laptop that records into
//! the wrong one.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sop_core::Settings;
use sop_repo::{Repo, settings};

pub struct AppState {
    settings_path: PathBuf,
    root: Mutex<Option<PathBuf>>,
    last_export: Mutex<Option<PathBuf>>,
    log_metadata: Mutex<()>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkingDirectory {
    pub path: Option<String>,
    pub configured_path: Option<String>,
    pub recent: Vec<String>,
}

impl AppState {
    pub fn load() -> Result<Self, String> {
        let state = Self::from_settings(settings::default_path())?;
        // A launcher flag is also an explicit choice, never a current-dir fallback.
        if let Ok(path) = std::env::var("FIELD_SOP_WORKDIR") {
            state.open(&path)?;
        }
        Ok(state)
    }

    fn from_settings(settings_path: PathBuf) -> Result<Self, String> {
        let settings = settings::load(&settings_path).map_err(|error| error.to_string())?;
        // Never use the program's launch directory as a data directory. An unavailable
        // remembered folder returns to the picker rather than recording somewhere else.
        let root = settings
            .repository
            .as_deref()
            .and_then(|configured| resolve(configured).ok());
        Ok(Self {
            settings_path,
            root: Mutex::new(root),
            last_export: Mutex::new(None),
            log_metadata: Mutex::new(()),
        })
    }

    pub fn root(&self) -> Result<PathBuf, String> {
        self.root
            .lock()
            .map_err(|_| "the working directory is locked by another operation".to_owned())?
            .clone()
            .ok_or_else(|| {
                "Choose a working directory containing your logs and test plans first".to_owned()
            })
    }

    pub fn working_directory(&self) -> Result<WorkingDirectory, String> {
        let settings = settings::load(&self.settings_path).map_err(|error| error.to_string())?;
        let root = self
            .root
            .lock()
            .map_err(|_| "the working directory is locked".to_owned())?;
        Ok(WorkingDirectory {
            path: root.as_ref().map(|path| path.display().to_string()),
            configured_path: settings.repository,
            recent: settings.recent_repositories,
        })
    }

    pub fn annotate_log(&self, path: &str, remark: &str, tags: &[String]) -> Result<(), String> {
        let _guard = self
            .log_metadata
            .lock()
            .map_err(|_| "log metadata is locked".to_owned())?;
        sop_repo::workspace::annotate_log(&self.repo()?, path, remark, tags)
            .map_err(|error| error.to_string())
    }

    pub fn repo(&self) -> Result<Repo, String> {
        Ok(Repo::open_data_directory(self.root()?))
    }

    /// Remember the folder a summary export just wrote, so the window can open it and
    /// nothing else: only this path is ever opened for an export result.
    pub fn set_last_export(&self, path: PathBuf) -> Result<(), String> {
        let path = path.canonicalize().unwrap_or(path);
        let mut last = self
            .last_export
            .lock()
            .map_err(|_| "the export path is locked by another operation".to_owned())?;
        *last = Some(path);
        Ok(())
    }

    /// True when `path` is exactly the folder the most recent export wrote.
    pub fn is_last_export(&self, path: &Path) -> Result<bool, String> {
        let last = self
            .last_export
            .lock()
            .map_err(|_| "the export path is locked by another operation".to_owned())?;
        Ok(last.as_deref() == Some(path))
    }

    /// Plans are local files in the same data directory as the logs and notes.
    pub fn testcase_repo_root(&self) -> Result<PathBuf, String> {
        self.root()
    }

    /// The folder that holds the `testplan/` tree.
    pub fn testcase_root(&self) -> Result<PathBuf, String> {
        Ok(self.testcase_repo_root()?.join("testplan"))
    }

    pub fn settings_path(&self) -> &Path {
        &self.settings_path
    }

    /// Where the working copy keeps its own settings, which travel with the repository.
    pub fn settings_repo_path(&self) -> Result<PathBuf, String> {
        Ok(settings::repo_path(&self.root()?))
    }

    /// The machine's settings with the repository's own overlaid on top.
    pub fn settings(&self) -> Result<Settings, String> {
        let mut settings =
            settings::load(&self.settings_path).map_err(|error| error.to_string())?;
        if let Ok(root) = self.root() {
            settings::apply_repo(&root, &mut settings).map_err(|error| error.to_string())?;
        }
        Ok(settings)
    }

    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        settings::save(&self.settings_path, settings).map_err(|error| error.to_string())
    }

    /// Change one setting, in the file that owns it.
    ///
    /// `remote` and `branch` describe the repository, not the laptop, so they are written
    /// into the working copy where a clone on another machine picks them up. Everything
    /// else is machine-local and stays out of the repository.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        if sop_core::settings::is_repo_key(key) {
            let root = self.root()?;
            let mut settings = self.settings()?;
            settings
                .set(key, value)
                .map_err(|error| error.to_string())?;
            return settings::save_repo(&root, &settings).map_err(|error| error.to_string());
        }
        let mut settings =
            settings::load(&self.settings_path).map_err(|error| error.to_string())?;
        settings
            .set(key, value)
            .map_err(|error| error.to_string())?;
        self.save(&settings)
    }

    /// Return one setting to its default, in the file that owns it.
    pub fn unset_setting(&self, key: &str) -> Result<(), String> {
        if sop_core::settings::is_repo_key(key) {
            let root = self.root()?;
            let mut settings = self.settings()?;
            settings.unset(key).map_err(|error| error.to_string())?;
            return settings::save_repo(&root, &settings).map_err(|error| error.to_string());
        }
        let mut settings =
            settings::load(&self.settings_path).map_err(|error| error.to_string())?;
        settings.unset(key).map_err(|error| error.to_string())?;
        self.save(&settings)
    }

    /// Open another working copy, and remember it for next time.
    pub fn open(&self, path: &str) -> Result<PathBuf, String> {
        let root = resolve(path)?;
        // Existing plain directories (including empty ones) are valid. Records create
        // their own subdirectories lazily; opening a folder never initializes git.
        let mut settings =
            settings::load(&self.settings_path).map_err(|error| error.to_string())?;
        settings.repository = Some(root.display().to_string());
        settings.remember(&root.display().to_string());
        self.save(&settings)?;
        let mut current = self
            .root
            .lock()
            .map_err(|_| "the working copy is locked by another operation".to_owned())?;
        *current = Some(root.clone());
        *self
            .last_export
            .lock()
            .map_err(|_| "the export path is locked".to_owned())? = None;
        Ok(root)
    }
}

/// A path as the operator typed it: absolute, or relative to where the app was started.
fn resolve(path: &str) -> Result<PathBuf, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("the path is empty".to_owned());
    }
    let expanded = if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\"))
    {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| "the home directory is not set".to_owned())?;
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(path)
    };
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(expanded)
    };
    if !absolute.is_dir() {
        return Err(format!("{} is not a directory", absolute.display()));
    }
    Ok(absolute.canonicalize().unwrap_or(absolute))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "sop-directory-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_first_launch_requires_an_explicit_data_directory() {
        let scratch = Scratch::new();
        let state = AppState::from_settings(scratch.0.join("settings.json")).unwrap();
        assert!(state.root().is_err());
        assert!(state.working_directory().unwrap().path.is_none());
    }

    #[test]
    fn an_empty_non_git_directory_can_be_selected_and_is_remembered() {
        let scratch = Scratch::new();
        let path = scratch.0.join("data");
        std::fs::create_dir(&path).unwrap();
        let settings = scratch.0.join("settings.json");
        let state = AppState::from_settings(settings.clone()).unwrap();
        let opened = state.open(path.to_str().unwrap()).unwrap();
        assert_eq!(state.root().unwrap(), opened);
        assert!(!path.join(".git").exists());
        let restored = AppState::from_settings(settings).unwrap();
        assert_eq!(restored.root().unwrap(), opened);
    }

    #[test]
    fn an_unavailable_saved_directory_does_not_fall_back_to_the_code_directory() {
        let scratch = Scratch::new();
        let path = scratch.0.join("missing");
        let settings_path = scratch.0.join("settings.json");
        let settings = Settings {
            repository: Some(path.display().to_string()),
            ..Settings::default()
        };
        settings::save(&settings_path, &settings).unwrap();
        let state = AppState::from_settings(settings_path).unwrap();
        assert!(state.root().is_err());
        assert!(state.working_directory().unwrap().configured_path.is_some());
    }
}
