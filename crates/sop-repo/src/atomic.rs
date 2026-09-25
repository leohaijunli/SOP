//! Writing a file so that a reader never sees half of it.
//!
//! The failures that matter here are the ones a field session produces: a laptop that
//! suspends, a battery that dies, a file copied to a USB stick while it is being written.
//! A truncated settings file is an annoyance; a truncated record of a morning's survey is
//! lost work. Writing to a sibling and renaming means a reader sees the old file or the
//! new one, never a fragment of either.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
#[error("{path}: {source}")]
pub struct WriteError {
    pub path: PathBuf,
    #[source]
    pub source: io::Error,
}

/// Write `text` by way of a sibling temporary file, then rename it into place.
pub fn write(path: &Path, text: &str) -> Result<(), WriteError> {
    if let Some(parent) = path.parent()
        && let Err(source) = fs::create_dir_all(parent)
    {
        return Err(WriteError {
            path: parent.to_path_buf(),
            source,
        });
    }

    let temporary = sibling_temporary(path);
    fs::write(&temporary, text).map_err(|source| WriteError {
        path: temporary.clone(),
        source,
    })?;
    fs::rename(&temporary, path).map_err(|source| WriteError {
        path: path.to_path_buf(),
        source,
    })
}

/// A temporary path beside the real one, so the rename cannot cross a filesystem.
fn sibling_temporary(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}
