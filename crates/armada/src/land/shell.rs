//! Running one of the swappable binaries — `$ARMADA_LAND_GH`,
//! `$ARMADA_LAND_ARMADA`, `$ARMADA_LAND_FOUNDATIONS` — and turning a spawn
//! failure into a [`Stopped`], the way `scripts/land`'s own `run()` does.
//! `gh`'s JSON answer is decoded through [`ipc::decode`], the same doorway
//! every state file under `land/` already goes through, rather than parsing
//! it directly here.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde::Deserialize;

use super::stop::Stopped;

/// One command's answer, kept whole rather than split into stdout/stderr at
/// the call site — most callers here read either half depending on what
/// went wrong, exactly as `scripts/land`'s own `run()` result does.
pub struct Ran {
    output: Output,
}

impl Ran {
    pub fn success(&self) -> bool {
        self.output.status.success()
    }

    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).to_string()
    }

    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).to_string()
    }

    /// stdout, falling back to stderr — `not_installed`'s own reading needs
    /// both halves of what a Check printed.
    pub fn combined(&self) -> String {
        format!("{}{}", self.stdout(), self.stderr())
    }

    /// The process's own exit code, or `-1` for one that ended by signal.
    pub fn status_code(&self) -> i32 {
        self.output.status.code().unwrap_or(-1)
    }
}

/// Run `argv[0] argv[1..]` in `cwd`, piping `stdin` in if given and
/// appending stdout and stderr to `log` if given. A spawn failure is a
/// [`Stopped`] naming the program and the directory — the way a relative
/// `ARMADA_LAND_ARMADA` is caught in practice.
pub fn run(
    argv: &[&str],
    cwd: &Path,
    stdin: Option<&str>,
    log: Option<&Path>,
) -> Result<Ran, Stopped> {
    let mut command = Command::new(argv[0]);
    command
        .args(&argv[1..])
        .current_dir(cwd)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let spawn_failed = |cause: std::io::Error| {
        Stopped::stopped(format!(
            "`{}` could not be run from {}: {cause}",
            argv[0],
            cwd.display()
        ))
    };
    let mut child = command.spawn().map_err(spawn_failed)?;
    if let Some(input) = stdin {
        if let Some(mut pipe) = child.stdin.take() {
            let _ = pipe.write_all(input.as_bytes());
        }
    }
    let output = child.wait_with_output().map_err(spawn_failed)?;
    let ran = Ran { output };
    if let Some(path) = log {
        append_log(path, argv, &ran);
    }
    Ok(ran)
}

fn append_log(path: &Path, argv: &[&str], ran: &Ran) {
    use std::fs::OpenOptions;
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "$ {}", argv.join(" "));
        let _ = file.write_all(&ran.output.stdout);
        let _ = file.write_all(&ran.output.stderr);
        let _ = writeln!(file, "[exit {}]", ran.status_code());
    }
}

/// What `gh pr view` may answer, across every field list a call site here
/// asks for. **One struct, every field optional**: `gh --json` prints only
/// the fields asked for, so a field this call did not request decodes as
/// `None` rather than failing the whole read.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct GhPullRequest {
    pub number: Option<u64>,
    pub state: Option<String>,
    #[serde(rename = "baseRefName")]
    pub base_ref_name: Option<String>,
    #[serde(rename = "headRefOid")]
    pub head_ref_oid: Option<String>,
    #[serde(rename = "mergeCommit")]
    pub merge_commit: Option<GhMergeCommit>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GhMergeCommit {
    pub oid: String,
}

/// `$ARMADA_LAND_GH pr view <pull_request> --json <fields>`, decoded through
/// [`ipc::decode`] — the one JSON boundary this module crosses, kept to the
/// doorway every state file under `land/` already uses, rather than reading
/// the bytes directly here.
pub fn gh_view(gh: &str, cwd: &Path, pull_request: &str, fields: &str) -> Option<GhPullRequest> {
    let ran = run(
        &[gh, "pr", "view", pull_request, "--json", fields],
        cwd,
        None,
        None,
    )
    .ok()?;
    if !ran.success() {
        return None;
    }
    ipc::decode("gh pr view", ran.stdout().as_bytes()).ok()
}
