//! Validating a whole repository against `SPEC.md` section 13.
//!
//! It decides which rules apply to which file, and which rules need the filesystem. The
//! rules themselves live in `sop_core::check` where they only need one document.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sop_core::{Diagnostic, Document, Value, check};

use crate::report::{Report, display_path};
use crate::{Loaded, Repo, RepoError, resolve_checklist};

/// Per-kind groups, classified by where a file sits in the repository.
///
/// Location, not front matter, decides which rules apply, so a file in the wrong place
/// is reported by the rule set for where it actually is.
#[derive(Debug, Default)]
struct Groups {
    project: Vec<PathBuf>,
    procedures: Vec<PathBuf>,
    checklists: Vec<PathBuf>,
    runs: Vec<PathBuf>,
    inbox: Vec<PathBuf>,
    help: Vec<PathBuf>,
}

impl Groups {
    fn all(&self) -> impl Iterator<Item = &PathBuf> {
        self.project
            .iter()
            .chain(&self.procedures)
            .chain(&self.checklists)
            .chain(&self.runs)
            .chain(&self.inbox)
            .chain(&self.help)
    }
}

/// Which rules apply to a file, decided by where it sits in the repository.
///
/// Location, not front matter, decides. A file in the wrong place is reported by the
/// rule set for where it actually is, which is how "this looks like a run record but it
/// is in `procedures/`" becomes a specific complaint rather than a silent pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Project,
    Procedure,
    Checklist,
    Run,
    Inbox,
    Help,
    Other,
}

fn kind_of(repo: &Repo, path: &Path) -> Kind {
    let rel = repo.relpath(path);
    if rel == sop_core::vocab::PROJECT_FILE {
        Kind::Project
    } else if rel.starts_with("procedures/") {
        Kind::Procedure
    } else if rel.starts_with("checklists/") {
        Kind::Checklist
    } else if rel.starts_with("runs/_inbox/") {
        Kind::Inbox
    } else if rel.starts_with("runs/") {
        Kind::Run
    } else if rel.starts_with("help/") {
        Kind::Help
    } else {
        Kind::Other
    }
}

/// Run records by run id, which is the filename stem.
fn run_records(repo: &Repo) -> BTreeMap<String, PathBuf> {
    let mut out = BTreeMap::new();
    for path in repo.discover().runs {
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            out.insert(stem.to_owned(), path);
        }
    }
    out
}

fn classify(repo: &Repo, paths: &[PathBuf]) -> Groups {
    let mut groups = Groups::default();
    for path in paths {
        match kind_of(repo, path) {
            Kind::Project => groups.project.push(path.clone()),
            Kind::Procedure => groups.procedures.push(path.clone()),
            Kind::Checklist => groups.checklists.push(path.clone()),
            Kind::Inbox => groups.inbox.push(path.clone()),
            Kind::Run => groups.runs.push(path.clone()),
            Kind::Help => groups.help.push(path.clone()),
            Kind::Other => {}
        }
    }
    groups
}

fn classify_all(repo: &Repo) -> Groups {
    let found = repo.discover();
    Groups {
        project: found.project.into_iter().collect(),
        procedures: found.procedures,
        checklists: found.checklists,
        runs: found.runs,
        inbox: found.inbox,
        help: found.help,
    }
}

/// The outcome of a validation pass.
pub struct Validation {
    pub report: Report,
    /// How many files were considered. Zero means the repository root was wrong, which
    /// is worth distinguishing from a repository that happens to be clean.
    pub files_checked: usize,
}

/// Validate every content file in the repository.
pub fn validate_repository(repo: &Repo) -> Validation {
    validate(repo, &[])
}

