//! Packaging a summary and its runs into one self-contained folder.
//!
//! The package is the thing handed to someone else, so these check it is complete (the
//! summary, every run's record and folder, a manifest), that a half-written package is
//! never left behind, and that an export cannot be steered out of the folder it names by
//! a bad label or a symlink.

use std::fs;
use std::path::Path;

use sop_core::run::AttachmentKind;
use sop_repo::export::{self, ExportError};
use sop_repo::run;
use sop_repo::summary::SummaryMeta;

mod common;
use common::Scratch;

const SOP: &str = "ground-walk-survey";
const RUN: &str = "2026-09-25-test-run";
/// The fixture project_id, which prefixes the run layout since the top level became the
/// project. The scratch repo's `project.md` carries it.
/// `runs/<project>/<sop>`, the folder a run's record and directory live under.
const RUN_DIR: &str = "runs/uvic-geomag-survey/ground-walk-survey";

/// The summary's timestamps stay as stored, so the output is deterministic.
fn ident(utc: &str) -> String {
    utc.to_owned()
}

fn meta() -> SummaryMeta {
    SummaryMeta {
        project: "Renfrew".to_owned(),
        version: "0.1.0".to_owned(),
        git_commit: Some("abc1234".to_owned()),
        git_dirty: false,
        exported: "2026-09-29 14:32".to_owned(),
    }
}

fn cases_repo() -> Scratch {
    let cases = Scratch::empty("export-cases");
    let plan = cases.path("testplan/preflight");
    fs::create_dir_all(&plan).unwrap();
    fs::write(
        plan.join("plan.md"),
        "---\ntitle: Preflight\norder: 1\n---\n\n",
    )
    .unwrap();
    fs::write(
        plan.join("walk.md"),
        "---\nkind: checklist\nsop_id: ground-walk-survey\ntitle: Walk Survey\n---\n\n## Step\n\n- [ ] x\n",
    )
    .unwrap();
    cases
}

/// One finished run with a photo, a file, and a note already added.
fn ended_with_material(scratch: &Scratch, repo: &sop_repo::Repo) {
    run::start(repo, SOP, RUN, "leo", "Renfrew 395", Some("automated test")).unwrap();
    run::end(repo, SOP, RUN, "partial", None).unwrap();
    let dir = scratch.path("incoming");
    fs::create_dir_all(&dir).unwrap();
    let jpg = dir.join("plot.jpg");
    let pdf = dir.join("notes.pdf");
    fs::write(&jpg, "jpeg bytes\n").unwrap();
    fs::write(&pdf, "pdf bytes\n").unwrap();
    run::attach_kind(repo, SOP, RUN, None, AttachmentKind::Photo, &jpg).unwrap();
    run::attach_kind(repo, SOP, RUN, None, AttachmentKind::File, &pdf).unwrap();
}

fn export_one(repo: &sop_repo::Repo, dest: &Path, label: &str) -> export::ExportReport {
    let cases = cases_repo();
    let cases_repo = cases.repo();
    let picked = vec![(SOP.to_owned(), RUN.to_owned())];
    export::export_package(
        repo,
        &cases_repo,
        dest,
        label,
        &ident,
        &meta(),
        Some(&picked),
    )
    .unwrap()
}

#[test]
fn a_package_holds_the_summary_every_run_and_a_manifest() {
    let scratch = Scratch::new("export-structure");
    let repo = scratch.repo();
    ended_with_material(&scratch, &repo);
    let dest = scratch.path("exports");

    let report = export_one(&repo, &dest, "2026-09-29_143200");

    assert_eq!(report.runs, 1, "{report:?}");
    assert!(report.failed.is_empty(), "{report:?}");
    // The package landed in `<dest>/export_<label>`, not directly in `dest`.
    assert_eq!(report.dir, dest.join("export_2026-09-29_143200"));
    assert!(report.dir.is_dir());

    // The two summaries and the manifest of every file.
    assert!(report.dir.join("summary.md").is_file());
    assert!(report.dir.join("summary.csv").is_file());
    assert!(report.dir.join("MANIFEST.sha256").is_file());

    // The run's record and its whole folder, with the photo/file subdirectories.
    let run_root = report.dir.join(RUN_DIR);
    assert!(run_root.join(format!("{RUN}.md")).is_file());
    assert!(run_root.join(RUN).join("events.jsonl").is_file());
    assert!(run_root.join(RUN).join("record.md").is_file());
    assert!(run_root.join(RUN).join("photos").join("plot.jpg").is_file());
    assert!(
        run_root
            .join(RUN)
            .join("attachments")
            .join("notes.pdf")
            .is_file()
    );

    // The summary's folder links are relative to the package, so the links still work
    // once the folder is moved or copied somewhere else.
    let summary = fs::read_to_string(report.dir.join("summary.md")).unwrap();
    assert!(
        summary.contains("runs/uvic-geomag-survey/ground-walk-survey/2026-09-25-test-run"),
        "the summary links to the run inside the package:\n{summary}"
    );
    // A file really was counted (the run record, its folder, and its material).
    assert!(report.files >= 6, "{} files copied", report.files);
    assert!(report.bytes > 0);

    // The manifest lists the files it covers, including the summary.
    let manifest = fs::read_to_string(report.dir.join("MANIFEST.sha256")).unwrap();
    assert!(manifest.contains("  summary.md"), "{manifest}");
    assert!(
        manifest.contains(&format!("  runs/uvic-geomag-survey/{SOP}/{RUN}/photos/plot.jpg")),
        "{manifest}"
    );
}

