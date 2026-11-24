//! The simplified notebook works in an explicitly chosen, non-git data directory.
use std::fs;

use sop_core::RunEvent;
use sop_repo::{manifest, run, workspace};

mod common;
use common::Scratch;

const ID: &str = "note-2026-11-24-120000";

#[test]
fn a_title_is_enough_to_create_a_searchable_record_in_an_empty_directory() {
    let scratch = Scratch::empty("notebook-title");
    let repo = scratch.repo();
    run::create_note(
        &repo,
        ID,
        "温度漂移测试",
        "",
        "",
        "预热后采样",
        &run::RunMeta::default(),
    )
    .unwrap();
    let index = manifest::build(&repo);
    assert_eq!(index.runs.len(), 1);
    assert_eq!(
        index.runs[0].title.as_ref().unwrap().as_str(),
        Some("温度漂移测试")
    );
    assert!(index.runs[0].notebook);
    assert_eq!(index.runs[0].notes[0].text, "预热后采样");
    assert!(index.runs[0].editable);
    let (failed, report) = scratch.validate();
    assert!(!failed, "{report}");
}

#[test]
fn invalid_or_duplicate_notebooks_never_overwrite_a_record() {
    let scratch = Scratch::empty("notebook-invalid");
    let repo = scratch.repo();
    assert!(
        run::create_note(
            &repo,
            "../outside",
            "test",
            "",
            "",
            "",
            &run::RunMeta::default()
        )
        .is_err()
    );
    assert!(run::create_note(&repo, ID, "\n", "", "", "", &run::RunMeta::default()).is_err());
    assert!(!scratch.path("runs").exists());
    let first = run::create_note(&repo, ID, "test", "", "", "", &run::RunMeta::default()).unwrap();
    let before = fs::read(&first.events_path).unwrap();
    assert!(
        run::create_note(
            &repo,
            ID,
            "replacement",
            "",
            "",
            "",
            &run::RunMeta::default()
        )
        .is_err()
    );
    assert_eq!(fs::read(first.events_path).unwrap(), before);
}

#[test]
fn notes_files_tags_and_remarks_refresh_the_live_index_and_export() {
    let scratch = Scratch::empty("notebook-index");
    let repo = scratch.repo();
    let loaded = run::create_note(
        &repo,
        ID,
        "测试",
        "人员: A",
        "实验室 #1",
        "",
        &run::RunMeta::default(),
    )
    .unwrap();
    let incoming = scratch.path("raw.csv");
    fs::write(&incoming, "time,value\n2026-11-24T11:00:00Z,2\n").unwrap();
    let attachment = run::attach(&repo, run::NOTE_SOP, ID, None, &incoming).unwrap();
    run::record(
        &repo,
        run::NOTE_SOP,
        ID,
        &RunEvent::NoteAdded {
            at: String::new(),
            step: None,
            text: "发现干扰峰".into(),
        },
    )
    .unwrap();
    run::annotate_file(
        &repo,
        run::NOTE_SOP,
        ID,
        &attachment.path,
        "前10秒为预热",
        vec!["温漂".into(), " 温漂 ".into(), "复测".into()],
    )
    .unwrap();
    let index = manifest::build(&repo);
    assert_eq!(index.runs[0].log_count, 1);
    assert_eq!(index.runs[0].notes[0].text, "发现干扰峰");
    assert_eq!(index.runs[0].files[0].tags, vec!["温漂", "复测"]);
    assert_eq!(index.runs[0].files[0].file_time, "2026-11-24T11:00:00Z");
    let export = run::record_document(&repo, run::NOTE_SOP, ID).unwrap();
    assert!(export.contains("前10秒为预热"));
    assert!(export.contains("温漂"));
    assert!(
        !export.contains("events_sealed_bytes"),
        "an ongoing notebook has no seal yet"
    );
    assert!(run::attachment_path(&repo, run::NOTE_SOP, ID, "../other.csv").is_err());
    assert!(
        fs::read_to_string(loaded.record_file)
            .unwrap()
            .contains("发现干扰峰")
    );
    let (failed, report) = scratch.validate();
    assert!(!failed, "{report}");
}

