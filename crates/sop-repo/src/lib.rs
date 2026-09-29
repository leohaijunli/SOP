//! Filesystem side of the content format: discovery, loading, include resolution.

pub mod atomic;
pub mod authoring;
pub mod git;
pub mod manifest;
pub mod project;
pub mod report;
pub mod run;
pub mod settings;
pub mod summary;
pub mod testplan;
pub mod validate;

use std::fs;
use std::io;
use std::path::Component;
use std::path::{Path, PathBuf};

use sop_core::{Document, Step};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{path}: {message}")]
    Text { path: PathBuf, message: String },
    #[error("{path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: sop_core::ParseError,
    },
}

impl RepoError {
    pub fn path(&self) -> &Path {
        match self {
            RepoError::Io { path, .. }
            | RepoError::Text { path, .. }
            | RepoError::Parse { path, .. } => path,
        }
    }

    /// The line the problem is attributed to, when it can be attributed to one.
    pub fn line(&self) -> Option<usize> {
        match self {
            RepoError::Parse { source, .. } => source.line(),
            RepoError::Io { .. } | RepoError::Text { .. } => None,
        }
    }
}

/// A loaded content file, with the text kept for write-back and hashing.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub path: PathBuf,
    /// Repository-relative, with forward slashes, for messages and comparisons.
    pub relpath: String,
    pub text: String,
    pub doc: Document,
}

#[derive(Debug, Clone)]
pub struct Repo {
    root: PathBuf,
}

/// Discovered files, in a stable order.
#[derive(Debug, Default, Clone)]
pub struct Discovery {
    /// The one project file, when the repository has one.
    pub project: Option<PathBuf>,
    pub procedures: Vec<PathBuf>,
    pub checklists: Vec<PathBuf>,
    /// Each directory under `testplan/` is one test plan.
    pub testplan_dirs: Vec<PathBuf>,
    pub runs: Vec<PathBuf>,
    pub inbox: Vec<PathBuf>,
    pub help: Vec<PathBuf>,
    pub log_dirs: Vec<PathBuf>,
}

impl Repo {
    pub fn open(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn relpath(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    pub fn resolve(&self, relative: &str) -> PathBuf {
        normalize(
            &self
                .root
                .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR)),
        )
    }

    /// Read a content file as UTF-8 text, rejecting the encodings the format forbids.
    ///
    /// A byte order mark and invalid UTF-8 are reported as content problems rather than
    /// as I/O problems, because they are mistakes in the file, not failures to read it.
    pub fn read_text(&self, path: &Path) -> Result<String, RepoError> {
        let bytes = fs::read(path).map_err(|source| RepoError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        decode_text(&bytes).map_err(|message| RepoError::Text {
            path: path.to_path_buf(),
            message,
        })
    }

    pub fn load(&self, path: &Path) -> Result<Loaded, RepoError> {
        let text = self.read_text(path)?;
        let doc = Document::parse(&text).map_err(|source| RepoError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Loaded {
            path: path.to_path_buf(),
            relpath: self.relpath(path),
            text,
            doc,
        })
    }

    pub fn discover(&self) -> Discovery {
        let mut out = Discovery {
            project: {
                let path = self.root.join(sop_core::vocab::PROJECT_FILE);
                path.is_file().then_some(path)
            },
            procedures: markdown_in(&self.root.join("procedures")),
            checklists: markdown_in(&self.root.join("checklists")),
            help: markdown_in(&self.root.join("help")),
            inbox: markdown_in(&self.root.join("runs").join("_inbox")),
            ..Discovery::default()
        };

        let runs_dir = self.root.join("runs");
        if let Ok(entries) = fs::read_dir(&runs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if path.file_name().is_some_and(|name| name == "_inbox") {
                    continue;
                }
                out.runs.extend(markdown_in(&path));
            }
        }
        out.runs.sort();

        let logs_root = self.root.join("logs");
        if let Ok(entries) = fs::read_dir(&logs_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    out.log_dirs.push(path);
                }
            }
        }
        out.log_dirs.sort();

        let testplan_root = self.root.join("testplan");
        if let Ok(entries) = fs::read_dir(&testplan_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    out.testplan_dirs.push(path);
                }
            }
        }
        out.testplan_dirs.sort();

        out
    }
}

/// Decode file bytes, rejecting the two encodings the format forbids.
pub fn decode_text(bytes: &[u8]) -> Result<String, String> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err("file starts with a UTF-8 BOM".to_owned());
    }
    String::from_utf8(bytes.to_vec()).map_err(|error| format!("file is not valid UTF-8: {error}"))
}

/// Resolve `.` and `..` without touching the filesystem.
///
/// The include rules compare paths as text, so `procedures/../docs/x.md` has to be
/// collapsed before the comparison, the way `os.path.normpath` does it.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if out.as_os_str().is_empty() {
                    out.push("..");
                } else {
                    let _ = out.pop();
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `.md` files directly inside `dir`, excluding `README.md`.
pub(crate) fn markdown_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "md")
                && path.file_name().is_some_and(|name| name != "README.md")
        })
        .collect();
    out.sort();
    out
}

/// A checklist with its include markers expanded, in document order.
pub struct ResolvedChecklist {
    pub steps: Vec<Step>,
    /// Include markers that could not be expanded: `(line, message)`.
    pub problems: Vec<(usize, String)>,
}

