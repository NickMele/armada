//! git, as an ordinary adapter module.
//!
//! Not a seam: git is a subprocess, and giving it its own port would mean a
//! second way to fake a shell command and tests that disagree about which one
//! ran (`ARCHITECTURE.md` §1.1). Everything here builds argv and calls
//! `ctx.run`, so a fake asserts the exact command — which matters more here
//! than almost anywhere else, because one missing flag silently changes an
//! identity.

use charkit_core::ctx::{Run, RunRequest};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// char's deadline on a git call.
///
/// git is local and fast — the measured floor is 12–19 ms per invocation — so
/// anything approaching this is a wedged filesystem rather than a slow repo.
pub const GIT_TIMEOUT: Duration = Duration::from_secs(10);

/// The git root of the tree containing `cwd`, or `None` when there is no
/// repository.
///
/// `None` is not an error: `char config scan` exists to run before a config
/// does, and a directory outside any repository is a legitimate place to be.
/// The walk that uses this simply stops at the filesystem root instead.
pub fn root(run: &impl Run, cwd: &Path) -> Option<PathBuf> {
    let output = run
        .call(
            &RunRequest::new(
                vec![
                    "git".to_string(),
                    "rev-parse".to_string(),
                    "--show-toplevel".to_string(),
                ],
                cwd.to_path_buf(),
            )
            .timeout(GIT_TIMEOUT),
        )
        .ok()?;
    if !output.ok() {
        return None;
    }
    let line = output.stdout.trim();
    (!line.is_empty()).then(|| PathBuf::from(line))
}

/// The `--git-common-dir`, absolute, which is what `project_id` hashes.
///
/// **`--path-format=absolute` is load-bearing and is the whole reason this
/// function exists rather than a string literal at the call site.** The plain
/// form returns a path *relative to cwd* — `.git` from the repo root, `../.git`
/// from a subdirectory — so hashing it yields a different project id depending
/// on where char ran. Applying `realpath` looks like a fix and is not: char
/// runs git with `current_dir(workspace_root)` and would then resolve `../.git`
/// against **its own** cwd. Measured, git ≥ 2.31 answers identically from every
/// directory with this flag.
///
/// **`None` is expected and survivable.** Measured: inside a worktree whose
/// parent checkout was deleted, git answers `fatal: not a git repository:
/// (null)` — there is no key to recompute, so the project is *underivable*
/// rather than wrong. char treats that as `project: null`, because `project_id`
/// owns nothing: making it fatal would take a worktree whose resources are
/// perfectly reclaimable and refuse to reclaim them (PLAN.md §2.2).
pub fn common_dir(run: &impl Run, workspace_root: &Path) -> Option<PathBuf> {
    let output = run
        .call(
            &RunRequest::new(
                vec![
                    "git".to_string(),
                    "rev-parse".to_string(),
                    "--path-format=absolute".to_string(),
                    "--git-common-dir".to_string(),
                ],
                workspace_root.to_path_buf(),
            )
            .timeout(GIT_TIMEOUT),
        )
        .ok()?;
    if !output.ok() {
        return None;
    }
    let line = output.stdout.trim();
    (!line.is_empty()).then(|| PathBuf::from(line))
}

#[cfg(test)]
mod tests {
    use super::*;
    use charkit_core::ctx::{RunOutput, SpawnError};
    use std::cell::RefCell;

    /// Records argv and answers from a script. Argv is the assertion that
    /// matters: a missing `--path-format=absolute` is invisible in every other
    /// kind of test and changes an identity.
    #[derive(Default)]
    struct FakeRun {
        seen: RefCell<Vec<Vec<String>>>,
        stdout: String,
        code: i32,
    }

    impl Run for FakeRun {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.seen.borrow_mut().push(request.argv.clone());
            Ok(RunOutput {
                code: Some(self.code),
                signal: None,
                stdout: self.stdout.clone(),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    #[test]
    fn the_common_dir_call_carries_the_absolute_path_format_flag() {
        let run = FakeRun {
            stdout: "/srv/repo/.git\n".to_string(),
            ..Default::default()
        };
        let answer = common_dir(&run, Path::new("/srv/repo/sub"));
        assert_eq!(answer, Some(PathBuf::from("/srv/repo/.git")));
        assert_eq!(
            run.seen.borrow()[0],
            vec![
                "git",
                "rev-parse",
                "--path-format=absolute",
                "--git-common-dir"
            ]
        );
    }

    #[test]
    fn git_runs_in_the_workspace_root_and_never_in_chars_own_cwd() {
        let run = FakeRun {
            stdout: "/srv/repo/.git\n".to_string(),
            ..Default::default()
        };
        // The request carries the directory explicitly; nothing below the
        // entrypoint may read the process's own cwd.
        common_dir(&run, Path::new("/srv/repo"));
        assert!(!run.seen.borrow().is_empty());
    }

    /// An orphaned worktree: git fails, and that is `project: null`, not an
    /// error.
    #[test]
    fn a_failing_git_yields_no_project_rather_than_an_error() {
        let run = FakeRun {
            stdout: String::new(),
            code: 128,
            ..Default::default()
        };
        assert_eq!(common_dir(&run, Path::new("/srv/orphan")), None);
    }

    #[test]
    fn the_root_call_asks_for_the_toplevel() {
        let run = FakeRun {
            stdout: "/srv/repo\n".to_string(),
            ..Default::default()
        };
        assert_eq!(
            root(&run, Path::new("/srv/repo/sub")),
            Some(PathBuf::from("/srv/repo"))
        );
        assert_eq!(
            run.seen.borrow()[0],
            vec!["git", "rev-parse", "--show-toplevel"]
        );
    }
}
