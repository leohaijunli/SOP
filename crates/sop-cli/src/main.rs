//! Command line interface for a field-sop content repository.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use sop_core::{Settings, Value};
use sop_repo::{Repo, git, manifest, project, report::display_path, settings, validate};

mod preview;

#[derive(Debug, Parser)]
#[command(
    name = "sop",
    version,
    about = "Tooling for a field-sop content repository",
    long_about = "Checks and packages the Markdown content that describes field procedures,\n\
                  checklists, and run records. The format is specified in SPEC.md."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check content against SPEC.md. Exits non-zero if any error is found.
    Validate {
        /// Repository root. Defaults to the current directory.
        #[arg(long, value_name = "PATH")]
        repo: Option<PathBuf>,
        /// Specific files to check. Defaults to the whole repository.
        files: Vec<PathBuf>,
    },
    /// Build dist/manifest.json, the single file the field app loads.
    Index {
        /// Repository root. Defaults to the current directory.
        #[arg(long, value_name = "PATH")]
        repo: Option<PathBuf>,
        /// Output path. Defaults to dist/manifest.json under the repository root.
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Look at the layout and the manifest in a browser, without the desktop app.
    Preview {
        /// Repository root. Defaults to the current directory.
        #[arg(long, value_name = "PATH")]
        repo: Option<PathBuf>,
        /// Loopback port to listen on.
        #[arg(long, value_name = "PORT", default_value_t = 8731)]
        port: u16,
    },
    /// Show the repository's identity, its git state, and what content it holds.
    Status {
        /// Repository root. Defaults to the configured working copy, then to here.
        #[arg(long, value_name = "PATH")]
        repo: Option<PathBuf>,
    },
    /// Show or set the repository's git remote URL. The URL is stored by git.
    Remote {
        /// The URL to set. Omit to show the current one.
        url: Option<String>,
        /// Remote name. Defaults to the configured remote.
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        /// Repository root. Defaults to the configured working copy, then to here.
        #[arg(long, value_name = "PATH")]
        repo: Option<PathBuf>,
    },
    /// Show or change the project's identity, which lives in project.md.
    Project {
        #[command(subcommand)]
        action: Option<ProjectAction>,
        /// Repository root. Defaults to the configured working copy, then to here.
        #[arg(long, value_name = "PATH", global = true)]
        repo: Option<PathBuf>,
    },
    /// Show or change this application's own settings.
    Settings {
        #[command(subcommand)]
        action: Option<SettingsAction>,
    },
}

#[derive(Debug, Subcommand)]
enum ProjectAction {
    /// Change one field of project.md, for example: sop project set title "Renfrew 2026".
    Set {
        /// Field name. Run `sop project` to list them.
        key: String,
        /// New value. Everything else in the file is left alone.
        value: String,
    },
}

#[derive(Debug, Subcommand)]
enum SettingsAction {
    /// Print the path of the settings file.
    Path,
    /// Change a setting.
    Set {
        /// Setting name. Run `sop settings` to list them.
        key: String,
        value: String,
    },
    /// Return a setting to its default.
    Unset { key: String },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Validate { repo, files } => run_validate(repo, files),
        Command::Index { repo, out } => run_index(repo, out),
        Command::Preview { repo, port } => run_preview(repo, port),
        Command::Status { repo } => run_status(repo),
        Command::Remote { url, name, repo } => run_remote(repo, name, url),
        Command::Project { action, repo } => run_project(repo, action),
        Command::Settings { action } => run_settings(action),
    }
}