/// Validate specific files, or the whole repository when `files` is empty.
///
/// Passing files does not change the rules, only which files are checked. Repository
/// level rules such as "every log directory has a run record" need the whole picture,
/// so they only run for a full pass.
pub fn validate(repo: &Repo, files: &[PathBuf]) -> Validation {
    let full = files.is_empty();
    let groups = if full {
        classify_all(repo)
    } else {
        classify(repo, files)
    };
    let records = run_records(repo);
    let mut report = Report::new();
    let files_checked = groups.all().count();

    for path in groups.all() {
        match repo.read_text(path) {
            Ok(text) => report.extend(path, check::formatting(&text)),
            Err(error) => report.push(
                Some(path.clone()),
                Diagnostic::error(error.to_string(), None),
            ),
        }
    }

    for path in &groups.procedures {
        if let Some(loaded) = load_or_report(repo, path, &mut report) {
            check_procedure(repo, &loaded, &mut report);
        }
    }
    for path in &groups.checklists {
        if let Some(loaded) = load_or_report(repo, path, &mut report) {
            check_checklist(repo, &loaded, &mut report);
        }
    }
    for path in &groups.runs {
        if let Some(loaded) = load_or_report(repo, path, &mut report) {
            check_run(repo, &loaded, &records, &mut report);
        }
    }
    for path in &groups.inbox {
        if let Some(loaded) = load_or_report(repo, path, &mut report) {
            check_inbox(repo, &loaded, &mut report);
        }
    }
    for path in &groups.help {
        if let Some(loaded) = load_or_report(repo, path, &mut report) {
            check_help(repo, &loaded, &mut report);
        }
    }
    for path in &groups.project {
        if let Some(loaded) = load_or_report(repo, path, &mut report) {
            check_project(repo, &loaded, &mut report);
        }
    }

    if full {
        check_log_directories(repo, &records, &mut report);
    }

    check_help_ordering(repo, &groups.help, &mut report);

    Validation {
        report,
        files_checked,
    }
}

/// Check one file's text as if it were at `path`, without touching the disk.
///
/// This is what lets an editor refuse a change before it is written: the candidate text
/// is checked with the same rules and the same path, so a message about it is a message
/// about the file it would become. Rules that need more than one file - duplicate help
/// order, every log directory having a record - are not applied, because one file cannot
/// see them; the caller runs a full pass as well.
pub fn check_text(repo: &Repo, path: &Path, text: &str) -> Report {
    let mut report = Report::new();
    report.extend(path, check::formatting(text));

    let loaded = match loaded_from_text(repo, path, text) {
        Ok(loaded) => loaded,
        Err(diagnostic) => {
            report.push(Some(path.to_path_buf()), diagnostic);
            return report;
        }
    };

    match kind_of(repo, path) {
        Kind::Project => check_project(repo, &loaded, &mut report),
        Kind::Procedure => check_procedure(repo, &loaded, &mut report),
        Kind::Checklist => check_checklist(repo, &loaded, &mut report),
        // Duplicate run ids are a repository-level rule, so the map stays empty here.
        Kind::Run => check_run(repo, &loaded, &BTreeMap::new(), &mut report),
        Kind::Inbox => check_inbox(repo, &loaded, &mut report),
        Kind::Help => check_help(repo, &loaded, &mut report),
        Kind::Other => {}
    }

    report
}

fn loaded_from_text(repo: &Repo, path: &Path, text: &str) -> Result<Loaded, Diagnostic> {
    let doc = Document::parse(text)
        .map_err(|error| Diagnostic::error(error.to_string(), error.line()))?;
    Ok(Loaded {
        path: path.to_path_buf(),
        relpath: repo.relpath(path),
        text: text.to_owned(),
        doc,
    })
}

/// Load a file, turning a parse failure into a diagnostic on that file.
fn load_or_report(repo: &Repo, path: &Path, report: &mut Report) -> Option<crate::Loaded> {
    match repo.load(path) {
        Ok(loaded) => Some(loaded),
        Err(error) => {
            report.push(Some(path.to_path_buf()), parse_diagnostic(&error));
            None
        }
    }
}

fn parse_diagnostic(error: &RepoError) -> Diagnostic {
    Diagnostic::error(error.to_string(), error.line())
}

