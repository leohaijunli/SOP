//! Packaging a summary and every run's material into one self-contained folder.
//!
//! The operator picks a destination and the tool writes
//! `<dest>/export_<label>/` holding `summary.md`, `summary.csv`, each run's committed
//! record, and each run's whole folder (`events.jsonl`, `record.md`, `notes.md`, `logs/`,
//! `photos/`, `attachments/`), plus a `MANIFEST.sha256`. A half-written export is worse
//! than none, so the package is built in a `.partial` directory and renamed into place
//! only once every file is copied; a failure removes the partial and reports why.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::summary::{self, FmtTime, SummaryMeta};
use crate::{Repo, git, project};

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("{0}")]
    Repo(#[from] crate::RepoError),
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: cannot be written: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("'{0}' is not a valid export label; use only letters, digits, '-', and '_'")]
    BadLabel(String),
    #[error("{0}")]
    Summary(String),
}

/// What an export produced: where it landed and what it holds.
#[derive(Debug, Clone)]
pub struct ExportReport {
    /// The folder that was written.
    pub dir: PathBuf,
    /// How many runs were packaged.
    pub runs: usize,
    /// How many files were copied (excluding the two summary files and the manifest).
    pub files: usize,
    /// Total bytes copied.
    pub bytes: u64,
    /// Paths that were skipped (a symlink, or a special file).
    pub skipped: Vec<String>,
    /// Runs that could not be found, by `sop/run_id`.
    pub failed: Vec<String>,
}

