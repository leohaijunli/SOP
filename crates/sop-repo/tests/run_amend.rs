//! Adding to a run after it ended, and removing a run that gained new subdirectories.
//!
//! A finished run is not frozen shut: the operator can upload a log, a photo, or another
//! file and add a note, as long as what the run *did* is untouched. The seal then covers
//! only the prefix recorded at end (`DECISIONS.md` D29). These also check that a run
//! carrying `photos/`, `attachments/`, and `notes.md` is still found once and removed
//! whole.

use std::fs;
use std::path::PathBuf;

use sop_core::RunEvent;
use sop_core::run::AttachmentKind;
use sop_repo::{Repo, manifest, run};

mod common;
use common::Scratch;

const SOP: &str = "ground-walk-survey";
const RUN: &str = "2026-09-25-test-run";
const RECORD: &str = "runs/uvic-geomag-survey/ground-walk-survey/2026-09-25-test-run.md";
const RUN_DIR: &str = "runs/uvic-geomag-survey/ground-walk-survey/2026-09-25-test-run";

fn start(repo: &Repo, run_id: &str) {
    run::start(
        repo,
        SOP,
        run_id,
        "leo",
        "Renfrew 395",
        Some("automated test"),
    )
    .unwrap();
}

/// A finished run to add to.
fn ended(repo: &Repo) {
    start(repo, RUN);
    run::end(repo, SOP, RUN, "partial", None).unwrap();
}