fn stem_of(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn check_procedure(repo: &Repo, loaded: &Loaded, report: &mut Report) {
    let path = loaded.path.as_path();
    let doc = &loaded.doc;
    let stem = stem_of(path);

    report.extend(path, check::front_matter(doc, "procedure"));
    report.extend(path, check::id_matches_filename(doc, "procedure_id", &stem));
    report.extend(path, check::steps(doc, true));
    report.extend(path, check::headings_without_steps(doc));

    if !doc.includes.is_empty() {
        report.error(path, "a procedure must not contain include markers", None);
    }

    report.extend(path, check::prechecked_items(doc));
    check_links(repo, path, doc, report);
}

fn check_checklist(repo: &Repo, loaded: &Loaded, report: &mut Report) {
    let path = loaded.path.as_path();
    let resolved = resolve_checklist(repo, loaded);
    let doc = &loaded.doc;
    let stem = stem_of(path);

    report.extend(path, check::front_matter(doc, "checklist"));
    report.extend(path, check::id_matches_filename(doc, "sop_id", &stem));
    report.extend(path, check::checklist_status(doc));
    report.extend(path, check::checklist_conditions(doc));

    if doc.front.string_list("equipment").is_empty() {
        report.warning(
            path,
            "'equipment' is empty; operators have no packing list",
            None,
        );
    }

    for (line, message) in &resolved.problems {
        report.error(path, message, Some(*line));
    }

    report.extend(path, check::steps(doc, false));
    report.extend(path, check::headings_without_steps(doc));
    report.extend(path, check::duplicate_ids(&resolved.steps));

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for include in &doc.includes {
        *counts.entry(include.target.as_str()).or_default() += 1;
    }
    for (target, count) in counts {
        if count > 1 {
            report.error(
                path,
                format!("procedure included more than once: {target}"),
                None,
            );
        }
    }

    if resolved.steps.is_empty() {
        report.error(path, "checklist resolves to zero steps", None);
    }

    report.extend(path, check::prechecked_items(doc));
    check_links(repo, path, doc, report);
}

fn check_run(
    repo: &Repo,
    loaded: &Loaded,
    records: &BTreeMap<String, PathBuf>,
    report: &mut Report,
) {
    let path = loaded.path.as_path();
    let doc = &loaded.doc;
    let stem = stem_of(path);

    report.extend(path, check::run_front_matter(doc));
    report.extend(path, check::id_matches_filename(doc, "run_id", &stem));

    if let Some(id) = doc.front.str("run_id").flatten()
        && records.get(id).is_some_and(|record| record != path)
    {
        report.error(
            path,
            format!("'run_id' '{id}' is used by more than one run record"),
            None,
        );
    }

    let sop = doc.front.str("sop").flatten();
    let mut checklist_steps: Option<Vec<String>> = None;
    match sop {
        // The checklist a run names can be a `checklists/` file or a `testplan/` case;
        // both record their `sop_id`, and both have to resolve here.
        Some(name) if repo.checklist_path(name).is_some() => {
            checklist_steps = repo.checklist_step_ids(name);
        }
        _ => report.error(
            path,
            format!(
                "'sop' {} does not name an existing checklist",
                sop.map_or("None".to_owned(), |name| format!("'{name}'"))
            ),
            Some(1),
        ),
    }

    // A record is read against the revision it was recorded against, which is the
    // snapshot frozen when it started (`SPEC.md` section 8 - checkbox identity, and step
    // ids with it, are defined by that revision). Resolving against the working copy
    // instead would mean that every later edit to the checklist retroactively broke every
    // run that used it, and the editor would refuse to make the edit at all.
    let snapshot_steps = sop.and_then(|name| repo.run_snapshot_step_ids(name, &stem));

    let expected_place = format!("runs/{}", sop.unwrap_or("None"));
    let actual_place = loaded
        .relpath
        .rsplit_once('/')
        .map_or(String::new(), |(directory, _)| directory.to_owned());
    if actual_place != expected_place {
        report.error(
            path,
            format!("run record must live in {expected_place}/"),
            None,
        );
    }

    // With no snapshot there is no recorded revision, so the best available answer is the
    // current checklist and it is advisory only: the record is evidence, and an SOP edit
    // is allowed to leave it behind. A record whose checklist does not resolve at all has
    // already been reported, and its citations are then checked against nothing.
    let citation = match snapshot_steps {
        Some(_) => check::Citation::Strict,
        None => check::Citation::Advisory,
    };
    if let Some(ids) = snapshot_steps.as_ref().or(checklist_steps.as_ref()) {
        report.extend(
            path,
            check::run_results(doc, sop.unwrap_or(""), ids, citation),
        );
        report.extend(path, check::complete_run_coverage(doc, ids, citation));
    } else {
        report.extend(
            path,
            check::run_results(doc, sop.unwrap_or(""), &[], check::Citation::Strict),
        );
    }

    // Time windows: a closed step needs a start, an end before its start is a clock that
    // went backwards, and an attached log has to overlap the window it was attached to.
    report.extend(path, check::missing_step_opened(doc));
    report.extend(path, check::time_regression(doc));
    report.extend(path, check::log_overlap(doc));

    // A run that has just started has no results yet, and that is the normal state of a
    // record on disk mid-run. Only a finished run with nothing recorded is worth a
    // second look.
    if doc.results.is_empty() && doc.front.contains("ended") {
        report.warning(path, "run record has no step results", None);
    }

    for entry in as_list(doc.front.get("logs")) {
        check_log_entry(repo, path, stem_of(path).as_str(), entry, report);
    }

    // A record that sealed its event log at end must match the log on disk; a log edited
    // afterwards no longer matches and is an error, not a soft warning.
    if let Some(sealed) = doc.front.str("events_sha256").flatten() {
        let record = repo.relpath(path);
        let run_dir = record.strip_suffix(".md").unwrap_or(&record);
        let events_path = repo.resolve(&format!("{run_dir}/events.jsonl"));
        if let Ok(bytes) = std::fs::read(&events_path) {
            let actual = hex(&sha256(&bytes));
            if sealed.to_ascii_lowercase() != actual {
                report.error(
                    path,
                    format!(
                        "the event log was changed after this record was sealed: sealed {sealed}, now {actual}"
                    ),
                    Some(1),
                );
            }
        }
    }

    check_links(repo, path, doc, report);
}

/// A value read as a list: sequences as themselves, everything else as one item.
fn as_list(value: Option<&Value>) -> Vec<&Value> {
    match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Sequence(items)) => items.iter().collect(),
        Some(other) => vec![other],
    }
}

