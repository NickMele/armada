//! The merge line's two worktrees: `candidate` and `base`, fixed names under
//! `.armada/land/` inside the repository, kept and reused rather than cut
//! fresh every turn.
//!
//! **Reused, not cut per turn.** A cut-per-turn worktree leaves a checkout
//! and a cold build behind on every run; [`TurnLock`](super::lock::TurnLock)
//! guarantees one turn at a time, which is what makes resetting a worktree
//! in place safe rather than merely convenient.
//!
//! **Inside the repository, at `<main_tree>/.armada/land/`.** A checkout
//! outside it fails this project's own Checks for reasons unrelated to the
//! branch being gated. `.armada/land/` sits beside `.armada/worktrees/` and
//! `.armada/bases/` but `armada clean` (`src/clean.rs`) never reaches it.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::git::{best_effort, checked, GitFailed};

/// What survives `git clean -xdff` between turns. `scripts/land`'s own
/// `KEEP` is environment-overridable and reads `armada.yml`; this stage
/// fixes the same two entries as a plain constant instead.
pub const KEEP: &[&str] = &["target", "node_modules"];

/// One of the merge line's two fixed worktrees.
///
/// An enum instead of a bare name: there are only ever two of these, by
/// design, and this makes a third one unspellable rather than merely
/// undocumented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandWorktree {
    /// The branch being landed.
    Candidate,
    /// The base it is being gated against.
    Base,
}

impl LandWorktree {
    fn directory_name(self) -> &'static str {
        match self {
            LandWorktree::Candidate => "candidate",
            LandWorktree::Base => "base",
        }
    }
}

/// The checkout every worktree of `repo`'s clone was cut from — the first
/// line of `git worktree list --porcelain`. `repo` may itself be any
/// worktree of the clone; the main tree is named first regardless.
pub fn main_tree(repo: &Path) -> Result<PathBuf, MainTreeError> {
    let output = checked(repo, &["worktree", "list", "--porcelain"]).map_err(MainTreeError::Git)?;
    let text = String::from_utf8_lossy(&output.stdout);
    let first_line = text.lines().next();
    let path = first_line.and_then(|line| line.strip_prefix("worktree "));
    match path {
        Some(path) => Ok(PathBuf::from(path)),
        None => Err(MainTreeError::NoWorktreeListed {
            repo: repo.to_path_buf(),
        }),
    }
}

/// `<main_tree>/.armada/land`, created if missing.
pub fn land_root(repo: &Path) -> Result<PathBuf, LandRootError> {
    let tree = main_tree(repo).map_err(LandRootError::MainTree)?;
    let at = tree.join(".armada").join("land");
    std::fs::create_dir_all(&at).map_err(|cause| LandRootError::Unwritable {
        path: at.clone(),
        cause,
    })?;
    Ok(at)
}

/// One of this line's two worktrees, at `commit`, holding nothing of the
/// last turn but its build directories.
///
/// **Remade, not repaired,** where it is missing or no longer a real
/// worktree — a killed runner can leave a half-merged tree that
/// `reset --hard` alone would not clear. Otherwise: abort any in-progress
/// merge, check out and hard-reset to `commit`, then `git clean -xdff`
/// keeping [`KEEP`].
///
/// The clean itself is best effort, matching `scripts/land`'s own
/// `check=False` there: a failure is logged to `<logs>/setup.log` rather
/// than failing the turn, since the worktree is at the right commit either
/// way.
pub fn reused(
    repo: &Path,
    which: LandWorktree,
    commit: &str,
    logs: &Path,
) -> Result<PathBuf, ReuseError> {
    let root = land_root(repo).map_err(ReuseError::LandRoot)?;
    let at = root.join(which.directory_name());

    let already_a_worktree = best_effort(&at, &["rev-parse", "--git-dir"])
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !already_a_worktree {
        drop_worktree(repo, &at);
        checked(
            repo,
            &[
                "worktree",
                "add",
                "--quiet",
                "--detach",
                &at.to_string_lossy(),
                commit,
            ],
        )
        .map_err(ReuseError::Git)?;
    }

    // Best effort: no merge in progress is the ordinary case, not a fault.
    let _ = best_effort(&at, &["merge", "--abort"]);

    checked(&at, &["checkout", "--detach", "--force", commit]).map_err(ReuseError::Git)?;
    checked(&at, &["reset", "--hard", "--quiet", commit]).map_err(ReuseError::Git)?;

    let mut clean_args: Vec<&str> = vec!["clean", "-xdff", "--quiet"];
    for path in KEEP {
        clean_args.push("-e");
        clean_args.push(path);
    }
    if let Ok(output) = best_effort(&at, &clean_args) {
        append_setup_log(logs, &clean_args, &output).map_err(ReuseError::Log)?;
    }

    Ok(at)
}