/// Write a file into `incoming/` and return its path. `incoming/` is not a directory
/// discovery or validation reads, so it stands in for a file picked from anywhere.
fn incoming(scratch: &Scratch, name: &str, bytes: &str) -> PathBuf {
    let dir = scratch.path("incoming");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

/// Add a run-level note the way a caller does: as a `NoteAdded` event. The tool stamps it.
fn note(repo: &Repo, text: &str) {
    run::record(
        repo,
        SOP,
        RUN,
        &RunEvent::NoteAdded {
            at: String::new(),
            step: None,
            text: text.to_owned(),
        },
    )
    .unwrap();
}

#[test]
fn a_photo_and_a_file_land_in_their_own_subdirectories() {
    let scratch = Scratch::new("amend-kinds");
    let repo = scratch.repo();
    ended(&repo);

    let jpg = incoming(&scratch, "plot.jpg", "jpeg bytes\n");
    let pdf = incoming(&scratch, "notes.pdf", "pdf bytes\n");
    let photo = run::attach_kind(&repo, SOP, RUN, None, AttachmentKind::Photo, &jpg).unwrap();
    let file = run::attach_kind(&repo, SOP, RUN, None, AttachmentKind::File, &pdf).unwrap();

    assert_eq!(photo.path, format!("{RUN_DIR}/photos/plot.jpg"));
    assert_eq!(file.path, format!("{RUN_DIR}/attachments/notes.pdf"));
    assert!(scratch.path(&photo.path).is_file());
    assert!(scratch.path(&file.path).is_file());
    assert!(
        photo.post_run && file.post_run,
        "both were added after the run ended"
    );
    // A photo is not scanned for a time column, unlike a log.
    assert_eq!(photo.t_min, None);

    let (failed, report) = scratch.validate();
    assert!(
        !failed,
        "a run with post-run photos/files must validate:\n{report}"
    );
    assert!(
        !report.contains("is not under"),
        "a photo/file entry names its kind, so the path check does not warn:\n{report}"
    );
}

#[test]
fn a_log_and_a_photo_with_the_same_bytes_are_not_deduplicated() {
    let scratch = Scratch::new("amend-kind-dedupe");
    let repo = scratch.repo();
    ended(&repo);
    // An empty file has the same sha256 whatever its kind, so the kind has to be part of
    // the identity or one would replace the other.
    let log_path = incoming(&scratch, "empty.log", "");
    let jpg_path = incoming(&scratch, "empty.jpg", "");

    let log = run::attach(&repo, SOP, RUN, None, &log_path).unwrap();
    let photo = run::attach_kind(&repo, SOP, RUN, None, AttachmentKind::Photo, &jpg_path).unwrap();
    assert_eq!(log.sha256, photo.sha256);
    assert_ne!(log.kind, photo.kind);
    let loaded = run::load(&repo, SOP, RUN).unwrap();
    assert_eq!(loaded.state.run_attachments.len(), 2);
}

#[test]
fn a_photo_entry_whose_path_does_not_match_its_kind_warns() {
    let scratch = Scratch::new("amend-kind-path");
    let repo = scratch.repo();
    ended(&repo);
    let jpg = incoming(&scratch, "plot.jpg", "jpeg\n");
    run::attach_kind(&repo, SOP, RUN, None, AttachmentKind::Photo, &jpg).unwrap();

    // A photo sits in `photos/`. Declaring it a `log` while it stays there is the
    // hand-edit the kind/path check exists to catch; the file is still present and hashes
    // correctly, so this is a warning rather than an error.
    scratch.replace_once(RECORD, "    kind: photo\n", "    kind: log\n");
    let (failed, report) = scratch.validate();
    assert!(
        !failed,
        "a misplaced entry is a warning, not an error:\n{report}"
    );
    assert!(
        report.contains("is not under"),
        "the warning names the expected directory:\n{report}"
    );
}

#[test]
fn a_post_run_attachment_and_note_refresh_the_committed_record() {
    let scratch = Scratch::new("amend-refresh");
    let repo = scratch.repo();
    ended(&repo);
    let sealed_before = scratch.read(RECORD);
    let seal_line = sealed_before
        .lines()
        .find(|line| line.starts_with("events_sealed_bytes:"))
        .unwrap()
        .to_owned();

    let csv = incoming(&scratch, "late.csv", "a,b\n1,2\n");
    run::attach(&repo, SOP, RUN, None, &csv).unwrap();
    note(&repo, "recovered after the run");

    let record = scratch.read(RECORD);
    // The seal is unchanged: it still covers the prefix recorded at end.
    assert!(
        record.contains(&seal_line),
        "the seal must not move:\n{record}"
    );
    assert!(record.contains("## Added after the run ended"), "{record}");
    assert!(record.contains("late.csv"), "{record}");
    assert!(record.contains("recovered after the run"), "{record}");
    // The note also lands in the generated notes.md.
    let notes = scratch.read(&format!("{RUN_DIR}/notes.md"));
    assert!(notes.contains("recovered after the run"), "{notes}");

    // And the record on disk is the same text `record_text` would render.
    let rendered = run::record_text(&repo, SOP, RUN).unwrap();
    assert_eq!(rendered.trim_end(), record.trim_end());

    let (failed, report) = scratch.validate();
    assert!(!failed, "a refreshed record must validate:\n{report}");
}

#[test]
fn a_checkbox_change_after_the_run_ends_is_refused() {
    let scratch = Scratch::new("amend-refuse");
    let repo = scratch.repo();
    ended(&repo);
    let before = scratch.read(&format!("{RUN_DIR}/events.jsonl"));
    let late = RunEvent::CheckboxToggled {
        at: "2026-09-25T13:00:00Z".into(),
        step: "cond-location".into(),
        index: 0,
        checked: true,
    };
    assert!(
        run::record(&repo, SOP, RUN, &late).is_err(),
        "the field record is frozen"
    );
    assert_eq!(scratch.read(&format!("{RUN_DIR}/events.jsonl")), before);
}

#[test]
fn deleting_a_run_removes_its_photos_attachments_and_notes() {
    let scratch = Scratch::new("amend-delete");
    let repo = scratch.repo();
    ended(&repo);
    let jpg = incoming(&scratch, "plot.jpg", "jpeg\n");
    run::attach_kind(&repo, SOP, RUN, None, AttachmentKind::Photo, &jpg).unwrap();
    note(&repo, "a late note");
    assert!(
        scratch
            .path(&format!("{RUN_DIR}/photos/plot.jpg"))
            .is_file()
    );
    assert!(scratch.path(&format!("{RUN_DIR}/notes.md")).is_file());

    run::delete(&repo, SOP, RUN).unwrap();
    assert!(!scratch.path(RECORD).exists());
    assert!(!scratch.path(RUN_DIR).exists());
}

#[test]
fn discovery_finds_the_record_once_and_ignores_generated_files() {
    let scratch = Scratch::new("amend-discover");
    let repo = scratch.repo();
    ended(&repo);
    note(&repo, "a note");
    // `record.md` and `notes.md` live inside the run directory; neither is a run record.
    assert!(scratch.path(&format!("{RUN_DIR}/record.md")).is_file());
    assert!(scratch.path(&format!("{RUN_DIR}/notes.md")).is_file());

    let found = run::all_runs(&repo);
    assert_eq!(
        found.iter().filter(|(_, id)| id == RUN).count(),
        1,
        "{found:?}"
    );
    let manifest_runs = manifest::build(&repo).runs;
    let matching: Vec<_> = manifest_runs
        .iter()
        .filter(|entry| entry.run_id.as_ref().and_then(|id| id.as_str()) == Some(RUN))
        .collect();
    assert_eq!(matching.len(), 1, "History must list the run once");
}