fn check_log_entry(repo: &Repo, path: &Path, run_id: &str, entry: &Value, report: &mut Report) {
    let Some(map) = entry.as_mapping() else {
        report.error(path, "each 'logs' entry must be a mapping", None);
        return;
    };

    let log_path = match map.get("path").and_then(|value| value.as_str()) {
        Some(log_path) => log_path.to_owned(),
        None => {
            report.error(path, "'logs' entry has no 'path'", None);
            return;
        }
    };

    // A run's data lives inside the run directory (`runs/<sop>/<run_id>/logs/`), which is
    // what the app writes and what D10 decided; `logs/<run_id>/` is kept as the location
    // for records written before that. Anything else is a warning, because a log outside
    // both is data the run cannot be archived with.
    let record = repo.relpath(path);
    let run_dir = record.strip_suffix(".md").unwrap_or(&record);
    let inside_run = format!("{run_dir}/logs/");
    let legacy = format!("logs/{run_id}/");
    if !log_path.starts_with(&inside_run) && !log_path.starts_with(&legacy) {
        report.warning(
            path,
            format!("log '{log_path}' is not under {inside_run} (see docs/LOGS.md)"),
            None,
        );
    }

    let digest = map.get("sha256").and_then(|value| value.as_str());
    if digest.is_none() {
        report.warning(
            path,
            format!("log '{log_path}': quote 'sha256' so leading zeros survive YAML"),
            None,
        );
    }

    if map.get("external").and_then(|value| value.as_bool()) == Some(true) {
        if digest.is_none() {
            report.error(
                path,
                format!("log '{log_path}' is marked external but has no 'sha256'"),
                None,
            );
        }
        return;
    }

    let absolute = repo.resolve(&log_path);
    let Ok(bytes) = std::fs::read(&absolute) else {
        report.error(
            path,
            format!("attached log does not exist: {log_path}"),
            None,
        );
        return;
    };

    let actual = hex(&sha256(&bytes));
    if let Some(digest) = digest
        && digest.to_ascii_lowercase() != actual
    {
        report.error(
            path,
            format!(
                "log '{log_path}': sha256 in the record does not match the file ({digest} != {actual})"
            ),
            None,
        );
    }

    if let Some(size) = map.get("size").and_then(|value| value.as_i64())
        && size != bytes.len() as i64
    {
        report.error(
            path,
            format!(
                "log '{log_path}': recorded size is {size} but the file is {} bytes",
                bytes.len()
            ),
            None,
        );
    }
}

