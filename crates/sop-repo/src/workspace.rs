//! Loose logs in the explicitly selected data directory.
//!
//! Files under `logs/` can be managed without first creating a test record. Remarks
//! and tags live in a small sidecar; no log is renamed, rewritten, or moved.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{Repo, atomic, run};

pub const REMARKS_FILE: &str = ".field-sop/log-remarks.json";

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: log remarks are not valid JSON: {message}")]
    InvalidRemarks { path: PathBuf, message: String },
    #[error("'{0}' is not a log inside the selected working directory's logs/ folder")]
    UnsafePath(String),
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Remark {
    #[serde(default)]
    remark: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    updated_at: String,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Remarks {
    #[serde(default)]
    files: BTreeMap<String, Remark>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

fn read_remarks(repo: &Repo) -> Result<Remarks, WorkspaceError> {
    let path = repo.resolve(REMARKS_FILE);
    if path.exists() {
        let root = repo
            .root()
            .canonicalize()
            .map_err(|source| WorkspaceError::Io {
                path: repo.root().to_path_buf(),
                source,
            })?;
        let actual = path.canonicalize().map_err(|source| WorkspaceError::Io {
            path: path.clone(),
            source,
        })?;
        if !actual.starts_with(root) {
            return Err(WorkspaceError::UnsafePath(REMARKS_FILE.to_owned()));
        }
    }
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).map_err(|error| WorkspaceError::InvalidRemarks {
            path,
            message: error.to_string(),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Remarks::default()),
        Err(source) => Err(WorkspaceError::Io { path, source }),
    }
}

fn collect_logs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), WorkspaceError> {
    if !dir.exists() {
        return Ok(());
    }
    if fs::symlink_metadata(dir)
        .map_err(|source| WorkspaceError::Io {
            path: dir.to_path_buf(),
            source,
        })?
        .file_type()
        .is_symlink()
    {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|source| WorkspaceError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| WorkspaceError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|source| WorkspaceError::Io {
            path: path.clone(),
            source,
        })?;
        // Never follow a link out of the chosen directory or into a recursive loop.
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            collect_logs(&path, out)?;
        } else if kind.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

pub fn logs(repo: &Repo) -> Result<Vec<run::RunFile>, WorkspaceError> {
    let remarks = read_remarks(repo)?;
    let mut paths = Vec::new();
    collect_logs(&repo.resolve("logs"), &mut paths)?;
    paths.sort();
    Ok(paths
        .into_iter()
        .map(|path| {
            let relative = repo.relpath(&path);
            let remark = remarks.files.get(&relative);
            run::RunFile {
                kind: "log".to_owned(),
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                path: relative,
                size: fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0),
                file_time: run::file_modified(&path).unwrap_or_default(),
                time_source: "modified".to_owned(),
                added_at: String::new(),
                post_run: false,
                step: None,
                missing: false,
                remark: remark.map(|value| value.remark.clone()).unwrap_or_default(),
                tags: remark.map(|value| value.tags.clone()).unwrap_or_default(),
                remark_at: remark
                    .map(|value| value.updated_at.clone())
                    .filter(|at| !at.is_empty()),
            }
        })
        .collect())
}

/// Resolve a loose log for annotations or opening its containing folder.
pub fn log_path(repo: &Repo, relative: &str) -> Result<PathBuf, WorkspaceError> {
    let root = repo
        .root()
        .canonicalize()
        .map_err(|source| WorkspaceError::Io {
            path: repo.root().to_path_buf(),
            source,
        })?;
    let logs = repo
        .resolve("logs")
        .canonicalize()
        .map_err(|source| WorkspaceError::Io {
            path: repo.resolve("logs"),
            source,
        })?;
    let target = repo
        .resolve(relative)
        .canonicalize()
        .map_err(|source| WorkspaceError::Io {
            path: repo.resolve(relative),
            source,
        })?;
    if !logs.starts_with(root) || !target.starts_with(logs) || !target.is_file() {
        return Err(WorkspaceError::UnsafePath(relative.to_owned()));
    }
    Ok(target)
}

