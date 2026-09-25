//! Invariants the generated manifest has to hold, whatever the content happens to be.
//!
//! These are deliberately about shape and ordering rather than counts, so that adding a
//! procedure or a help page does not break the tests.

use std::path::{Path, PathBuf};

use sop_repo::{Repo, manifest};

fn repo() -> Repo {
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    Repo::open(root)
}

#[test]
fn every_help_page_is_complete_and_in_panel_order() {
    let manifest = manifest::build(&repo());
    assert!(!manifest.help.is_empty(), "the repository ships help pages");

    let mut previous = i64::MIN;
    for page in &manifest.help {
        let id = page
            .help_id
            .as_ref()
            .and_then(|id| id.as_str())
            .unwrap_or_default();
        assert!(!id.is_empty(), "help page without a help_id: {}", page.path);
        assert!(
            page.path.starts_with("help/"),
            "{id} lives outside help/: {}",
            page.path
        );
        assert!(
            page.title
                .as_ref()
                .and_then(|title| title.as_str())
                .is_some_and(|t| !t.is_empty()),
            "{id} has no title"
        );
        assert!(
            page.section
                .as_ref()
                .and_then(|section| section.as_str())
                .is_some_and(|s| !s.is_empty()),
            "{id} has no section, so the panel cannot group it"
        );
        assert!(!page.body.trim().is_empty(), "{id} has an empty body");

        let order = page.order.as_i64().expect("order is always an integer");
        assert!(order >= previous, "{id} is out of panel order");
        previous = order;
    }
}

#[test]
fn help_sections_are_the_distinct_sections_in_first_appearance_order() {
    let manifest = manifest::build(&repo());
    let mut expected: Vec<String> = Vec::new();
    for page in &manifest.help {
        let title = page
            .section
            .as_ref()
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .to_owned();
        if !expected.contains(&title) {
            expected.push(title);
        }
    }
    let actual: Vec<String> = manifest
        .help_sections
        .iter()
        .map(|s| s.title.clone())
        .collect();
    assert_eq!(actual, expected);

    for section in &manifest.help_sections {
        let lowest = manifest
            .help
            .iter()
            .filter(|page| {
                page.section.as_ref().and_then(|s| s.as_str()) == Some(section.title.as_str())
            })
            .filter_map(|page| page.order.as_i64())
            .min()
            .unwrap();
        assert_eq!(
            section.order, lowest,
            "{} has the wrong order",
            section.title
        );
    }
}

#[test]
fn identifier_columns_are_sorted() {
    let manifest = manifest::build(&repo());
    let ids: Vec<String> = manifest
        .procedures
        .iter()
        .map(|entry| {
            entry
                .procedure_id
                .as_ref()
                .and_then(|id| id.as_str())
                .unwrap_or("")
                .to_owned()
        })
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "procedures are sorted by procedure_id");

    let ids: Vec<String> = manifest
        .checklists
        .iter()
        .map(|entry| {
            entry
                .sop_id
                .as_ref()
                .and_then(|id| id.as_str())
                .unwrap_or("")
                .to_owned()
        })
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "checklists are sorted by sop_id");
}

#[test]
fn a_resolved_checklist_has_steps_and_no_unresolved_includes() {
    let manifest = manifest::build(&repo());
    for checklist in &manifest.checklists {
        let id = checklist
            .sop_id
            .as_ref()
            .and_then(|id| id.as_str())
            .unwrap_or_default();
        assert!(!checklist.steps.is_empty(), "{id} resolves to no steps");
        assert!(
            checklist.unresolved_includes.is_empty(),
            "{id} has unresolved includes: {:?}",
            checklist.unresolved_includes
        );
        assert_eq!(checklist.step_count, checklist.steps.len());
    }
}

#[test]
fn every_run_record_points_at_a_known_checklist_directory() {
    let manifest = manifest::build(&repo());
    for run in &manifest.runs {
        let sop = run
            .sop
            .as_ref()
            .and_then(|s| s.as_str())
            .unwrap_or_default();
        assert!(
            run.path.starts_with(&format!("runs/{sop}/")),
            "{} is not under runs/{sop}/",
            run.path
        );
    }
}

#[test]
fn the_manifest_is_byte_stable_across_builds() {
    let repo = repo();
    let first = manifest::build(&repo).to_json();
    let second = manifest::build(&repo).to_json();
    assert_eq!(
        first, second,
        "the manifest must not depend on iteration order"
    );
    assert!(
        first.ends_with("}\n"),
        "a trailing newline keeps git diffs quiet"
    );
}