/// Remove one of this line's own worktrees, under `.armada/land/`.
///
/// Best effort throughout — matching `scripts/land`'s own `drop_worktree`,
/// which never raises. `remove_dir_all` clears the directory whether or not
/// `git worktree remove` accepted it, and a path that is already absent is
/// a no-op rather than a failure: absence is this function's own goal.
pub fn drop_worktree(repo: &Path, at: &Path) {
    if at.exists() {
        let _ = best_effort(
            repo,
            &["worktree", "remove", "--force", &at.to_string_lossy()],
        );
        let _ = std::fs::remove_dir_all(at);
    }
    let _ = best_effort(repo, &["worktree", "prune"]);
}

/// Append one command's output to `<logs>/setup.log`, matching
/// `scripts/land`'s own `run(..., log=...)` shape.
fn append_setup_log(logs: &Path, args: &[&str], output: &Output) -> Result<(), LogError> {
    use std::io::Write;
    let path = logs.join("setup.log");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|cause| LogError {
            path: path.clone(),
            cause,
        })?;
    let mut write_all = || -> io::Result<()> {
        writeln!(file, "$ git {}", args.join(" "))?;
        file.write_all(&output.stdout)?;
        file.write_all(&output.stderr)?;
        writeln!(file, "[exit {}]", output.status.code().unwrap_or(-1))?;
        Ok(())
    };
    write_all().map_err(|cause| LogError { path, cause })
}

/// `<logs>/setup.log` could not be written.
#[derive(Debug)]
pub struct LogError {
    pub path: PathBuf,
    pub cause: io::Error,
}

impl fmt::Display for LogError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "{} could not be written", self.path.display())
    }
}

impl std::error::Error for LogError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

/// Why the main tree could not be found.
#[derive(Debug)]
pub enum MainTreeError {
    Git(GitFailed),
    /// Not reachable against a real git — named rather than unwrapped so a
    /// panic is never how this surfaces.
    NoWorktreeListed {
        repo: PathBuf,
    },
}

impl fmt::Display for MainTreeError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MainTreeError::Git(why) => write!(out, "{why}"),
            MainTreeError::NoWorktreeListed { repo } => {
                write!(out, "{} did not list a worktree", repo.display())
            }
        }
    }
}

impl std::error::Error for MainTreeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MainTreeError::Git(why) => Some(why),
            MainTreeError::NoWorktreeListed { .. } => None,
        }
    }
}

/// Why the land root could not be resolved.
#[derive(Debug)]
pub enum LandRootError {
    MainTree(MainTreeError),
    Unwritable { path: PathBuf, cause: io::Error },
}

impl fmt::Display for LandRootError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LandRootError::MainTree(why) => write!(out, "{why}"),
            LandRootError::Unwritable { path, .. } => {
                write!(out, "{} could not be created", path.display())
            }
        }
    }
}

impl std::error::Error for LandRootError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LandRootError::MainTree(why) => Some(why),
            LandRootError::Unwritable { cause, .. } => Some(cause),
        }
    }
}

/// Why a worktree could not be reused.
#[derive(Debug)]
pub enum ReuseError {
    LandRoot(LandRootError),
    Git(GitFailed),
    Log(LogError),
}

impl fmt::Display for ReuseError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReuseError::LandRoot(why) => write!(out, "{why}"),
            ReuseError::Git(why) => write!(out, "{why}"),
            ReuseError::Log(why) => write!(out, "{why}"),
        }
    }
}

