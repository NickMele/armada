//! Small git queries `preflight` and a turn both need — the branch checked
//! out, what the remote holds, and whether one commit is an ancestor of
//! another. `super::git` is the shared shell-out; this is the vocabulary
//! `scripts/land`'s own `current_branch`, `remote_head` and the
//! `merge-base --is-ancestor` calls scattered through it name once.

use std::path::Path;

use super::git::{best_effort, checked, GitFailed};

/// The branch checked out in `cwd`, or `None` for a detached `HEAD`.
pub fn current_branch(cwd: &Path) -> Option<String> {
    let output = best_effort(cwd, &["symbolic-ref", "--quiet", "--short", "HEAD"]).ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!name.is_empty()).then_some(name)
}

/// What `remote` holds for `branch`, or `None` where the remote has no such
/// branch.
pub fn remote_head(cwd: &Path, remote: &str, branch: &str) -> Result<Option<String>, GitFailed> {
    let output = checked(cwd, &["ls-remote", remote, &format!("refs/heads/{branch}")])?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(text.split_whitespace().next().map(str::to_string))
}

/// Whether `ancestor` is reachable from `descendant` — `false` on any git
/// failure, matching `scripts/land`'s own `check=False` at every call site.
pub fn is_ancestor(cwd: &Path, ancestor: &str, descendant: &str) -> bool {
    best_effort(cwd, &["merge-base", "--is-ancestor", ancestor, descendant])
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn rev_parse(cwd: &Path, rev: &str) -> Result<String, GitFailed> {
    let output = checked(cwd, &["rev-parse", rev])?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The paths that changed between two commits — `git diff --name-only`.
pub fn changed_paths(cwd: &Path, from: &str, to: &str) -> Result<Vec<String>, GitFailed> {
    let output = checked(cwd, &["diff", "--name-only", from, to])?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

pub fn merge_base(cwd: &Path, one: &str, other: &str) -> Result<String, GitFailed> {
    let output = checked(cwd, &["merge-base", one, other])?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `git rev-parse --path-format=absolute --git-common-dir` — what a detached
/// runner is started with, so it acts on the repository rather than
/// whichever worktree happened to queue it.
pub fn common_git_dir(cwd: &Path) -> Result<std::path::PathBuf, GitFailed> {
    let output = checked(
        cwd,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    Ok(std::path::PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}
