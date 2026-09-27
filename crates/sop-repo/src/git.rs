//! Asking `git` about the working copy, and asking it to do one thing.
//!
//! The app does not implement git. It reads state that `git` already knows, and when it
//! changes the remote or publishes the working copy it asks `git` to do it, so the URL
//! stays in the repository's own configuration and the commit stays in the operator's
//! identity. Publishing is the only operation here that reaches the network, and even
//! that is `git push` doing it: the app holds no credentials and, because it runs with
//! `GIT_TERMINAL_PROMPT=0`, cannot be made to ask for any.
//!
//! A repository that is not a git repository at all is a normal state - a working copy
//! copied to a laptop as a plain directory is still usable - so nothing here fails hard
//! on the first missing answer.

use std::path::Path;
use std::process::Command;

/// What the app knows about the working copy's git state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitState {
    /// False when the directory is not a git working copy at all.
    pub is_repository: bool,
    pub branch: Option<String>,
    /// The remote name that was looked up, which is an application setting.
    pub remote: String,
    /// The URL that remote points at, which belongs to the repository.
    pub url: Option<String>,
    /// True when the URL embeds a username or a token.
    pub url_has_credentials: bool,
    pub dirty: bool,
    pub ahead: Option<u64>,
    pub behind: Option<u64>,
}

#[derive(Debug)]
pub enum GitError {
    NotAvailable(String),
    NotARepository(String),
    CommandFailed { command: String, message: String },
}

impl std::fmt::Display for GitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::NotAvailable(message) => write!(formatter, "git is not available: {message}"),
            GitError::NotARepository(root) => write!(
                formatter,
                "{root} is not a git working copy; run `git init` there first"
            ),
            GitError::CommandFailed { command, message } => {
                write!(formatter, "`{command}` failed: {}", message.trim())
            }
        }
    }
}

impl std::error::Error for GitError {}

fn run(root: &Path, args: &[&str]) -> Result<Option<String>, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| GitError::NotAvailable(error.to_string()))?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        return Ok(Some(text));
    }
    Ok(None)
}

/// Read the working copy's git state. Never fails because of a missing remote, a missing
/// upstream, or a directory that is not a repository.
pub fn state(root: &Path, remote: &str) -> GitState {
    let mut state = GitState {
        remote: remote.to_owned(),
        ..GitState::default()
    };

    let inside = run(root, &["rev-parse", "--is-inside-work-tree"]);
    state.is_repository = matches!(inside, Ok(Some(ref text)) if text == "true");
    if !state.is_repository {
        return state;
    }

    state.branch = run(root, &["symbolic-ref", "--short", "HEAD"])
        .ok()
        .flatten();
    state.url = run(root, &["remote", "get-url", remote]).ok().flatten();
    state.url_has_credentials = state.url.as_deref().is_some_and(has_credentials);
    state.dirty = run(root, &["status", "--porcelain"])
        .ok()
        .flatten()
        .is_some_and(|text| !text.is_empty());

    if let Ok(Some(counts)) = run(
        root,
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
    ) && let Some((behind, ahead)) = counts.split_once('\t')
    {
        state.behind = behind.trim().parse().ok();
        state.ahead = ahead.trim().parse().ok();
    }

    state
}

/// What one publish did, in the order it happened, for the app to show the operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushReport {
    pub branch: Option<String>,
    pub remote: String,
    /// How many paths `git status` reported, and so how many went into the commit.
    pub changed: usize,
    /// The short commit this publish made, when there was something to commit.
    pub commit: Option<String>,
    /// True when `git push` ran and said there was nothing new on the remote.
    pub up_to_date: bool,
    /// One line per thing that happened, in order. This is what the window renders.
    pub log: Vec<String>,
}

