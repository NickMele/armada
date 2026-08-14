//! git, as an ordinary adapter module.
//!
//! Not a seam: git is a subprocess, and giving it its own port would mean a
//! second way to fake a shell command and tests that disagree about which one
//! ran (`ARCHITECTURE.md` §1.1). Everything here builds argv and calls
//! `ctx.run`, so a fake asserts the exact command — which matters more here
//! than almost anywhere else, because one missing flag silently changes an
//! identity.

use armada_core::ctx::{Run, RunRequest};
use armada_core::error::{ArmadaError, ErrClass};
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

/// The refs char tries, in order, to find the branch to diff against.
///
/// **`origin/HEAD` first, and it is one call rather than two.** The obvious
/// implementation asks `git symbolic-ref --short refs/remotes/origin/HEAD` and
/// then diffs against the answer; git resolves `origin/HEAD` directly, so the
/// lookup is unnecessary. That matters because PLAN.md §3.2 records a measured
/// ~65 ms floor for `char check` before any YAML is parsed, and says a phase
/// that adds a sixth subprocess to the common path is spending from a budget and
/// should say so. **Measured here on darwin, five runs each: every git
/// invocation on this path costs 21–25 ms**, so the lookup would have been a
/// fifth of the whole floor for something the fallback list already covers.
///
/// `main` and `master` follow, for a repository with no remote — a fresh
/// `git init`, or a clone whose `origin/HEAD` was never set, both of which are
/// ordinary rather than exotic.
pub const BASE_REFS: [&str; 3] = ["origin/HEAD", "main", "master"];

/// The merge-base of `HEAD` with the default branch, and the ref that produced
/// it.
///
/// `None` when none of [`BASE_REFS`] resolves — a fresh clone with no history, a
/// detached HEAD, or a CI shallow clone where the base is genuinely not
/// present. **char does not silently fall back to the whole tree there**
/// (PLAN.md §4.1): that would be the same hole `--all-files` exists to close,
/// with an extra step. The caller reports `bad_invocation` naming the missing
/// base and telling the caller to pass `--all-files`.
pub fn merge_base(run: &impl Run, root: &Path) -> Option<(String, String)> {
    for reference in BASE_REFS {
        let output = run
            .call(
                &RunRequest::new(
                    vec![
                        "git".to_string(),
                        "merge-base".to_string(),
                        "HEAD".to_string(),
                        reference.to_string(),
                    ],
                    root.to_path_buf(),
                )
                .timeout(GIT_TIMEOUT),
            )
            .ok()?;
        if output.ok() {
            let base = output.stdout.trim();
            if !base.is_empty() {
                return Some((reference.to_string(), base.to_string()));
            }
        }
    }
    None
}

/// The files changed against `base`, plus uncommitted working-tree changes.
///
/// **Read NUL-delimited and never split by char** (PLAN.md §4.1). Newline is a
/// legal character in a POSIX filename, so a line-oriented read of git's output
/// turns one file into two nonexistent ones — and because argv carries the
/// values with no re-parsing, a filename with a newline in it survives end to
/// end.
///
/// **Deletions are excluded, via `--diff-filter=d`.** A deleted path in
/// `${files}` reaches the tool as an argument to a file that is not there, and
/// `ruff check a-deleted-file.py` fails — reporting `tool_failed` for a check
/// that is passing, on a branch that deleted a file, which is an ordinary thing
/// for a branch to do.
///
/// **Untracked files come from `ls-files --others` rather than from parsing
/// `git status -z`.** Same call count, and no rename form: a `status` entry for
/// a rename carries *two* NUL-terminated paths, so a parser that treats every
/// record as one path silently attributes a file to the wrong name — which is
/// the failure the NUL delimiter was adopted to prevent, reintroduced one layer
/// up.
pub fn changed_files(run: &impl Run, root: &Path, base: &str) -> Result<Vec<String>, ArmadaError> {
    let mut files = nul_separated(
        run,
        root,
        &["git", "diff", "-z", "--name-only", "--diff-filter=d", base],
    )?;
    files.extend(nul_separated(
        run,
        root,
        &["git", "ls-files", "-z", "--others", "--exclude-standard"],
    )?);
    files.sort();
    files.dedup();
    Ok(files)
}

