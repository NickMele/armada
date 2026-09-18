//! Shelling to `git` and turning a non-zero exit into a typed fault —
//! shared by every module under `land/` that drives a worktree directly,
//! the way `crates/adapters` keeps its own private `git`/`run_in` helpers
//! for the same granularity of call.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Run `git -C cwd <args>`, without asking whether it succeeded.
pub fn best_effort(cwd: &Path, args: &[&str]) -> io::Result<Output> {
    Command::new("git").arg("-C").arg(cwd).args(args).output()
}

/// The same, but a non-zero exit is a fault the caller cannot ignore.
pub fn checked(cwd: &Path, args: &[&str]) -> Result<Output, GitFailed> {
    let output = best_effort(cwd, args).map_err(|cause| GitFailed {
        cwd: cwd.to_path_buf(),
        argv: owned(args),
        why: GitFailure::Unavailable(cause),
    })?;
    if !output.status.success() {
        return Err(GitFailed {
            cwd: cwd.to_path_buf(),
            argv: owned(args),
            why: GitFailure::NonZero {
                why: complaint(&output),
            },
        });
    }
    Ok(output)
}

fn owned(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

/// Prefers stderr, where git writes its complaints, falling back to stdout.
fn complaint(output: &Output) -> String {
    let text = if !output.stderr.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    String::from_utf8_lossy(text).trim().to_string()
}

/// One `git` invocation that could not be run at all, or ran and refused.
#[derive(Debug)]
pub struct GitFailed {
    pub cwd: PathBuf,
    pub argv: Vec<String>,
    pub why: GitFailure,
}

#[derive(Debug)]
pub enum GitFailure {
    Unavailable(io::Error),
    NonZero { why: String },
}

impl fmt::Display for GitFailed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "`git {}` in {}: {}",
            self.argv.join(" "),
            self.cwd.display(),
            self.why
        )
    }
}

impl std::error::Error for GitFailed {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.why {
            GitFailure::Unavailable(cause) => Some(cause),
            GitFailure::NonZero { .. } => None,
        }
    }
}

impl fmt::Display for GitFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitFailure::Unavailable(cause) => write!(out, "could not be run: {cause}"),
            GitFailure::NonZero { why } => out.write_str(why),
        }
    }
}