fn check_log_directories(repo: &Repo, records: &BTreeMap<String, PathBuf>, report: &mut Report) {
    let logs_root = repo.root().join("logs");
    let Ok(entries) = std::fs::read_dir(&logs_root) else {
        return;
    };
    let mut names: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    names.sort();

    for path in names {
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_owned(),
            None => continue,
        };
        if !records.contains_key(&name) {
            report.push(
                Some(path.clone()),
                Diagnostic::error(
                    format!("log directory '{name}' has no matching run record in runs/"),
                    None,
                ),
            );
        }
    }
}

fn check_inbox(repo: &Repo, loaded: &Loaded, report: &mut Report) {
    let path = loaded.path.as_path();
    let doc = &loaded.doc;

    for key in ["created", "author", "observed_in", "target", "confidence"] {
        if !doc.front.contains(key) {
            report.error(path, format!("inbox entry is missing '{key}'"), Some(1));
        }
    }

    match doc.front.str("confidence").flatten() {
        Some(confidence) if ["low", "medium", "high"].contains(&confidence) => {}
        other => report.error(
            path,
            format!(
                "'confidence' {} must be low, medium, or high",
                other.map_or("None".to_owned(), |value| format!("'{value}'"))
            ),
            Some(1),
        ),
    }

    for key in ["observed_in", "target"] {
        if let Some(value) = doc.front.str(key).flatten()
            && !repo.resolve(value).exists()
        {
            report.error(path, format!("'{key}' does not exist: {value}"), Some(1));
        }
    }

    check_links(repo, path, doc, report);
}

fn check_project(repo: &Repo, loaded: &Loaded, report: &mut Report) {
    let path = loaded.path.as_path();
    let doc = &loaded.doc;
    report.extend(path, check::project_front_matter(doc));
    check_links(repo, path, doc, report);
}

fn check_help(repo: &Repo, loaded: &Loaded, report: &mut Report) {
    let path = loaded.path.as_path();
    let doc = &loaded.doc;

    report.extend(path, check::help_front_matter(doc));
    report.extend(
        path,
        check::id_matches_filename(doc, "help_id", &stem_of(path)),
    );
    check_links(repo, path, doc, report);
}

/// Two help pages may not claim the same panel position.
///
/// The order is presentation only, so this is a warning: the panel still renders, it
/// just falls back on the section and page id to break the tie.
fn check_help_ordering(repo: &Repo, paths: &[PathBuf], report: &mut Report) {
    let mut seen: BTreeMap<i64, String> = BTreeMap::new();
    for path in paths {
        let Ok(loaded) = repo.load(path) else {
            continue;
        };
        let doc = &loaded.doc;
        let order = doc
            .front
            .i64("order")
            .unwrap_or(sop_core::DEFAULT_HELP_ORDER);
        let id = doc
            .front
            .str("help_id")
            .flatten()
            .map(str::to_owned)
            .unwrap_or_else(|| stem_of(path));
        if let Some(first) = seen.insert(order, id.clone()) {
            report.warning(
                path,
                format!(
                    "help page '{id}' has the same 'order' as '{first}'; the panel will \
                     fall back on the section and page id (see SPEC.md section 12)"
                ),
                Some(1),
            );
        }
    }
}

/// Relative links must point at something that exists, either from the file or from the
/// repository root. Anchors and absolute URLs have nothing on disk to check.
fn check_links(repo: &Repo, path: &Path, doc: &Document, report: &mut Report) {
    let base = path.parent().unwrap_or(repo.root());
    for link in check::link_targets(doc) {
        let candidate = link.path();
        if candidate.is_empty() {
            continue;
        }
        let resolved = base.join(candidate).exists() || repo.root().join(candidate).exists();
        if !resolved {
            report.error(
                path,
                format!("link target does not resolve: {}", link.target),
                Some(link.line),
            );
        }
    }
}

/// Lowercase hex, for comparing digests in a record with a file on disk.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

/// Repository-relative path for messages, exposed for callers that render their own way.
pub fn relative(repo: &Repo, path: &Path) -> String {
    display_path(repo.root(), path)
}