#[test]
fn post_run_metadata_preserves_the_sealed_prefix_and_file_bytes() {
    let scratch = Scratch::empty("notebook-seal");
    let repo = scratch.repo();
    run::create_note(&repo, ID, "测试", "", "", "", &run::RunMeta::default()).unwrap();
    let source = scratch.path("raw.log");
    fs::write(&source, "original bytes").unwrap();
    let attachment = run::attach(&repo, run::NOTE_SOP, ID, None, &source).unwrap();
    let ended = run::end(&repo, run::NOTE_SOP, ID, "complete", None).unwrap();
    let before = fs::read(&ended.events_path).unwrap();
    run::annotate_file(
        &repo,
        run::NOTE_SOP,
        ID,
        &attachment.path,
        "异常段",
        vec!["待检查".into()],
    )
    .unwrap();
    let after = fs::read(&ended.events_path).unwrap();
    assert!(after.starts_with(&before));
    assert_eq!(
        fs::read(repo.resolve(&attachment.path)).unwrap(),
        b"original bytes"
    );
    let (failed, report) = scratch.validate();
    assert!(!failed, "{report}");
}

#[test]
fn loose_logs_have_persistent_tags_without_changing_the_logs() {
    let scratch = Scratch::empty("workspace-logs");
    fs::create_dir_all(scratch.path("logs/2026-11")).unwrap();
    let path = scratch.path("logs/2026-11/raw.log");
    fs::write(&path, "raw").unwrap();
    let repo = scratch.repo();
    workspace::annotate_log(
        &repo,
        "logs/2026-11/raw.log",
        "天气变化",
        &["外场".into(), "复测".into()],
    )
    .unwrap();
    let index = manifest::build(&repo);
    assert_eq!(index.workspace_files.len(), 1);
    assert_eq!(index.workspace_files[0].tags, vec!["外场", "复测"]);
    assert_eq!(index.workspace_files[0].remark, "天气变化");
    assert_eq!(fs::read_to_string(path).unwrap(), "raw");
    let (failed, report) = scratch.validate();
    assert!(!failed, "unassociated log folders are supported: {report}");
    assert!(workspace::annotate_log(&repo, "../raw.log", "x", &[]).is_err());
    assert!(workspace::annotate_log(&repo, "raw.log", "x", &[]).is_err());
}

#[test]
fn invalid_existing_log_metadata_is_reported_and_not_overwritten() {
    let scratch = Scratch::empty("workspace-corrupt");
    fs::create_dir_all(scratch.path("logs")).unwrap();
    fs::create_dir_all(scratch.path(".field-sop")).unwrap();
    fs::write(scratch.path("logs/raw.log"), "raw").unwrap();
    fs::write(scratch.path(workspace::REMARKS_FILE), "not json").unwrap();
    let repo = scratch.repo();
    assert!(workspace::annotate_log(&repo, "logs/raw.log", "x", &[]).is_err());
    assert!(manifest::build(&repo).workspace_error.is_some());
    assert_eq!(scratch.read(workspace::REMARKS_FILE), "not json");
}

#[test]
fn log_imports_reuse_identical_files_and_never_overwrite_different_bytes() {
    let scratch = Scratch::empty("workspace-import");
    fs::create_dir_all(scratch.path("incoming")).unwrap();
    let source = scratch.path("incoming/raw.log");
    fs::write(&source, "one").unwrap();
    let repo = scratch.repo();
    assert_eq!(
        workspace::import_log(&repo, &source).unwrap(),
        "logs/raw.log"
    );
    assert_eq!(
        workspace::import_log(&repo, &source).unwrap(),
        "logs/raw.log"
    );
    fs::write(&source, "two").unwrap();
    assert_eq!(
        workspace::import_log(&repo, &source).unwrap(),
        "logs/raw-2.log"
    );
    assert_eq!(scratch.read("logs/raw.log"), "one");
    assert_eq!(scratch.read("logs/raw-2.log"), "two");
}

#[test]
fn a_full_export_includes_unassociated_logs_and_their_tags() {
    let scratch = Scratch::empty("workspace-export");
    fs::create_dir_all(scratch.path("logs")).unwrap();
    fs::write(scratch.path("logs/raw.log"), "raw").unwrap();
    let repo = scratch.repo();
    workspace::annotate_log(&repo, "logs/raw.log", "说明", &["外场".into()]).unwrap();
    let meta = sop_repo::export::repo_meta(&repo, "now".into());
    let report = sop_repo::export::export_package(
        &repo,
        &repo,
        &scratch.path("exports"),
        "test",
        &str::to_owned,
        &meta,
        None,
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(report.dir.join("logs/raw.log")).unwrap(),
        "raw"
    );
    assert!(
        fs::read_to_string(report.dir.join(workspace::REMARKS_FILE))
            .unwrap()
            .contains("外场")
    );
    assert!(
        sop_repo::export::export_package(
            &repo,
            &repo,
            &scratch.path("logs/exports"),
            "bad",
            &str::to_owned,
            &meta,
            None
        )
        .is_err()
    );
}

