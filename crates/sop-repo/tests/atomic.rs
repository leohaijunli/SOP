//! The durability promise of `atomic::write`: a reader sees the old file or the new one,
//! never a fragment, and no temporary sibling is left behind for `git status` to pick up.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use sop_repo::atomic;

fn scratch_dir(name: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "field-sop-atomic-{}-{serial}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_write_leaves_exactly_the_target_and_no_temporary() {
    let dir = scratch_dir("clean");
    let path = dir.join("nested").join("record.md");
    // The parent does not exist yet; the writer has to make it.
    atomic::write(&path, "hello\n").unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), "hello\n");
    let siblings: Vec<String> = fs::read_dir(path.parent().unwrap())
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        siblings,
        vec!["record.md".to_string()],
        "the temporary is renamed away, not left behind"
    );
}

#[test]
fn an_overwrite_replaces_the_whole_file() {
    let dir = scratch_dir("overwrite");
    let path = dir.join("events.jsonl");
    atomic::write(&path, "first version, much longer\n").unwrap();
    atomic::write(&path, "second\n").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");

    let _ = fs::remove_dir_all(&dir);
}