fn run_preview(repo: Option<PathBuf>, port: u16) -> ExitCode {
    let (_, settings) = load_settings();
    let repo = repository_for(repo, &settings);
    if let Err(error) = preview::serve(repo.root(), port) {
        eprintln!("could not serve the preview on port {port}: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Resolve a path the way the reference tooling does: relative to the current directory.
fn absolutise(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(path),
        Err(_) => path,
    }
}

fn repo_root(repo: Option<PathBuf>) -> PathBuf {
    let root = repo.unwrap_or_else(|| PathBuf::from("."));
    // Walk lexical `.` and `..` out of the root so that messages and path comparisons
    // do not depend on how the caller spelled it.
    let root = absolutise(root);
    match root.canonicalize() {
        Ok(real) => real,
        Err(_) => root,
    }
}

/// Load the application settings, reporting a broken file rather than ignoring it.
fn load_settings() -> (PathBuf, Settings) {
    let path = settings::default_path();
    match settings::load(&path) {
        Ok(settings) => (path, settings),
        Err(error) => {
            eprintln!("{error}");
            eprintln!("continuing with defaults; fix the file or run `sop settings path`");
            (path, Settings::default())
        }
    }
}

/// The working copy to act on: the one named on the command line, then the configured
/// one, then the current directory.
fn repository_for(explicit: Option<PathBuf>, settings: &Settings) -> Repo {
    if let Some(path) = explicit {
        return Repo::open(repo_root(Some(path)));
    }
    if let Some(configured) = &settings.repository {
        return Repo::open(repo_root(Some(PathBuf::from(configured))));
    }
    Repo::open(repo_root(None))
}

fn run_status(repo: Option<PathBuf>) -> ExitCode {
    let (settings_path, settings) = load_settings();
    let repo = repository_for(repo, &settings);
    let root = repo.root();
    let mut readable = true;

    println!("working copy   {}", root.display());
    println!("settings       {}", settings_path.display());

    match project::load(&repo) {
        Ok(Some(loaded)) => {
            let front = &loaded.doc.front;
            let title = front.str("title").flatten().unwrap_or("(no title)");
            let id = front
                .str("project_id")
                .flatten()
                .unwrap_or("(no project_id)");
            println!("project        {title} ({id})");
            if let Some(institution) = front.str("institution").flatten() {
                println!("institution    {institution}");
            }
        }
        Ok(None) => println!("project        no project.md; see templates/project.md"),
        Err(error) => {
            println!("project        unreadable: {error}");
            readable = false;
        }
    }

    let state = git::state(root, &settings.remote);
    if !state.is_repository {
        println!("git            not a working copy; sop reads and writes the files all the same");
    } else {
        println!(
            "branch         {}",
            state.branch.as_deref().unwrap_or("(detached)")
        );
        match &state.url {
            Some(url) => println!("remote         {} {}", state.remote, url),
            None => println!("remote         {} (no URL configured)", state.remote),
        }
        let sync = match (state.ahead, state.behind) {
            (Some(ahead), Some(behind)) => format!("{ahead} ahead, {behind} behind"),
            _ => "no upstream to compare with".to_owned(),
        };
        println!(
            "sync           {}{}",
            if state.dirty {
                "uncommitted changes; "
            } else {
                ""
            },
            sync
        );
        if state.url_has_credentials {
            println!(
                "               warning: that URL carries a username or a token, and anyone \
                 who can read the working copy can read it"
            );
        }
    }

    let found = repo.discover();
    println!(
        "content        {} procedure(s), {} checklist(s), {} run record(s), {} help page(s)",
        found.procedures.len(),
        found.checklists.len(),
        found.runs.len(),
        found.help.len()
    );
    if !found.log_dirs.is_empty() {
        let count = found.log_dirs.len();
        println!(
            "logs           {count} run director{} holding attachments",
            if count == 1 { "y" } else { "ies" }
        );
    }

    if readable {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run_remote(repo: Option<PathBuf>, name: Option<String>, url: Option<String>) -> ExitCode {
    let (_, settings) = load_settings();
    let repo = repository_for(repo, &settings);
    let root = repo.root();
    let explicit = name.is_some();
    let name = name.unwrap_or_else(|| settings.remote.clone());

    if !git::state(root, &name).is_repository {
        eprintln!(
            "{} is not a git working copy; run `git init` there first, or point --repo at one",
            root.display()
        );
        return ExitCode::FAILURE;
    }

    let url = match url {
        Some(url) => url,
        // No URL given: report the one git holds, and say nothing about changing it.
        None => {
            let state = git::state(root, &name);
            return match state.url {
                Some(current) => {
                    println!("{name}\t{current}");
                    if state.url_has_credentials {
                        println!(
                            "warning: that URL carries a username or a token, and the app has no \
                         way to keep it out of a copy of the repository"
                        );
                    }
                    ExitCode::SUCCESS
                }
                None => {
                    println!("{name} has no URL yet");
                    ExitCode::from(2)
                }
            };
        }
    };

    let url = url.trim();
    if url.is_empty() {
        eprintln!("the URL must not be empty");
        return ExitCode::FAILURE;
    }
    match git::set_remote_url(root, &name, url) {
        Ok(()) => {
            println!("{name}\t{url}");
            if git::has_credentials(url) {
                println!(
                    "warning: that URL carries a username or a token; it now sits in .git/config, \
                     where git will keep it but where any copy of the directory carries it too"
                );
            }
            if explicit && settings.remote != name {
                println!("run `sop settings set remote {name}` to make this the default");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_settings(action: Option<SettingsAction>) -> ExitCode {
    let (path, mut settings) = load_settings();

    let Some(action) = action else {
        println!("settings   {}", path.display());
        if !path.exists() {
            println!("           no file yet; these are the defaults");
        }
        for (key, description) in sop_core::settings::KEYS {
            let value = settings.get(key).unwrap_or_else(|| "(unset)".to_owned());
            println!("  {key:<20} {value:<28} {description}");
        }
        println!(
            "\nThe remote URL is kept by git, in the repository's own configuration, so that no \
             copy of this file can leak a token."
        );
        return ExitCode::SUCCESS;
    };

    match action {
        SettingsAction::Path => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        SettingsAction::Set { key, value } => {
            if let Err(error) = settings.set(&key, &value) {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
            match settings::save(&path, &settings) {
                Ok(()) => {
                    println!("{} = {}", key, settings.get(&key).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
        SettingsAction::Unset { key } => {
            if let Err(error) = settings.unset(&key) {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
            match settings::save(&path, &settings) {
                Ok(()) => {
                    println!(
                        "{key} = {}",
                        settings.get(&key).unwrap_or_else(|| "(unset)".to_owned())
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn run_project(repo: Option<PathBuf>, action: Option<ProjectAction>) -> ExitCode {
    let (_, settings) = load_settings();
    let repo = repository_for(repo, &settings);

    let Some(action) = action else {
        return match project::load(&repo) {
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
            Ok(None) => {
                println!("{} has no project.md", repo.root().display());
                println!("copy templates/project.md into place, then set its fields");
                ExitCode::from(2)
            }
            Ok(Some(loaded)) => {
                println!("{}", display_path(repo.root(), &loaded.path));
                for (key, description) in project::FIELDS {
                    let value = match loaded.doc.front.get(key) {
                        Some(Value::String(text)) => text.clone(),
                        Some(_) => "(not text)".to_owned(),
                        None => "(unset)".to_owned(),
                    };
                    // A one-sentence summary does not fit in a column, so it goes on its
                    // own line rather than pushing the description out of alignment.
                    if value.chars().count() > 32 {
                        println!("  {key:<14} {value}");
                        println!("  {:<14} {description}", "");
                    } else {
                        println!("  {key:<14} {value:<32} {description}");
                    }
                }
                ExitCode::SUCCESS
            }
        };
    };

    match action {
        ProjectAction::Set { key, value } => match project::set(&repo, &key, &value) {
            Ok(path) => {
                println!(
                    "{}: {key} = {}",
                    display_path(repo.root(), &path),
                    value.trim()
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
    }
}

fn run_validate(repo: Option<PathBuf>, files: Vec<PathBuf>) -> ExitCode {
    let (_, settings) = load_settings();
    let repo = repository_for(repo, &settings);
    let files: Vec<PathBuf> = files.into_iter().map(absolutise).collect();
    let validation = validate::validate(&repo, &files);
    let report = validation.report;

    if files.is_empty() && validation.files_checked == 0 {
        eprintln!(
            "no content found under {}; expected procedures/, checklists/, or runs/ \
             (use --repo to point at a repository root)",
            repo.root().display()
        );
        return ExitCode::from(2);
    }

    print!("{}", report.render(repo.root()));
    if report.has_errors() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_index(repo: Option<PathBuf>, out: Option<PathBuf>) -> ExitCode {
    let (_, settings) = load_settings();
    let repo = repository_for(repo, &settings);
    let out = out.map_or_else(
        || repo.root().join("dist").join("manifest.json"),
        absolutise,
    );

    let manifest = manifest::build(&repo);
    let json = manifest.to_json();
    if let Some(parent) = out.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        eprintln!("{}: {error}", parent.display());
        return ExitCode::FAILURE;
    }
    if let Err(error) = std::fs::write(&out, json) {
        eprintln!("{}: {error}", out.display());
        return ExitCode::FAILURE;
    }

    println!(
        "wrote {}: {} procedure(s), {} checklist(s), {} run record(s), {} help page(s)",
        display_path(repo.root(), &out),
        manifest.procedures.len(),
        manifest.checklists.len(),
        manifest.runs.len(),
        manifest.help.len()
    );
    ExitCode::SUCCESS
}
