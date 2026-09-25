//! Shared scaffolding for the tests that need a repository to break.
//!
//! The interesting tests are about a repository that is *almost* right, so they copy the
//! real one, change one thing, and check what happens. Copying the real content rather
//! than a fixture is deliberate: a fixture would drift away from the format, and the
//! tests would keep passing while the content stopped being valid.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use sop_repo::{Repo, validate};

/// Top-level entries never copied into a scratch repository.
const SKIP: &[&str] = &[".git", "dist", "target", "__pycache__", ".venv"];

pub fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A throwaway repository copy, removed when the test ends.
pub struct Scratch {
    pub root: PathBuf,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Scratch {
    pub fn new(name: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "field-sop-test-{}-{serial}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        copy_tree(&source_root(), &root);
        Self { root }
    }

    /// A scratch repository with no content at all.
    pub fn empty(name: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "field-sop-empty-{}-{serial}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn repo(&self) -> Repo {
        Repo::open(&self.root)
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path(rel)).unwrap()
    }

    pub fn write(&self, rel: &str, text: &str) {
        fs::write(self.path(rel), text).unwrap();
    }

    pub fn replace_once(&self, rel: &str, from: &str, to: &str) {
        let text = self.read(rel);
        assert!(
            text.contains(from),
            "fixture drifted: {from:?} not found in {rel}"
        );
        self.write(rel, &text.replacen(from, to, 1));
    }

    pub fn append(&self, rel: &str, extra: &str) {
        let text = self.read(rel);
        self.write(rel, &format!("{text}{extra}"));
    }

    /// Replace a front matter block - `key:` plus its indented lines - with `key: []`.
    pub fn clear_front_matter_block(&self, rel: &str, key: &str) {
        let text = self.read(rel);
        let lines: Vec<&str> = text.split('\n').collect();
        let header = format!("{key}:");
        let start = lines
            .iter()
            .position(|line| *line == header)
            .unwrap_or_else(|| panic!("fixture drifted: no '{header}' block in {rel}"));
        let mut end = start + 1;
        while end < lines.len() && lines[end].starts_with("  ") {
            end += 1;
        }
        let mut out: Vec<&str> = lines[..start].to_vec();
        let replacement = format!("{key}: []");
        out.push(&replacement);
        out.extend_from_slice(&lines[end..]);
        self.write(rel, &out.join("\n"));
    }

    /// Run the validator and return whether it failed, plus the rendered report.
    pub fn validate(&self) -> (bool, String) {
        let repo = self.repo();
        let outcome = validate::validate_repository(&repo);
        (
            outcome.report.has_errors(),
            outcome.report.render(repo.root()),
        )
    }
}

pub fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy().to_string();
        if SKIP.contains(&name.as_str()) || name.ends_with(".pyc") {
            continue;
        }
        let target = to.join(&name);
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            let _ = fs::remove_file(&target);
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}
