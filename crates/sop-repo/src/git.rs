//! Optional local git metadata for the legacy CLI and record provenance.
//!
//! There is no repository synchronization, staging, committing, pulling, or pushing.
//! These queries do not contact a remote. The legacy CLI can still edit a local remote
//! URL, but the desktop notebook does not need or expose git configuration.
//!
//! A repository that is not a git repository at all is a normal state - a working copy
//! copied to a laptop as a plain directory is still usable - so nothing here fails hard
//! on the first missing answer.

use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// How long a local `git` query may take before it is killed.
const LOCAL_TIMEOUT: Duration = Duration::from_secs(15);

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
    /// The command was still running at the deadline and was killed.
    TimedOut {
        command: String,
        seconds: u64,
    },
    CommandFailed {
        command: String,
        message: String,
    },
}

impl std::fmt::Display for GitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::NotAvailable(message) => write!(formatter, "git is not available: {message}"),
            GitError::TimedOut { command, seconds } => {
                write!(formatter, "`{command}` did not finish within {seconds}s")
            }
            GitError::CommandFailed { command, message } => {
                write!(formatter, "`{command}` failed: {}", message.trim())
            }
        }
    }
}

impl std::error::Error for GitError {}

/// Run the command, reading its output on helper threads and killing it at `limit`.
///
/// `Command::output()` waits forever and cannot be interrupted; a `git pull` at a dead
/// remote would pin the calling thread. The output is drained concurrently so a chatty
/// command cannot fill its pipe and block before the deadline.
fn output_with_timeout(
    command: &mut Command,
    limit: Duration,
    label: &str,
) -> Result<(ExitStatus, Vec<u8>, Vec<u8>), GitError> {
    let mut child = child_of(command)?;
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });

    let deadline = Instant::now() + limit;
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => return Err(GitError::NotAvailable(error.to_string())),
        }
    }
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    if timed_out {
        return Err(GitError::TimedOut {
            command: label.to_owned(),
            seconds: limit.as_secs(),
        });
    }
    let status = child
        .wait()
        .map_err(|error| GitError::NotAvailable(error.to_string()))?;
    Ok((status, stdout, stderr))
}

/// Spawn the child with its streams piped and stdin closed, so git cannot wait for input.
fn child_of(command: &mut Command) -> Result<Child, GitError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| GitError::NotAvailable(error.to_string()))
}

fn label(root: &Path, args: &[&str]) -> String {
    format!("git -C {} {}", root.display(), args.join(" "))
}

fn run(root: &Path, args: &[&str]) -> Result<Option<String>, GitError> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args);
    let (status, stdout, _) = output_with_timeout(&mut command, LOCAL_TIMEOUT, &label(root, args))?;

    if status.success() {
        let text = String::from_utf8_lossy(&stdout).trim().to_owned();
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

/// The short commit at `HEAD`, when the working copy is a git repository.
pub fn current_commit(root: &Path) -> Option<String> {
    let commit = run(root, &["rev-parse", "--short", "HEAD"])
        .ok()
        .flatten()?;
    let commit = commit.trim();
    if commit.is_empty() {
        None
    } else {
        Some(commit.to_owned())
    }
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
    let mut child = Command::new("git");
    child.arg("-C").arg(root).args(&args);
    let (status, _, stderr) = output_with_timeout(&mut child, LOCAL_TIMEOUT, &label(root, &args))?;
    if status.success() {
        Ok(())
    } else {
        Err(GitError::CommandFailed {
            command: label(root, &args),
            message: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quick_command_returns_its_output() {
        let mut command = Command::new("git");
        command.args(["--version"]);
        let (status, stdout, _) =
            output_with_timeout(&mut command, LOCAL_TIMEOUT, "git --version").unwrap();
        assert!(status.success());
        assert!(String::from_utf8_lossy(&stdout).contains("git version"));
    }

    #[test]
    fn a_command_that_overruns_is_killed() {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("powershell");
            command.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"]);
            command
        } else {
            let mut command = Command::new("sleep");
            command.arg("30");
            command
        };
        let started = Instant::now();
        let error =
            output_with_timeout(&mut command, Duration::from_millis(150), "sleep 30").unwrap_err();
        assert!(matches!(error, GitError::TimedOut { .. }), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the helper returns at the deadline, not when the child would have finished"
        );
    }

    #[test]
    fn output_larger_than_a_pipe_buffer_does_not_deadlock() {
        // 300 KB is several times the usual 64 KB pipe buffer; the reader threads have to
        // drain it while the process is still running.
        let mut command = if cfg!(windows) {
            let mut command = Command::new("powershell");
            command.args([
                "-NoProfile",
                "-Command",
                "[Console]::OpenStandardOutput().Write([byte[]]::new(300000), 0, 300000)",
            ]);
            command
        } else {
            let mut command = Command::new("head");
            command.args(["-c", "300000", "/dev/zero"]);
            command
        };
        let (status, stdout, _) = output_with_timeout(&mut command, LOCAL_TIMEOUT, "head").unwrap();
        assert!(status.success());
        assert_eq!(stdout.len(), 300000);
    }
}
