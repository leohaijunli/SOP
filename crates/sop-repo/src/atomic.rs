//! Writing a file so that a reader never sees half of it.
//!
//! The failures that matter here are the ones a field session produces: a laptop that
//! suspends, a battery that dies, a file copied to a USB stick while it is being written.
//! A truncated settings file is an annoyance; a truncated record of a morning's survey is
//! lost work. Writing to a sibling and renaming means a reader sees the old file or the
//! new one, never a fragment of either.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
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
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent
        && let Err(source) = fs::create_dir_all(parent)
    {
        return Err(WriteError {
            path: parent.to_path_buf(),
            source,
        });
    }

    let temporary = sibling_temporary(path);
    // The rename is atomic for a *reader*, but only the physical write is durable across
    // a power cut: without the fsync a record can be renamed into place while still
    // sitting in the page cache, and come back empty or half-written.
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|source| WriteError {
            path: temporary.clone(),
            source,
        })?;
    file.write_all(text.as_bytes())
        .map_err(|source| WriteError {
            path: temporary.clone(),
            source,
        })?;
    file.sync_all().map_err(|source| WriteError {
        path: temporary.clone(),
        source,
    })?;
    drop(file);
    fs::rename(&temporary, path).map_err(|source| WriteError {
        path: path.to_path_buf(),
        source,
    })?;
    // Durably record the rename itself, so the directory cannot come back without the
    // entry. A platform that will not open a directory for sync is not fatal, so the
    // error is dropped rather than losing work already safely written.
    if let Some(parent) = parent
        && let Ok(directory) = File::open(parent)
    {
        let _ = directory.sync_all();
    }
    Ok(())
}

/// A temporary path beside the real one, so the rename cannot cross a filesystem.
///
/// The suffix carries the process id and a per-process counter: two writers (the app and
/// the CLI, or two threads) must not share a temporary, or one renames it away while the
/// other is still writing and the second rename fails with "no such file".
fn sibling_temporary(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(
        ".{}.{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    name.push(".tmp");
    path.with_file_name(name)
}