/// The document header for an export, read from the working copy.
pub fn repo_meta(repo: &Repo, exported: String) -> SummaryMeta {
    let git = git::state(repo.root(), sop_core::settings::DEFAULT_REMOTE);
    SummaryMeta {
        project: project::title(repo).unwrap_or_default(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        git_commit: git::current_commit(repo.root()),
        git_dirty: git.dirty,
        exported,
    }
}

/// Export a summary and the runs into `<dest>/export_<label>/`.
///
/// `runs` is the set to export as `(sop_id, run_id)`; `None` exports every run. `label`
/// is the timestamp tag the caller chose (it must be `[0-9A-Za-z_-]`), and `fmt_time`
/// turns stored UTC timestamps into the operator's local time in the summary.
pub fn export_package(
    repo: &Repo,
    testcases: &Repo,
    dest: &Path,
    label: &str,
    fmt_time: FmtTime<'_>,
    meta: &SummaryMeta,
    runs: Option<&[(String, String)]>,
) -> Result<ExportReport, ExportError> {
    if !valid_label(label) {
        return Err(ExportError::BadLabel(label.to_owned()));
    }
    fs::create_dir_all(dest).map_err(|source| io(dest, source))?;
    let target = unique_target(dest, label);
    let partial = dest.join(format!("export_{label}.partial"));
    if partial.exists() {
        fs::remove_dir_all(&partial).map_err(|source| io(&partial, source))?;
    }

    let mut report = ExportReport {
        dir: target,
        runs: 0,
        files: 0,
        bytes: 0,
        skipped: Vec::new(),
        failed: Vec::new(),
    };
    if let Err(error) = fill(&mut report, repo, testcases, &partial, fmt_time, meta, runs) {
        let _ = fs::remove_dir_all(&partial);
        return Err(error);
    }
    if let Err(error) = write_manifest(&partial) {
        let _ = fs::remove_dir_all(&partial);
        return Err(error);
    }
    fs::rename(&partial, &report.dir).map_err(|source| io(&report.dir, source))?;
    Ok(report)
}

fn fill(
    report: &mut ExportReport,
    repo: &Repo,
    testcases: &Repo,
    partial: &Path,
    fmt_time: FmtTime<'_>,
    meta: &SummaryMeta,
    runs: Option<&[(String, String)]>,
) -> Result<(), ExportError> {
    fs::create_dir_all(partial).map_err(|source| io(partial, source))?;

    // The summary is written first, from the same data the copy is of, so its links line
    // up with the folders beside it.
    let markdown = summary::markdown(repo, testcases, fmt_time, meta);
    write_file(&partial.join("summary.md"), markdown.as_bytes())?;
    let csv = summary::csv(repo, testcases, fmt_time, meta);
    write_file(&partial.join("summary.csv"), csv.as_bytes())?;

    let listed: Vec<(String, String)> = match runs {
        Some(list) => list.to_vec(),
        None => crate::run::all_runs(repo),
    };
    for (sop, run_id) in listed {
        if !sop_core::vocab::is_valid_id(&sop) || !sop_core::vocab::is_valid_id(&run_id) {
            report.failed.push(format!("{sop}/{run_id}"));
            continue;
        }
        let out_runs = partial
            .join("runs")
            .join(crate::run::run_subdir(repo, &sop));
        fs::create_dir_all(&out_runs).map_err(|source| io(&out_runs, source))?;

        let record_src = repo.resolve(&format!("runs/{}/{run_id}.md", crate::run::run_subdir(repo, &sop)));
        let dir_src = repo.resolve(&format!("runs/{}/{run_id}", crate::run::run_subdir(repo, &sop)));
        let mut found = false;
        if record_src.is_file() {
            copy_file(&record_src, &out_runs.join(format!("{run_id}.md")), report)?;
            found = true;
        }
        if dir_src.is_dir() {
            copy_tree(&dir_src, &out_runs.join(&run_id), report)?;
            found = true;
        }
        if found {
            report.runs += 1;
        } else {
            report.failed.push(format!("{sop}/{run_id}"));
        }
    }
    Ok(())
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), ExportError> {
    fs::write(path, bytes).map_err(|source| io(path, source))
}

/// Copy one regular file and check it arrived whole.
fn copy_file(from: &Path, to: &Path, report: &mut ExportReport) -> Result<(), ExportError> {
    fs::copy(from, to).map_err(|source| io(from, source))?;
    let source_len = fs::metadata(from).map_err(|source| io(from, source))?.len();
    let copied_len = fs::metadata(to).map_err(|source| io(to, source))?.len();
    if source_len != copied_len {
        return Err(ExportError::Read {
            path: to.to_path_buf(),
            source: std::io::Error::other(format!(
                "copied {copied_len} bytes, expected {source_len}"
            )),
        });
    }
    report.files += 1;
    report.bytes += copied_len;
    Ok(())
}

/// Copy a directory tree, creating directories as needed. Symbolic links and special
/// files are skipped rather than followed, so an export cannot leave the run it names.
fn copy_tree(from: &Path, to: &Path, report: &mut ExportReport) -> Result<(), ExportError> {
    fs::create_dir_all(to).map_err(|source| io(to, source))?;
    for entry in fs::read_dir(from).map_err(|source| io(from, source))? {
        let entry = entry.map_err(|source| io(from, source))?;
        let source = entry.path();
        let destination = to.join(entry.file_name());
        let meta = fs::symlink_metadata(&source).map_err(|error| io(&source, error))?;
        if meta.file_type().is_symlink() {
            report.skipped.push(source.display().to_string());
        } else if meta.is_dir() {
            copy_tree(&source, &destination, report)?;
        } else if meta.is_file() {
            copy_file(&source, &destination, report)?;
        } else {
            report.skipped.push(source.display().to_string());
        }
    }
    Ok(())
}

/// Write `MANIFEST.sha256`: the sha256 of every file in the package, relative paths in a
/// stable order, so a copy can be checked later.
fn write_manifest(root: &Path) -> Result<(), ExportError> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    let mut out = String::new();
    for relative in files {
        let bytes = fs::read(root.join(&relative)).map_err(|source| ExportError::Read {
            path: root.join(&relative),
            source,
        })?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let digest: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        out.push_str(&format!("{digest}  {relative}\n"));
    }
    write_file(&root.join("MANIFEST.sha256"), out.as_bytes())
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), ExportError> {
    for entry in fs::read_dir(dir).map_err(|source| io(dir, source))? {
        let entry = entry.map_err(|source| io(dir, source))?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|error| io(&path, error))?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            collect_files(root, &path, out)?;
        } else if meta.is_file() {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if relative != "MANIFEST.sha256" {
                out.push(relative);
            }
        }
    }
    Ok(())
}

/// The folder to write: `export_<label>`, with `-2`, `-3`, ... when that exists.
fn unique_target(dest: &Path, label: &str) -> PathBuf {
    let base = dest.join(format!("export_{label}"));
    if !base.exists() {
        return base;
    }
    for suffix in 2u32.. {
        let candidate = dest.join(format!("export_{label}-{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("a free export folder name always exists")
}

fn valid_label(label: &str) -> bool {
    !label.is_empty()
        && label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn io(path: &Path, source: std::io::Error) -> ExportError {
    ExportError::Io {
        path: path.to_path_buf(),
        source,
    }
}