#[test]
fn log_time_ranges_handle_offsets_and_out_of_order_rows() {
    let scratch = Scratch::empty("notebook-log-time");
    let repo = scratch.repo();
    run::create_note(&repo, ID, "test", "", "", "", &run::RunMeta::default()).unwrap();
    let path = scratch.path("offset.csv");
    fs::write(&path, "time,value\n2026-11-24T10:00:00+08:00,1\n2026-11-24T12:00:00+08:00,2\n2026-11-24T11:00:00+08:00,3\n").unwrap();
    let attachment = run::attach(&repo, run::NOTE_SOP, ID, None, &path).unwrap();
    assert_eq!(attachment.t_min.as_deref(), Some("2026-11-24T02:00:00Z"));
    assert_eq!(attachment.t_max.as_deref(), Some("2026-11-24T04:00:00Z"));
}

#[test]
fn markdown_file_remarks_cannot_create_fake_step_results() {
    let scratch = Scratch::empty("notebook-remark-fence");
    let repo = scratch.repo();
    run::create_note(&repo, ID, "test", "", "", "", &run::RunMeta::default()).unwrap();
    let source = scratch.path("raw.log");
    fs::write(&source, "raw").unwrap();
    let attachment = run::attach(&repo, run::NOTE_SOP, ID, None, &source).unwrap();
    run::annotate_file(
        &repo,
        run::NOTE_SOP,
        ID,
        &attachment.path,
        "```yaml result\nstep: fake-step\nstatus: done\n```",
        vec!["复测".into()],
    )
    .unwrap();
    let document = run::record_document(&repo, run::NOTE_SOP, ID).unwrap();
    assert!(
        sop_core::Document::parse(&document)
            .unwrap()
            .results
            .is_empty()
    );
    let (failed, report) = scratch.validate();
    assert!(!failed, "{report}");
}

#[test]
fn opening_and_indexing_an_old_data_directory_does_not_move_its_logs() {
    let scratch = Scratch::empty("notebook-no-migration");
    let repo = sop_repo::Repo::open_data_directory(&scratch.root);
    let first = run::create_note(
        &repo,
        ID,
        "old record",
        "",
        "",
        "",
        &run::RunMeta::default(),
    )
    .unwrap();
    let source = scratch.path("raw.log");
    fs::write(&source, "raw").unwrap();
    let attachment = run::attach(&repo, run::NOTE_SOP, ID, None, &source).unwrap();
    // A project is added later. Merely choosing this directory must not move the
    // old flat records or break the relative paths already in their event logs.
    fs::copy(
        common::source_root().join("project.md"),
        scratch.path("project.md"),
    )
    .unwrap();
    let reopened = sop_repo::Repo::open_data_directory(&scratch.root);
    let index = manifest::build(&reopened);
    assert!(first.dir.is_dir());
    assert!(first.record_file.is_file());
    assert!(reopened.resolve(&attachment.path).is_file());
    assert_eq!(
        run::load(&reopened, run::NOTE_SOP, ID).unwrap().dir,
        first.dir
    );
    assert!(!index.runs[0].files[0].missing);
    let meta = sop_repo::export::repo_meta(&reopened, "now".into());
    let exported = sop_repo::export::export_package(
        &reopened,
        &reopened,
        &scratch.path("exports"),
        "old",
        &str::to_owned,
        &meta,
        None,
    )
    .unwrap();
    assert!(exported.dir.join(&attachment.path).is_file());
    assert!(
        first.dir.is_dir(),
        "export also leaves the source layout alone"
    );
}

#[test]
fn a_project_id_cannot_redirect_notebook_writes_outside_the_working_directory() {
    let scratch = Scratch::empty("notebook-scope");
    fs::write(
        scratch.path("project.md"),
        "---\nkind: project\nproject_id: ../../outside\ntitle: test\n---\n",
    )
    .unwrap();
    let repo = sop_repo::Repo::open_data_directory(&scratch.root);
    assert!(run::create_note(&repo, ID, "test", "", "", "", &run::RunMeta::default()).is_err());
    assert!(!scratch.path("runs").exists());
}