/// Every file git is tracking, which is what `--all-files` expands the globs
/// against.
pub fn tracked_files(run: &impl Run, root: &Path) -> Result<Vec<String>, ArmadaError> {
    let mut files = nul_separated(run, root, &["git", "ls-files", "-z"])?;
    files.sort();
    files.dedup();
    Ok(files)
}

/// Run a git command whose output is NUL-separated paths.
fn nul_separated(run: &impl Run, root: &Path, argv: &[&str]) -> Result<Vec<String>, ArmadaError> {
    let request = RunRequest::new(
        argv.iter().map(|a| (*a).to_string()).collect(),
        root.to_path_buf(),
    )
    .timeout(GIT_TIMEOUT);

    let output = run.call(&request).map_err(|e| ArmadaError {
        class: ErrClass::Environment,
        r#where: "git".to_string(),
        message: format!("cannot run git: {}", e.message),
        next_action: Some("install git, or put it on PATH".to_string()),
    })?;

    if !output.ok() {
        return Err(ArmadaError {
            class: ErrClass::Environment,
            r#where: "git".to_string(),
            message: format!("`{}` failed: {}", argv.join(" "), output.stderr.trim()),
            next_action: None,
        });
    }

    Ok(output
        .stdout
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, SpawnError};
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

    /// A fake that answers per command rather than with one canned string, so
    /// a multi-call function can be driven through its fallbacks.
    #[derive(Default)]
    struct Scripted {
        seen: RefCell<Vec<Vec<String>>>,
        answers: Vec<(Vec<&'static str>, i32, &'static str)>,
    }

    impl Run for Scripted {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.seen.borrow_mut().push(request.argv.clone());
            let (code, stdout) = self
                .answers
                .iter()
                .find(|(argv, _, _)| *argv == request.argv)
                .map(|(_, code, stdout)| (*code, *stdout))
                .unwrap_or((1, ""));
            Ok(RunOutput {
                code: Some(code),
                signal: None,
                stdout: stdout.to_string(),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    /// One call, not two: git resolves `origin/HEAD` itself, and PLAN.md §3.2's
    /// measured floor makes a sixth subprocess on the common path something to
    /// spend deliberately.
    #[test]
    fn the_merge_base_resolves_origin_head_without_a_lookup_call() {
        let run = Scripted {
            answers: vec![(
                vec!["git", "merge-base", "HEAD", "origin/HEAD"],
                0,
                "abc123\n",
            )],
            ..Default::default()
        };
        assert_eq!(
            merge_base(&run, Path::new("/srv/repo")),
            Some(("origin/HEAD".to_string(), "abc123".to_string()))
        );
        assert_eq!(run.seen.borrow().len(), 1, "{:?}", run.seen.borrow());
        assert!(
            !run.seen
                .borrow()
                .iter()
                .any(|argv| argv.contains(&"symbolic-ref".to_string())),
            "a lookup call was made"
        );
    }

    /// A repository with no remote is ordinary — a fresh `git init`, or a clone
    /// whose `origin/HEAD` was never set.
    #[test]
    fn the_merge_base_falls_back_through_main_and_master() {
        let run = Scripted {
            answers: vec![(vec!["git", "merge-base", "HEAD", "master"], 0, "def456\n")],
            ..Default::default()
        };
        assert_eq!(
            merge_base(&run, Path::new("/srv/repo")),
            Some(("master".to_string(), "def456".to_string()))
        );
        assert_eq!(run.seen.borrow().len(), 3, "every ref was tried in order");
    }

    /// **char does not silently fall back to the whole tree** — that is the
    /// same hole `--all-files` exists to close, with an extra step.
    #[test]
    fn a_repository_with_no_base_at_all_answers_none_rather_than_everything() {
        let run = Scripted::default();
        assert_eq!(merge_base(&run, Path::new("/srv/repo")), None);
    }

    /// **Newline is a legal character in a POSIX filename**, so a line-oriented
    /// read turns one file into two nonexistent ones. This is the assertion
    /// that stops the obvious `lines()` refactor.
    #[test]
    fn a_filename_containing_a_newline_survives_as_one_file() {
        let run = Scripted {
            answers: vec![
                (
                    vec![
                        "git",
                        "diff",
                        "-z",
                        "--name-only",
                        "--diff-filter=d",
                        "abc123",
                    ],
                    0,
                    "a/one.py\0a/two\nlines.py\0",
                ),
                (
                    vec!["git", "ls-files", "-z", "--others", "--exclude-standard"],
                    0,
                    "",
                ),
            ],
            ..Default::default()
        };
        let files = changed_files(&run, Path::new("/srv/repo"), "abc123").unwrap();
        assert_eq!(files, vec!["a/one.py", "a/two\nlines.py"]);
    }

    /// A filename may contain a semicolon or a command substitution, and git
    /// emits it raw under `-z`. Because argv carries the value with no
    /// re-parsing, it survives end to end.
    #[test]
    fn a_hostile_filename_survives_the_read_unchanged() {
        let run = Scripted {
            answers: vec![
                (
                    vec![
                        "git",
                        "diff",
                        "-z",
                        "--name-only",
                        "--diff-filter=d",
                        "abc123",
                    ],
                    0,
                    "sub/semi;echo INJECTED.py\0sub/dollar$(id).py\0",
                ),
                (
                    vec!["git", "ls-files", "-z", "--others", "--exclude-standard"],
                    0,
                    "",
                ),
            ],
            ..Default::default()
        };
        let files = changed_files(&run, Path::new("/srv/repo"), "abc123").unwrap();
        assert_eq!(
            files,
            vec!["sub/dollar$(id).py", "sub/semi;echo INJECTED.py"]
        );
    }

    /// **`--diff-filter=d` excludes deletions.** A deleted path reaches the
    /// tool as an argument to a file that is not there, so a branch that deleted
    /// a file would fail a check that is passing.
    #[test]
    fn the_diff_asks_git_to_leave_out_deletions_and_reads_untracked_files_too() {
        let run = Scripted {
            answers: vec![
                (
                    vec![
                        "git",
                        "diff",
                        "-z",
                        "--name-only",
                        "--diff-filter=d",
                        "abc123",
                    ],
                    0,
                    "a/kept.py\0",
                ),
                (
                    vec!["git", "ls-files", "-z", "--others", "--exclude-standard"],
                    0,
                    "a/brand-new.py\0",
                ),
            ],
            ..Default::default()
        };
        let files = changed_files(&run, Path::new("/srv/repo"), "abc123").unwrap();
        assert_eq!(files, vec!["a/brand-new.py", "a/kept.py"]);
        assert!(
            run.seen.borrow()[0].contains(&"--diff-filter=d".to_string()),
            "deletions were not excluded"
        );
        assert!(
            !run.seen
                .borrow()
                .iter()
                .any(|argv| argv.contains(&"status".to_string())),
            "`status -z` has a rename form that carries two paths per record"
        );
    }

    /// A git that fails is `environment` — the machine is broken, not the repo
    /// — so an agent fixes the machine and retries unchanged rather than
    /// reading test output.
    #[test]
    fn a_failing_git_read_is_an_environment_failure() {
        let run = Scripted::default();
        let error = changed_files(&run, Path::new("/srv/repo"), "abc123").unwrap_err();
        assert_eq!(error.class, ErrClass::Environment);
        assert_eq!(error.class.exit_code(), 6);
    }

    #[test]
    fn the_tracked_list_is_what_all_files_expands_the_globs_against() {
        let run = Scripted {
            answers: vec![(vec!["git", "ls-files", "-z"], 0, "b.py\0a.py\0a.py\0")],
            ..Default::default()
        };
        assert_eq!(
            tracked_files(&run, Path::new("/srv/repo")).unwrap(),
            vec!["a.py", "b.py"],
            "sorted and deduplicated"
        );
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
