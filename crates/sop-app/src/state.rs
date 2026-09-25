//! Which working copy this window is looking at.
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
    root: Mutex<PathBuf>,
}

impl AppState {
    pub fn load() -> Result<Self, String> {
        let settings_path = settings::default_path();
        let settings = settings::load(&settings_path).map_err(|error| error.to_string())?;
        let root = match settings.repository.as_deref() {
            Some(configured) => resolve(configured)?,
            None => std::env::current_dir().map_err(|error| error.to_string())?,
        };
        Ok(Self {
            settings_path,
            root: Mutex::new(root),
        })
    }

    pub fn root(&self) -> Result<PathBuf, String> {
        self.root
            .lock()
            .map(|root| root.clone())
            .map_err(|_| "the working copy is locked by another operation".to_owned())
    }

    pub fn repo(&self) -> Result<Repo, String> {
        Ok(Repo::open(self.root()?))
    }

    pub fn settings_path(&self) -> &Path {
        &self.settings_path
    }

    pub fn settings(&self) -> Result<Settings, String> {
        settings::load(&self.settings_path).map_err(|error| error.to_string())
    }

    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        settings::save(&self.settings_path, settings).map_err(|error| error.to_string())
    }

    /// Open another working copy, and remember it for next time.
    pub fn open(&self, path: &str) -> Result<PathBuf, String> {
        let root = resolve(path)?;
        if !looks_like_repository(&root) {
            return Err(format!(
                "{} does not look like a field-sop repository: no project.md, \
                 procedures/, or checklists/ in it",
                root.display()
            ));
        }
        let mut settings = self.settings()?;
        settings.repository = Some(root.display().to_string());
        settings.remember(&root.display().to_string());
        self.save(&settings)?;
        let mut current = self
            .root
            .lock()
            .map_err(|_| "the working copy is locked by another operation".to_owned())?;
        *current = root.clone();
        Ok(root)
    }
}

/// A path as the operator typed it: absolute, or relative to where the app was started.
fn resolve(path: &str) -> Result<PathBuf, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("the path is empty".to_owned());
    }
    let expanded = if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_owned())?;
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

/// The content the app needs to find before it will record anything.
fn looks_like_repository(root: &Path) -> bool {
    root.join(sop_core::vocab::PROJECT_FILE).is_file()
        || root.join("procedures").is_dir()
        || root.join("checklists").is_dir()
}
