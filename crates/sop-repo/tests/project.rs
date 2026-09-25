//! `sop project` edits `project.md` in place.
//!
//! The file is written by a person and read by everyone, so the tests are about what an
//! edit must *not* do: it must not reorder keys, must not drop an unknown key, must not
//! touch the prose, and must not write anything at all when the result would be invalid.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use sop_repo::{Repo, project};

const SAMPLE: &str = "\
---
kind: project
project_id: uvic-geomag-survey
title: Geomagnetic Survey Field Work
institution: University of Victoria
# the lead is the person to ask about the content
lead: leo
updated: 2026-09-25
future_key: kept verbatim
---

The campaign this repository serves.

## Scope

Magnetic field surveying.
";

/// A throwaway repository holding one project file.
struct Scratch {
    root: PathBuf,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Scratch {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "field-sop-project-{}-{serial}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("project.md"), SAMPLE).unwrap();
        Self { root }
    }

    fn repo(&self) -> Repo {
        Repo::open(&self.root)
    }

    fn path(&self) -> PathBuf {
        self.root.join("project.md")
    }

    fn text(&self) -> String {
        fs::read_to_string(self.path()).unwrap()
    }

    fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.root)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

#[test]
fn setting_a_field_keeps_everything_else_in_the_file() {
    let scratch = Scratch::new("keep");
    let path = project::set(&scratch.repo(), "title", "Renfrew 2026").unwrap();
    assert_eq!(path, scratch.path());

    let text = scratch.text();
    assert!(text.contains("title: Renfrew 2026"), "{text}");
    assert!(text.contains("# the lead is the person to ask"), "{text}");
    assert!(text.contains("future_key: kept verbatim"), "{text}");
    assert!(text.contains("Magnetic field surveying."), "{text}");
    assert!(
        text.find("kind: project").unwrap() < text.find("title:").unwrap(),
        "key order must not change: {text}"
    );
    assert_eq!(
        scratch.entries(),
        vec!["project.md".to_owned()],
        "no temporary file may be left behind"
    );
}

#[test]
fn a_hyphenated_field_name_is_accepted() {
    let scratch = Scratch::new("hyphen");
    project::set(&scratch.repo(), "project-id", "renfrew-2026").unwrap();
    assert!(scratch.text().contains("project_id: renfrew-2026"));
    assert!(
        !scratch.text().contains("project-id:"),
        "the spec's key wins"
    );
}

#[test]
fn a_field_that_is_absent_is_added_to_the_front_matter() {
    let scratch = Scratch::new("absent");
    project::set(&scratch.repo(), "contact", "haijunli@uvic.ca").unwrap();
    let text = scratch.text();
    let front = text.split("---").nth(1).unwrap();
    assert!(front.contains("contact: haijunli@uvic.ca"), "{text}");
}

#[test]
fn a_value_that_would_not_validate_is_refused_and_nothing_is_written() {
    let scratch = Scratch::new("refused");
    let before = scratch.text();

    let error = project::set(&scratch.repo(), "project_id", "Renfrew 2026").unwrap_err();
    assert!(
        matches!(error, project::ProjectError::Refused(_)),
        "expected a refusal, got {error}"
    );
    assert!(error.to_string().contains("is not a valid id"), "{error}");
    assert_eq!(scratch.text(), before, "the file must be untouched");

    let error = project::set(&scratch.repo(), "updated", "yesterday").unwrap_err();
    assert!(error.to_string().contains("YYYY-MM-DD"), "{error}");
    assert_eq!(scratch.text(), before);
}

#[test]
fn a_word_a_yaml_reader_would_misread_is_written_as_text() {
    let scratch = Scratch::new("quoted");
    project::set(&scratch.repo(), "lead", "no").unwrap();
    let text = scratch.text();
    assert!(text.contains("lead: \"no\""), "{text}");
    assert_eq!(
        project::title(&scratch.repo()).as_deref(),
        Some("Geomagnetic Survey Field Work")
    );
}

#[test]
fn an_unknown_field_is_rejected_by_name() {
    let scratch = Scratch::new("unknown");
    let error = project::set(&scratch.repo(), "funding", "NSERC").unwrap_err();
    assert!(
        matches!(error, project::ProjectError::UnknownField(_)),
        "{error}"
    );
    assert!(error.to_string().contains("sop project"), "{error}");
}

#[test]
fn a_repository_without_a_project_file_says_so() {
    let scratch = Scratch::new("missing");
    fs::remove_file(scratch.path()).unwrap();
    let error = project::set(&scratch.repo(), "title", "Anything").unwrap_err();
    assert!(
        matches!(error, project::ProjectError::Missing { .. }),
        "{error}"
    );
    assert!(project::load(&scratch.repo()).unwrap().is_none());
    assert_eq!(project::title(&scratch.repo()), None);
}

#[test]
fn the_title_is_unavailable_rather_than_guessed() {
    let scratch = Scratch::new("kind");
    fs::write(
        scratch.path(),
        "---\nkind: help\ntitle: Not a project\nhelp_id: x\nsection: s\nupdated: 2026-01-01\n---\n",
    )
    .unwrap();
    assert_eq!(
        project::title(&scratch.repo()),
        None,
        "a file that is not a project file has no project title"
    );
}

#[test]
fn an_empty_value_is_rejected_before_the_file_is_touched() {
    let scratch = Scratch::new("empty");
    let error = project::set(&scratch.repo(), "title", "   ").unwrap_err();
    assert!(error.to_string().contains("must not be empty"), "{error}");
    assert_eq!(scratch.text(), SAMPLE);
}
