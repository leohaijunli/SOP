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

#[test]
fn git_settings_are_kept_in_the_working_copy_and_travel_with_it() {
    let scratch = Scratch::new("repo-scope");
    fs::create_dir_all(&scratch.root).unwrap();
    let mut settings = Settings::default();

    // What `sop settings set remote upstream` does: write the repository's file.
    settings.set("remote", "upstream").unwrap();
    settings.set("branch", "field").unwrap();
    settings::save_repo(&scratch.root, &settings).unwrap();

    let text = fs::read_to_string(settings::repo_path(&scratch.root)).unwrap();
    assert!(text.contains("\"remote\": \"upstream\""), "{text}");
    assert!(text.contains("\"branch\": \"field\""), "{text}");
    assert!(
        !text.contains("repository"),
        "a repository must never carry the machine's path: {text}"
    );

    // A second machine with its own settings picks the repository's values up.
    let mut other_machine = Settings::default();
    other_machine.set("repository", "/home/somebody/else").unwrap();
    settings::apply_repo(&scratch.root, &mut other_machine).unwrap();
    assert_eq!(other_machine.remote, "upstream");
    assert_eq!(other_machine.branch.as_deref(), Some("field"));
    assert_eq!(
        other_machine.repository.as_deref(),
        Some("/home/somebody/else"),
        "the repository says nothing about where the machine keeps its copy"
    );
}

#[test]
fn a_repository_that_declares_nothing_inherits_the_machine() {
    let scratch = Scratch::new("repo-absent");
    fs::create_dir_all(&scratch.root).unwrap();
    let mut settings = Settings::default();
    settings.set("remote", "origin").unwrap();
    settings.set("branch", "main").unwrap();

    // No `.field-sop/settings.json` at all: not an error, and nothing is overwritten.
    settings::apply_repo(&scratch.root, &mut settings).unwrap();
    assert_eq!(settings.remote, "origin");
    assert_eq!(settings.branch.as_deref(), Some("main"));
    assert!(!settings::repo_path(&scratch.root).exists());
}

#[test]
fn clearing_a_branch_in_the_repository_beats_the_machine() {
    let scratch = Scratch::new("repo-clear");
    fs::create_dir_all(&scratch.root).unwrap();
    let mut settings = Settings::default();
    settings.set("branch", "main").unwrap();
    settings.unset("branch").unwrap();
    settings::save_repo(&scratch.root, &settings).unwrap();
    assert!(
        fs::read_to_string(settings::repo_path(&scratch.root))
            .unwrap()
            .contains("\"branch\": null"),
        "an unset branch is written as null so clearing it is a visible diff"
    );

    let mut machine = Settings::default();
    machine.set("branch", "some-other-branch").unwrap();
    settings::apply_repo(&scratch.root, &mut machine).unwrap();
    assert_eq!(
        machine.branch, None,
        "the repository's null clears the machine's branch"
    );
}

#[test]
fn a_repository_file_that_is_not_an_object_is_reported() {
    let scratch = Scratch::new("repo-bad");
    let path = settings::repo_path(&scratch.root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "[1, 2, 3]\n").unwrap();

    let mut settings = Settings::default();
    let error = settings::apply_repo(&scratch.root, &mut settings).unwrap_err().to_string();
    assert!(error.contains("settings.json"), "{error}");
}

#[test]
fn the_sensor_inventory_round_trips_and_merges_repeated_models() {
    use sop_core::settings::{DEFAULT_SENSORS, format_sensors, parse_sensors};

    assert_eq!(
        parse_sensors(DEFAULT_SENSORS),
        vec![
            sop_core::settings::SensorStock { model: "UAS-MAG".to_owned(), serials: vec![] },
            sop_core::settings::SensorStock { model: "RM3100".to_owned(), serials: vec![] },
        ],
        "the default names the models the form offers"
    );

    // A model written twice keeps both sets of serials rather than losing one, and a
    // serial repeated inside one model is only listed once.
    let stocks = parse_sensors("UAS-MAG: 1001, 1002; RM3100; UAS-MAG: 1001, 1003 ");
    assert_eq!(stocks.len(), 2, "a repeated model is merged: {stocks:?}");
    assert_eq!(stocks[0].serials, vec!["1001", "1002", "1003"]);
    assert_eq!(stocks[1].model, "RM3100");
    assert!(stocks[1].serials.is_empty());
    assert_eq!(format_sensors(&stocks), "UAS-MAG: 1001, 1002, 1003; RM3100");
    assert_eq!(parse_sensors(&format_sensors(&stocks)), stocks);
}

#[test]
fn the_sensor_setting_defaults_normalises_and_can_be_cleared() {
    let mut settings = Settings::default();
    assert_eq!(settings.get("sensors").as_deref(), Some("UAS-MAG; RM3100"));

    // Writing a serial back after a run normalises the text on the way in.
    settings.set("sensors", "UAS-MAG: 1001 ; RM3100: 3001,3002").unwrap();
    assert_eq!(
        settings.get("sensors").as_deref(),
        Some("UAS-MAG: 1001; RM3100: 3001, 3002")
    );
    assert!(settings.to_json().contains("sensors"), "the value is persisted");

    // A value with no model at all is rejected rather than stored as an empty picker.
    let error = settings.set("sensors", "  ;  ").unwrap_err().to_string();
    assert!(error.contains("sensors"), "{error}");
    assert_eq!(
        settings.get("sensors").as_deref(),
        Some("UAS-MAG: 1001; RM3100: 3001, 3002"),
        "a rejected value leaves the setting alone"
    );

    // Clearing the box in the settings page is how the operator goes back to the
    // built-in list; it must not fail the whole save.
    settings.set("sensors", "   ").unwrap();
    assert_eq!(settings.get("sensors").as_deref(), Some("UAS-MAG; RM3100"));
    settings.set("sensors", "UAS-MAG: 1001").unwrap();
    settings.unset("sensors").unwrap();
    assert_eq!(settings.get("sensors").as_deref(), Some("UAS-MAG; RM3100"));
}