impl std::error::Error for ReuseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReuseError::LandRoot(why) => Some(why),
            ReuseError::Git(why) => Some(why),
            ReuseError::Log(why) => Some(why),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use crate::tests::TempDir;

    use super::{drop_worktree, land_root, main_tree, reused, LandWorktree};

    fn git(repo: &std::path::Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .status()
            .expect("git on PATH — a test nothing can run is a test that does not exist");
        assert!(status.success(), "git {args:?} failed");
    }

    fn commit_sha(repo: &std::path::Path) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("git rev-parse");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// A real repository with two commits on `main`.
    fn a_repo_with_two_commits() -> (TempDir, String, String) {
        let dir = TempDir::new();
        git(
            dir.path(),
            &["-c", "init.defaultBranch=main", "init", "--quiet"],
        );
        git(dir.path(), &["config", "user.email", "test@example.com"]);
        git(dir.path(), &["config", "user.name", "test"]);

        dir.write("a.txt", "first\n");
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "--quiet", "-m", "first"]);
        let first = commit_sha(dir.path());

        dir.write("a.txt", "second\n");
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "--quiet", "-m", "second"]);
        let second = commit_sha(dir.path());

        (dir, first, second)
    }

    #[test]
    fn reused_creates_the_worktree_fresh_at_the_right_commit() {
        let (repo, first, _second) = a_repo_with_two_commits();
        let logs = TempDir::new();

        let at = reused(repo.path(), LandWorktree::Candidate, &first, logs.path())
            .expect("a fresh worktree");

        assert!(at.ends_with("candidate"));
        assert_eq!(commit_sha(&at), first);
        assert_eq!(
            std::fs::read_to_string(at.join("a.txt"))
                .expect("a.txt")
                .trim(),
            "first"
        );
    }

    #[test]
    fn reusing_it_again_resets_and_cleans_but_keeps_target() {
        let (repo, first, second) = a_repo_with_two_commits();
        let logs = TempDir::new();

        let at = reused(repo.path(), LandWorktree::Candidate, &first, logs.path())
            .expect("the first turn's worktree");

        std::fs::create_dir_all(at.join("target")).expect("a build directory");
        std::fs::write(at.join("target").join("built.txt"), "built").expect("a built artifact");
        std::fs::write(at.join("stray.txt"), "left behind by a killed turn").expect("a stray file");

        let at_again = reused(repo.path(), LandWorktree::Candidate, &second, logs.path())
            .expect("the second turn's worktree");

        assert_eq!(at, at_again, "the same directory is reused, not replaced");
        assert_eq!(
            commit_sha(&at_again),
            second,
            "reset to the new turn's commit"
        );
        assert!(
            at_again.join("target").join("built.txt").exists(),
            "the build directory survives reuse — the entire point of keeping it"
        );
        assert!(
            !at_again.join("stray.txt").exists(),
            "an untracked file left by the last turn is cleaned away"
        );
    }

    #[test]
    fn main_tree_is_found_from_inside_a_worktree_of_it() {
        let (repo, first, _second) = a_repo_with_two_commits();
        let logs = TempDir::new();

        let at = reused(repo.path(), LandWorktree::Base, &first, logs.path())
            .expect("a worktree to look from");

        let found = main_tree(&at).expect("main_tree from inside a worktree");
        assert_eq!(found, repo.path().to_path_buf());
    }

    #[test]
    fn land_root_is_under_the_main_trees_own_armada_directory() {
        let (repo, _first, _second) = a_repo_with_two_commits();
        let root = land_root(repo.path()).expect("land_root");
        assert_eq!(root, repo.path().join(".armada").join("land"));
        assert!(root.is_dir());
    }

    #[test]
    fn drop_worktree_removes_it_and_is_a_noop_when_already_absent() {
        let (repo, first, _second) = a_repo_with_two_commits();
        let logs = TempDir::new();
        let at = reused(repo.path(), LandWorktree::Candidate, &first, logs.path())
            .expect("a worktree to drop");
        assert!(at.exists());

        drop_worktree(repo.path(), &at);
        assert!(!at.exists());

        let listed = Command::new("git")
            .arg("-C")
            .arg(repo.path())
            .args(["worktree", "list", "--porcelain"])
            .output()
            .expect("git worktree list");
        let text = String::from_utf8_lossy(&listed.stdout);
        assert!(
            !text.contains("candidate"),
            "git itself no longer lists the dropped worktree: {text}"
        );

        // No-op, not an error — there is no `Result` here to be one.
        drop_worktree(repo.path(), &at);
    }
}