/// Expand a checklist's include markers in place, preserving document order.
///
/// Include markers position the included procedure's steps; local steps stay where they
/// are written. See `SPEC.md` section 7.
pub fn resolve_checklist(repo: &Repo, checklist: &Loaded) -> ResolvedChecklist {
    enum Item {
        Include(usize, String),
        Local(usize),
    }

    let mut items: Vec<Item> = Vec::new();
    for include in &checklist.doc.includes {
        items.push(Item::Include(include.line, include.target.clone()));
    }
    for index in 0..checklist.doc.steps.len() {
        items.push(Item::Local(index));
    }
    items.sort_by_key(|item| match item {
        Item::Include(line, _) => *line,
        Item::Local(index) => checklist.doc.steps[*index].line,
    });

    let mut steps: Vec<Step> = Vec::new();
    let mut problems: Vec<(usize, String)> = Vec::new();
    let procedures_root = normalize(&repo.root().join("procedures"));

    for item in items {
        match item {
            Item::Local(index) => {
                let mut step = checklist.doc.steps[index].clone();
                step.source = Some(checklist.relpath.clone());
                steps.push(step);
            }
            Item::Include(line, target) => {
                let path = if repo.resolve(&target).is_file() {
                    repo.resolve(&target)
                } else {
                    let parent = Path::new(&checklist.path)
                        .parent()
                        .unwrap_or_else(|| Path::new(&checklist.path));
                    parent.join(&target)
                };
                if !path.is_file() {
                    problems.push((line, format!("include target does not exist: {target}")));
                    continue;
                }
                if path.starts_with(&procedures_root) || !checklist.path.starts_with(repo.root()) {
                    // Allowed
                } else {
                    problems.push((line, format!("include target is not a procedure: {target}")));
                    continue;
                }
                match repo.load(&path) {
                    Err(error) => {
                        let where_ = match error.line() {
                            Some(number) => format!(" at line {number}"),
                            None => String::new(),
                        };
                        problems.push((
                            line,
                            format!("included procedure {target} is unreadable{where_}: {error}"),
                        ));
                    }
                    Ok(loaded) => {
                        for step in &loaded.doc.steps {
                            let mut step = step.clone();
                            step.source = Some(loaded.relpath.clone());
                            steps.push(step);
                        }
                    }
                }
            }
        }
    }

    ResolvedChecklist { steps, problems }
}

impl Repo {
    /// The file a run's `sop` id names today: `checklists/<id>.md`, or a test case under
    /// `testplan/` whose front matter carries that `sop_id`.
    ///
    /// A run started from a test case records the case's `sop_id`, not a path, so every
    /// reader that has to find the checklist again - the validator, drift, step-id
    /// resolution - goes through here.
    pub fn checklist_path(&self, sop_id: &str) -> Option<PathBuf> {
        let direct = self.root.join("checklists").join(format!("{sop_id}.md"));
        if direct.is_file() {
            return Some(direct);
        }
        for dir in self.discover().testplan_dirs {
            for path in markdown_in(&dir) {
                if path.file_name().is_some_and(|name| name == "plan.md") {
                    continue;
                }
                if let Ok(loaded) = self.load(&path)
                    && loaded.doc.front.str("sop_id").flatten() == Some(sop_id)
                {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Step ids of a checklist, resolved. Used by run records to check citations.
    pub fn checklist_step_ids(&self, sop_id: &str) -> Option<Vec<String>> {
        let path = self.checklist_path(sop_id)?;
        let loaded = self.load(&path).ok()?;
        let resolved = resolve_checklist(self, &loaded);
        Some(
            resolved
                .steps
                .iter()
                .filter_map(|step| step.id.clone())
                .collect(),
        )
    }

    /// Step ids of the checklist revision a run was recorded against, read from the
    /// snapshot frozen when it started.
    ///
    /// This is the set a record must be interpreted against (`SPEC.md` section 8): a
    /// checklist is allowed to change after a run, and the record must stay valid when
    /// it does. `None` means the run has no snapshot, which is only true of records
    /// written before runs became self-contained directories.
    pub fn run_snapshot_step_ids(&self, sop_id: &str, run_id: &str) -> Option<Vec<String>> {
        let path = self
            .root
            .join("runs")
            .join(sop_id)
            .join(run_id)
            .join("snapshot.md");
        let text = self.read_text(&path).ok()?;
        let doc = Document::parse(&text).ok()?;
        Some(
            doc.steps
                .iter()
                .filter_map(|step| step.id.clone())
                .collect(),
        )
    }

    /// The files each snapshot step declares it should produce, `(step id, outputs)`.
    ///
    /// The validator uses this to check a `complete` run actually collected each declared
    /// output as an attachment; a checklist edit after the run must not change the answer.
    pub fn run_snapshot_step_outputs(&self, sop_id: &str, run_id: &str) -> Option<Vec<(String, Vec<String>)>> {
        let path = self
            .root
            .join("runs")
            .join(sop_id)
            .join(run_id)
            .join("snapshot.md");
        let text = self.read_text(&path).ok()?;
        let doc = Document::parse(&text).ok()?;
        Some(
            doc.steps
                .iter()
                .filter_map(|step| step.id.clone().map(|id| (id, step.outputs.clone())))
                .collect(),
        )
    }
}