/// The desktop serializes sidecar edits; the write itself is atomic and preserves
/// unknown fields. Invalid existing JSON is reported, never replaced with an empty map.
pub fn annotate_log(
    repo: &Repo,
    relative: &str,
    remark: &str,
    tags: &[String],
) -> Result<(), WorkspaceError> {
    let target = log_path(repo, relative)?;
    let root = repo
        .root()
        .canonicalize()
        .map_err(|source| WorkspaceError::Io {
            path: repo.root().to_path_buf(),
            source,
        })?;
    let key = target
        .strip_prefix(&root)
        .map_err(|_| WorkspaceError::UnsafePath(relative.to_owned()))?
        .to_string_lossy()
        .replace('\\', "/");
    let mut remarks = read_remarks(repo)?;
    let parent = repo.resolve(".field-sop");
    if parent.exists()
        && !parent
            .canonicalize()
            .map_err(|source| WorkspaceError::Io {
                path: parent.clone(),
                source,
            })?
            .starts_with(&root)
    {
        return Err(WorkspaceError::UnsafePath(REMARKS_FILE.to_owned()));
    }
    let value = remarks.files.entry(key).or_default();
    value.remark = remark.trim().to_owned();
    value.tags = sop_core::run::normalize_tags(tags);
    value.updated_at = sop_core::now_utc_rfc3339();
    let path = repo.resolve(REMARKS_FILE);
    let text =
        serde_json::to_string_pretty(&remarks).map_err(|error| WorkspaceError::InvalidRemarks {
            path: path.clone(),
            message: error.to_string(),
        })?;
    atomic::write(&path, &(text + "\n")).map_err(|error| WorkspaceError::Io {
        path,
        source: error.source,
    })?;
    Ok(())
}

/// Copy a chosen log into `logs/`, retaining its source timestamp. Same-name conflicts
/// get a suffix; an identical existing file is reused. Large logs are streamed.
pub fn import_log(repo: &Repo, source: &Path) -> Result<String, WorkspaceError> {
    let io = |path: &Path, source| WorkspaceError::Io {
        path: path.to_path_buf(),
        source,
    };
    let root = repo
        .root()
        .canonicalize()
        .map_err(|source| io(repo.root(), source))?;
    let logs = repo.resolve("logs");
    fs::create_dir_all(&logs).map_err(|source| io(&logs, source))?;
    let logs = logs.canonicalize().map_err(|source| io(&logs, source))?;
    if !logs.starts_with(&root) {
        return Err(WorkspaceError::UnsafePath("logs/".to_owned()));
    }
    let name = source
        .file_name()
        .ok_or_else(|| WorkspaceError::UnsafePath(source.display().to_string()))?;
    let stem = source.file_stem().unwrap_or(name).to_string_lossy();
    let extension = source
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    let hash = file_hash(source)?;
    for suffix in 1u32.. {
        let name = if suffix == 1 {
            name.to_os_string()
        } else {
            format!("{stem}-{suffix}{extension}").into()
        };
        let destination = logs.join(name);
        if destination.is_file() && file_hash(&destination)? == hash {
            return Ok(destination
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"));
        }
        let mut out = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(io(&destination, source)),
        };
        let result = fs::File::open(source)
            .and_then(|mut input| std::io::copy(&mut input, &mut out))
            .and_then(|_| out.sync_all());
        if let Err(error) = result {
            drop(out);
            let _ = fs::remove_file(&destination); // only the file reserved by this call
            return Err(io(source, error));
        }
        if let Ok(modified) = fs::metadata(source).and_then(|meta| meta.modified()) {
            let _ = out.set_times(fs::FileTimes::new().set_modified(modified));
        }
        return Ok(destination
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/"));
    }
    unreachable!("a free suffixed log name exists")
}

fn file_hash(path: &Path) -> Result<Vec<u8>, WorkspaceError> {
    let mut file = fs::File::open(path).map_err(|source| WorkspaceError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|source| WorkspaceError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest.finalize().to_vec())
}