#[test]
fn a_failed_copy_leaves_no_package_and_no_partial() {
    let scratch = Scratch::new("export-failure");
    let repo = scratch.repo();
    ended_with_material(&scratch, &repo);
    let dest = scratch.path("exports");

    // Make one copied file unreadable so the copy fails part way through. A test that
    // runs as root can read anything, so in that case there is nothing to prove here.
    let victim = scratch.path(&format!("runs/uvic-geomag-survey/{SOP}/{RUN}/events.jsonl"));
    let mut perms = fs::metadata(&victim).unwrap().permissions();
    perms.set_readonly(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o000);
    }
    fs::set_permissions(&victim, perms).unwrap();
    if fs::File::open(&victim).is_ok() {
        // Running as a user who can read anything; restore and skip.
        let mut perms = fs::metadata(&victim).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o644);
        }
        fs::set_permissions(&victim, perms).unwrap();
        return;
    }

    let cases = cases_repo();
    let cases_repo = cases.repo();
    let picked = vec![(SOP.to_owned(), RUN.to_owned())];
    let result = export::export_package(
        &repo,
        &cases_repo,
        &dest,
        "2026-09-29_143200",
        &ident,
        &meta(),
        Some(&picked),
    );

    // Restore the file so the scratch directory can be cleaned up.
    let mut perms = fs::metadata(&victim).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o644);
    }
    fs::set_permissions(&victim, perms).unwrap();

    assert!(result.is_err(), "an unreadable file must fail the export");
    assert!(
        !dest.join("export_2026-09-29_143200").exists(),
        "no final folder"
    );
    assert!(
        !dest.join("export_2026-09-29_143200.partial").exists(),
        "no half-written folder left behind"
    );
    // Nothing else was left in the destination either.
    let leftovers: Vec<_> = fs::read_dir(&dest)
        .map(|entries| entries.flatten().map(|e| e.file_name()).collect())
        .unwrap_or_default();
    assert!(leftovers.is_empty(), "leftover entries: {leftovers:?}");
}

#[test]
fn a_second_export_of_the_same_moment_gets_a_suffix() {
    let scratch = Scratch::new("export-suffix");
    let repo = scratch.repo();
    ended_with_material(&scratch, &repo);
    let dest = scratch.path("exports");

    let first = export_one(&repo, &dest, "same");
    let second = export_one(&repo, &dest, "same");

    assert_eq!(first.dir, dest.join("export_same"));
    assert_eq!(second.dir, dest.join("export_same-2"));
    assert!(first.dir.is_dir() && second.dir.is_dir());
}

#[test]
fn a_symlink_inside_a_run_is_not_followed() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let scratch = Scratch::new("export-symlink");
        let repo = scratch.repo();
        ended_with_material(&scratch, &repo);
        let dest = scratch.path("exports");

        // A secret outside the run, reached by a link inside it. The export must skip
        // the link rather than copy what it points at.
        let secret = scratch.path("secret.txt");
        fs::write(&secret, "do not package me\n").unwrap();
        let link = scratch.path(&format!("runs/uvic-geomag-survey/{SOP}/{RUN}/leak.txt"));
        symlink(&secret, &link).unwrap();

        let report = export_one(&repo, &dest, "linked");

        assert!(
            !report
                .dir
                .join(RUN_DIR)
                .join(RUN)
                .join("leak.txt")
                .exists(),
            "the link itself is not copied"
        );
        assert!(
            report.skipped.iter().any(|path| path.ends_with("leak.txt")),
            "{report:?}"
        );
        // The copied package holds no copy of the secret's bytes.
        let packaged = fs::read_dir(report.dir.join(RUN_DIR).join(RUN)).unwrap();
        for entry in packaged.flatten() {
            if entry.path().is_file() {
                let text = fs::read_to_string(entry.path()).unwrap_or_default();
                assert!(
                    !text.contains("do not package me"),
                    "{}",
                    entry.path().display()
                );
            }
        }
    }
}

#[test]
fn a_label_with_separators_or_dots_is_refused() {
    let scratch = Scratch::new("export-label");
    let repo = scratch.repo();
    ended_with_material(&scratch, &repo);
    let dest = scratch.path("exports");
    let cases = cases_repo();
    let cases_repo = cases.repo();

    for bad in ["a/b", "../evil", "a b", "", "naïve"] {
        let error = export::export_package(&repo, &cases_repo, &dest, bad, &ident, &meta(), None)
            .unwrap_err();
        assert!(
            matches!(error, ExportError::BadLabel(_)),
            "label {bad:?} must be refused, got {error:?}"
        );
    }
    assert!(!dest.exists(), "a refused label writes nothing at all");
}
