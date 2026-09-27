//! Publishing the working copy: `git add -A`, commit, push.
//!
//! Every test builds its own throwaway repository, so nothing here depends on the
//! machine's git configuration and nothing runs against a network. The "remote" is a
//! bare repository in a sibling directory, which is what makes the push real without
//! being remote.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use sop_repo::git::{self, GitError};

struct Scape {
    root: PathBuf,
}

impl Drop for Scape {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git_in(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

impl Scape {
    /// A working copy with one commit, a bare "origin" remote, and a clean tree.
    fn new(name: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "field-sop-git-{}-{serial}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let scape = Self { root };

        let bare = scape.root.join("origin.git");
        fs::create_dir_all(&bare).unwrap();
        git_in(&bare, &["init", "--bare", "-b", "main"]);

        let work = scape.work();
        fs::create_dir_all(&work).unwrap();
        git_in(&work, &["init", "-b", "main"]);
        git_in(&work, &["config", "user.name", "Test Operator"]);
        git_in(&work, &["config", "user.email", "operator@example.invalid"]);
        git_in(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
        scape
    }

    fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    fn commit_count(&self) -> usize {
        git_in(&self.work(), &["rev-list", "--count", "HEAD"])
            .parse()
            .unwrap_or(0)
    }
}

#[test]
fn a_publish_commits_the_working_copy_and_pushes_it() {
    let scape = Scape::new("publish");
    let work = scape.work();
    fs::write(work.join("project.md"), "kind: project\n").unwrap();

    let report = git::commit_and_push(&work, "origin", "field-sop: publish").unwrap();

    assert_eq!(report.changed, 1);
    assert_eq!(report.branch.as_deref(), Some("main"));
    assert!(
        report.commit.is_some(),
        "a publish with changes makes a commit"
    );
    assert!(!report.up_to_date);
    assert_eq!(scape.commit_count(), 1);

    // The bare remote has the commit, which is what makes this a push and not just a
    // commit: `ls-remote` asks the remote, not the local repository.
    let local = git_in(&work, &["rev-parse", "HEAD"]);
    let remote = git_in(&work, &["ls-remote", "origin", "HEAD"]);
    assert_eq!(remote.split_whitespace().next(), Some(local.as_str()));
}

#[test]
fn a_second_publish_with_nothing_new_reports_up_to_date() {
    let scape = Scape::new("steady");
    let work = scape.work();
    fs::write(work.join("project.md"), "kind: project\n").unwrap();
    git::commit_and_push(&work, "origin", "first").unwrap();

    let report = git::commit_and_push(&work, "origin", "second").unwrap();

    assert_eq!(report.changed, 0);
    assert_eq!(report.commit, None);
    assert!(report.up_to_date, "log was {:?}", report.log);
    assert_eq!(scape.commit_count(), 1);
}

#[test]
fn a_push_that_fails_has_still_committed() {
    let scape = Scape::new("no-remote");
    let work = scape.work();
    fs::write(work.join("project.md"), "kind: project\n").unwrap();

    let error = git::commit_and_push(&work, "upstream", "field-sop: publish").unwrap_err();

    assert!(matches!(error, GitError::CommandFailed { .. }), "{error:?}");
    assert!(git::current_commit(&work).is_some(), "the commit happened");
}

#[test]
fn a_directory_that_is_not_a_repository_is_rejected() {
    let scape = Scape::new("plain");
    let plain = scape.root.join("plain");
    fs::create_dir_all(&plain).unwrap();

    let error = git::commit_and_push(&plain, "origin", "message").unwrap_err();

    assert!(matches!(error, GitError::NotARepository(_)), "{error:?}");
}
