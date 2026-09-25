//! Collecting diagnostics from many checks, and rendering them the way operators and
//! CI expect to read them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sop_core::{Diagnostic, Diagnostics, Severity};

/// One reported problem, tied to the file it is about.
#[derive(Debug, Clone)]
pub struct Finding {
    /// `None` means the repository as a whole rather than one file.
    pub path: Option<PathBuf>,
    pub diagnostic: Diagnostic,
}

#[derive(Debug, Default, Clone)]
pub struct Report {
    items: Vec<Finding>,
}

impl Report {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, path: Option<PathBuf>, diagnostic: Diagnostic) {
        self.items.push(Finding { path, diagnostic });
    }

    pub fn error(
        &mut self,
        path: impl Into<PathBuf>,
        message: impl Into<String>,
        line: Option<usize>,
    ) {
        self.push(Some(path.into()), Diagnostic::error(message, line));
    }

    pub fn warning(
        &mut self,
        path: impl Into<PathBuf>,
        message: impl Into<String>,
        line: Option<usize>,
    ) {
        self.push(Some(path.into()), Diagnostic::warning(message, line));
    }

    /// Add every diagnostic from a check about one file.
    pub fn extend(&mut self, path: &Path, diagnostics: Diagnostics) {
        for diagnostic in diagnostics.into_items() {
            self.push(Some(path.to_path_buf()), diagnostic);
        }
    }

    pub fn items(&self) -> &[Finding] {
        &self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|item| item.diagnostic.is_error())
    }

    pub fn error_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.diagnostic.is_error())
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.items.len() - self.error_count()
    }

    /// Files that have at least one error, relative to `root`, sorted.
    pub fn files_with_errors(&self, root: &Path) -> Vec<String> {
        let mut out: Vec<String> = self
            .items
            .iter()
            .filter(|item| item.diagnostic.is_error())
            .filter_map(|item| item.path.as_deref())
            .map(|path| display_path(root, path))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// `path/to/file.md`\n`  error: line 12: message`\n … , then a summary line.
    pub fn render(&self, root: &Path) -> String {
        let mut by_path: BTreeMap<String, Vec<&Diagnostic>> = BTreeMap::new();
        let mut order: Vec<String> = Vec::new();
        for item in &self.items {
            let key = match item.path.as_deref() {
                Some(path) => display_path(root, path),
                None => "<repo>".to_owned(),
            };
            if !by_path.contains_key(&key) {
                order.push(key.clone());
            }
            by_path.entry(key).or_default().push(&item.diagnostic);
        }

        let mut out = String::new();
        for key in order {
            out.push_str(&key);
            out.push('\n');
            let mut group: Vec<&Diagnostic> = by_path[&key].clone();
            group.sort_by_key(|diagnostic| diagnostic.line.unwrap_or(0));
            for diagnostic in group {
                let where_ = match diagnostic.line {
                    Some(line) => format!("line {line}: "),
                    None => String::new(),
                };
                let severity = match diagnostic.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                };
                out.push_str(&format!("  {severity}: {where_}{}\n", diagnostic.message));
            }
        }
        out.push('\n');
        out.push_str(&format!(
            "{} error(s), {} warning(s)\n",
            self.error_count(),
            self.warning_count()
        ));
        out
    }
}

/// Repository-relative display path, falling back to the path as given.
pub fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
