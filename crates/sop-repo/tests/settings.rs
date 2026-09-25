//! The settings file, and the one thing the application refuses to keep in it.
//!
//! Settings are the app's own preferences. They are not content, they are not committed,
//! and the hard rule is that no value in them may be a remote URL or a credential - a
//! file only ever copied by hand is a file that leaks only by hand.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use sop_core::Settings;
use sop_repo::{git, settings};

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
            "field-sop-settings-{}-{serial}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        Self { root }
    }

    /// A settings path under a directory that does not exist yet.
    fn path(&self) -> PathBuf {
        self.root.join("field-sop").join("settings.json")
    }

    fn entries(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.root.join("field-sop")) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

#[test]
fn a_missing_file_loads_the_defaults_rather_than_failing() {
    let scratch = Scratch::new("missing");
    assert_eq!(
        settings::load(&scratch.path()).unwrap(),
        Settings::default()
    );
    assert!(!scratch.path().exists(), "loading must not create the file");
}

#[test]
fn settings_survive_a_round_trip_and_are_written_atomically() {
    let scratch = Scratch::new("round-trip");
    let path = scratch.path();

    let mut settings = Settings::default();
    settings.set("repository", "/home/leo/field-sop").unwrap();
    settings.set("help-open", "false").unwrap();
    settings.remember("/home/leo/field-sop");
    settings::save(&path, &settings).unwrap();

    assert_eq!(settings::load(&path).unwrap(), settings);
    assert_eq!(
        scratch.entries(),
        vec!["settings.json".to_owned()],
        "a half-written temporary file must never be left behind"
    );
}

#[test]
fn a_broken_file_is_reported_with_its_path() {
    let scratch = Scratch::new("broken");
    let path = scratch.path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{ not json").unwrap();

    let error = settings::load(&path).unwrap_err().to_string();
    assert!(error.contains("settings.json"), "{error}");
}

#[test]
fn a_key_written_by_another_build_is_kept() {
    let scratch = Scratch::new("future");
    let path = scratch.path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "{\n  \"remote\": \"origin\",\n  \"future_key\": \"kept\"\n}\n",
    )
    .unwrap();

    let settings = settings::load(&path).unwrap();
    assert_eq!(
        settings.get("future_key").as_deref(),
        Some("\"kept\""),
        "an unknown key is readable"
    );
    settings::save(&path, &settings).unwrap();
    assert!(
        fs::read_to_string(&path).unwrap().contains("future_key"),
        "saving must not drop a key this build does not know"
    );
}

#[test]
fn a_remote_url_is_not_a_setting() {
    let mut settings = Settings::default();
    for url in [
        "https://github.com/leo/geomag-field-sop.git",
        "git@github.com:leo/geomag-field-sop.git",
        "git://example.org/repo.git",
    ] {
        let error = settings.set("remote", url).unwrap_err().to_string();
        assert!(error.contains("remote"), "{error}");
        assert_eq!(settings.remote, "origin", "the setting must not change");
    }
    settings.set("remote", "upstream").unwrap();
    assert_eq!(settings.remote, "upstream");
}

#[test]
fn a_token_in_a_remote_url_is_recognised() {
    for url in [
        "https://ghp_example@github.com/org/repo.git",
        "https://user:token@github.com/org/repo.git",
        "http://user:password@example.org/repo.git",
        "ssh://user:password@example.org/repo.git",
    ] {
        assert!(git::has_credentials(url), "{url}");
    }
}

#[test]
fn the_credential_check_says_what_it_does_not_look_at() {
    // The check reads the authority, so a secret hidden in a query string is not found.
    // It is a warning about the common mistake rather than a guarantee, and a test that
    // records the limit is more useful than one that pretends there is none.
    assert!(!git::has_credentials(
        "https://github.com/org/repo.git?token=abc"
    ));
}

#[test]
fn an_ssh_login_name_is_not_a_credential() {
    for url in [
        "git@github.com:org/repo.git",
        "ssh://git@github.com/org/repo.git",
        "https://github.com/org/repo.git",
        "file:///srv/git/repo.git",
        "/srv/git/repo.git",
    ] {
        assert!(!git::has_credentials(url), "{url} is not a credential");
    }
}