/// Stage everything, commit it when there is anything to commit, then push.
///
/// Staging is `git add -A` from the repository root, so a publish puts the working copy
/// in the state `git status` showed and nothing else. The push names `HEAD` rather than a
/// configured upstream, so a repository that has never been pushed still works.
///
/// The commit is the operator's: their name and email, their message. Credentials are
/// the operator's too, and `GIT_TERMINAL_PROMPT=0` makes sure a missing one is an error
/// the window can show rather than a prompt nobody can answer.
pub fn commit_and_push(
    root: &Path,
    remote: &str,
    message: &str,
) -> Result<PushReport, GitError> {
    if !state(root, remote).is_repository {
        return Err(GitError::NotARepository(root.display().to_string()));
    }

    let mut report = PushReport {
        branch: run(root, &["symbolic-ref", "--short", "HEAD"]).ok().flatten(),
        remote: remote.to_owned(),
        changed: 0,
        commit: None,
        up_to_date: false,
        log: Vec::new(),
    };

    let status = run_capture(root, &["status", "--porcelain"])?;
    let changed: Vec<&str> = status.lines().filter(|line| !line.trim().is_empty()).collect();
    report.changed = changed.len();

    if changed.is_empty() {
        report.log.push("nothing to commit".to_owned());
    } else {
        run_capture(root, &["add", "-A"])?;
        run_capture(root, &["commit", "-m", message])?;
        report.commit = run(root, &["rev-parse", "--short", "HEAD"]).ok().flatten();
        let on = report.branch.as_deref().unwrap_or("HEAD");
        report.log.push(format!(
            "committed {} path(s) on {on}{}",
            changed.len(),
            report
                .commit
                .as_deref()
                .map(|commit| format!(" as {commit}"))
                .unwrap_or_default()
        ));
    }

    let answer = run_capture(root, &["push", remote, "HEAD"])?;
    report.up_to_date = answer.contains("Everything up-to-date");
    report.log.push(if report.up_to_date {
        format!("{remote} is up to date")
    } else if answer.is_empty() {
        format!("pushed to {remote}")
    } else {
        format!("pushed to {remote}: {}", answer.replace('\n', "; "))
    });

    Ok(report)
}

/// Pull the current branch from a remote, fast-forward only.
///
/// Fast-forward keeps history linear; a divergent branch is reported as an error so the
/// operator can decide how to reconcile it rather than the app silently merging.
pub fn pull(root: &Path, remote: &str) -> Result<Vec<String>, GitError> {
    if !state(root, remote).is_repository {
        return Err(GitError::NotARepository(root.display().to_string()));
    }
    let branch = run(root, &["symbolic-ref", "--short", "HEAD"])
        .ok()
        .flatten()
        .unwrap_or_else(|| "HEAD".to_owned());
    let answer = run_capture(root, &["pull", "--ff-only", remote, branch.as_str()])?;
    let log: Vec<String> = if answer.is_empty() {
        vec!["already up to date".to_owned()]
    } else {
        answer.split('\n').map(str::to_owned).collect()
    };
    Ok(log)
}

/// Run `git` and hand back what it printed, or the command and its complaint.
///
/// Unlike [`run`], a failure is an error rather than a `None`: a publish that fails has
/// to say why, and the reason is usually a sentence from `git` on standard error.
fn run_capture(root: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| GitError::NotAvailable(error.to_string()))?;

    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        return Ok(text.trim().to_owned());
    }
    Err(GitError::CommandFailed {
        command: format!("git -C {} {}", root.display(), args.join(" ")),
        message: text,
    })
}

/// The short commit at `HEAD`, when the working copy is a git repository.
pub fn current_commit(root: &Path) -> Option<String> {
    let commit = run(root, &["rev-parse", "--short", "HEAD"]).ok().flatten()?;
    let commit = commit.trim();
    if commit.is_empty() { None } else { Some(commit.to_owned()) }
}

/// True when the URL embeds a username or a token.
///
/// The same `@` means different things under different schemes, so the rule differs too.
/// On http and https the userinfo is a secret even when there is no password, because a
/// token is usually pasted in as the username:
///
/// ```text
/// https://ghp_example@github.com/org/repo.git
/// ```
///
/// On ssh the login name is part of how the remote is spelled and is nobody's secret, so
/// `git@github.com:org/repo.git` and `ssh://git@host/repo.git` are not flagged. Only a
/// password after the colon is.
///
/// The check reads the authority only, so a credential hidden in a query string or a
/// path is not detected. It exists to catch the common mistake, and it is a warning
/// rather than a guarantee.
pub fn has_credentials(url: &str) -> bool {
    let Some((scheme, rest)) = url.split_once("://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or(rest);
    let Some((userinfo, _host)) = authority.rsplit_once('@') else {
        return false;
    };
    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        return true;
    }
    userinfo.contains(':')
}

/// Point a remote at a URL, in the repository's own configuration.
pub fn set_remote_url(root: &Path, remote: &str, url: &str) -> Result<(), GitError> {
    let exists = run(root, &["remote", "get-url", remote]);
    let args: Vec<&str> = if matches!(exists, Ok(Some(_))) {
        vec!["remote", "set-url", remote, url]
    } else {
        vec!["remote", "add", remote, url]
    };
    let command = format!("git -C {} {}", root.display(), args.join(" "));
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(&args)
        .output()
        .map_err(|error| GitError::NotAvailable(error.to_string()))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(GitError::CommandFailed {
            command,
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
