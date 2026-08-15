//! `git worktree`, as an ordinary adapter module.
//!
//! **Fleet invents no worktree concept.** `git worktree` is the only primitive;
//! Fleet adds policy only — where it lives, what it is named, that it gets a
//! port block from Manifest, and that it is recorded (PLAN.md §14.1).
//!
//! Every function here builds argv and calls `ctx.run`, so a test asserts the
//! exact command. That matters more here than almost anywhere else: `git
//! worktree add` without `-b` checks out an existing branch, which silently puts
//! two Jobs on one branch and makes their commits interleave.

use armada_core::ctx::{Run, RunRequest};
use armada_core::error::{ArmadaError, ErrClass};
use armada_manifest::git::GIT_TIMEOUT;
use std::path::Path;

/// The branch a Job's worktree is created on.
///
/// **Namespaced, so a repository's own branches are never touched.** `armada
/// fleet kill` deletes this branch by name, and a Job that had been given a
/// bare `rate-limit` could delete a branch a person was working on.
pub fn branch_for(name: &str) -> String {
    format!("armada/{name}")
}

/// Create the worktree for a Job.
///
/// **`-b` is not optional.** Without it `git worktree add` checks out an
/// existing branch when the name happens to match, which puts two Jobs on one
/// branch — and the first symptom is a merge nobody made.
pub fn add(run: &impl Run, repo_root: &Path, path: &Path, branch: &str) -> Result<(), ArmadaError> {
    // The parent directory is Fleet's policy rather than git's: `git worktree
    // add` creates the leaf and not `~/.armada/workspaces/<repo>/`.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ArmadaError {
            class: ErrClass::Environment,
            r#where: parent.display().to_string(),
            message: format!("could not make room for the worktree: {e}"),
            next_action: Some("check ~/.armada/workspaces/ is writable".to_string()),
        })?;
    }

    let argv = vec![
        "git".to_string(),
        "worktree".to_string(),
        "add".to_string(),
        "-b".to_string(),
        branch.to_string(),
        path.display().to_string(),
    ];
    call(run, repo_root, argv, "could not create the worktree")
}

/// Remove a Job's worktree.
///
/// **`--force`, because a Drone leaves a dirty tree behind.** That is the
/// ordinary case rather than the exception: a Job that was killed mid-turn has
/// uncommitted work by definition, and refusing to remove it would leave the
/// port block claimed and the directory stranded — the state `kill` exists to
/// prevent. The work is not lost silently: `--keep-branch` keeps the commits and
/// `--keep-worktree` keeps the directory, and a caller who wants either says so.
pub fn remove(run: &impl Run, repo_root: &Path, path: &Path) -> Result<(), ArmadaError> {
    let argv = vec![
        "git".to_string(),
        "worktree".to_string(),
        "remove".to_string(),
        "--force".to_string(),
        path.display().to_string(),
    ];
    call(run, repo_root, argv, "could not remove the worktree")
}

/// Delete a Job's branch.
///
/// **`-D`, and only ever on the `armada/` namespace** ([`branch_for`]). A Job's
/// branch has not been merged anywhere by definition, so `-d` would refuse every
/// time and the flag would mean nothing.
pub fn delete_branch(run: &impl Run, repo_root: &Path, branch: &str) -> Result<(), ArmadaError> {
    let argv = vec![
        "git".to_string(),
        "branch".to_string(),
        "-D".to_string(),
        branch.to_string(),
    ];
    call(run, repo_root, argv, "could not delete the branch")
}

/// Whether `path` is still a worktree git knows about.
///
/// Read off `git worktree list --porcelain`, which is the only thing that knows:
/// a directory that exists is not necessarily registered, and one that is
/// registered may have been deleted underneath git.
pub fn is_registered(run: &impl Run, repo_root: &Path, path: &Path) -> bool {
    let argv = vec![
        "git".to_string(),
        "worktree".to_string(),
        "list".to_string(),
        "--porcelain".to_string(),
    ];
    let Ok(output) = run.call(&RunRequest::new(argv, repo_root.to_path_buf()).timeout(GIT_TIMEOUT))
    else {
        return false;
    };
    let wanted = path.display().to_string();
    output
        .stdout
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .any(|line| line.trim() == wanted)
}

fn call(
    run: &impl Run,
    repo_root: &Path,
    argv: Vec<String>,
    what: &str,
) -> Result<(), ArmadaError> {
    let spelled = argv.join(" ");
    let output = run
        .call(&RunRequest::new(argv, repo_root.to_path_buf()).timeout(GIT_TIMEOUT))
        .map_err(|e| ArmadaError {
            // git missing from `PATH` is the machine being incomplete, not the
            // repository being wrong.
            class: ErrClass::Environment,
            r#where: "git".to_string(),
            message: format!("{what}: {}", e.message),
            next_action: Some("install git, then retry unchanged".to_string()),
        })?;
    if output.ok() {
        return Ok(());
    }
    Err(ArmadaError {
        // git ran and refused. That is a real result rather than Armada's fault,
        // and git's own message is more useful than any paraphrase of it.
        class: ErrClass::ToolFailed,
        r#where: spelled,
        message: format!("{what}: {}", first_line(&output.stderr)),
        next_action: None,
    })
}

fn first_line(stderr: &str) -> String {
    stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("git said nothing")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, SpawnError};
    use std::cell::RefCell;
    use std::path::PathBuf;

    struct FakeRun {
        seen: RefCell<Vec<Vec<String>>>,
        code: i32,
        stdout: String,
        stderr: String,
    }

    impl FakeRun {
        fn ok() -> FakeRun {
            FakeRun {
                seen: RefCell::new(Vec::new()),
                code: 0,
                stdout: String::new(),
                stderr: String::new(),
            }
        }

        fn refusing(stderr: &str) -> FakeRun {
            FakeRun {
                stderr: stderr.to_string(),
                code: 128,
                ..FakeRun::ok()
            }
        }

        fn argv(&self) -> Vec<String> {
            self.seen.borrow()[0].clone()
        }
    }

    impl Run for FakeRun {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.seen.borrow_mut().push(request.argv.clone());
            Ok(RunOutput {
                code: Some(self.code),
                signal: None,
                stdout: self.stdout.clone(),
                stderr: self.stderr.clone(),
                timed_out: false,
            })
        }
    }

    /// **`-b` is not optional**, and this is the assertion that keeps it there:
    /// without it, two Jobs whose names collide would land on one branch and
    /// their commits would interleave.
    #[test]
    fn creating_a_worktree_always_creates_its_branch() {
        let run = FakeRun::ok();
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("workspaces/api/rate-limit");
        add(&run, Path::new("/code/api"), &path, "armada/rate-limit").unwrap();
        assert_eq!(
            run.argv(),
            [
                "git",
                "worktree",
                "add",
                "-b",
                "armada/rate-limit",
                &path.display().to_string(),
            ]
        );
    }

    /// The call is made **from the repository**, not from the worktree that does
    /// not exist yet.
    #[test]
    fn the_worktree_is_created_from_inside_the_repository() {
        struct Cwd(RefCell<Option<PathBuf>>);
        impl Run for Cwd {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                *self.0.borrow_mut() = Some(request.cwd.clone());
                Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    timed_out: false,
                })
            }
        }
        let run = Cwd(RefCell::new(None));
        let home = tempfile::tempdir().unwrap();
        add(&run, Path::new("/code/api"), &home.path().join("w"), "b").unwrap();
        assert_eq!(run.0.borrow().clone().unwrap(), PathBuf::from("/code/api"));
    }

    /// **A killed Job has a dirty tree by definition**, so removal forces. The
    /// alternative leaves the port block claimed and the directory stranded,
    /// which is the state `kill` exists to prevent.
    #[test]
    fn removing_a_worktree_forces_because_a_drone_leaves_a_dirty_tree() {
        let run = FakeRun::ok();
        remove(&run, Path::new("/code/api"), Path::new("/w/rate-limit")).unwrap();
        assert_eq!(
            run.argv(),
            ["git", "worktree", "remove", "--force", "/w/rate-limit"]
        );
    }

    /// A Job's branch is never merged anywhere, so `-d` would refuse every time
    /// and the flag would mean nothing.
    #[test]
    fn deleting_a_branch_uses_the_form_that_works_on_an_unmerged_one() {
        let run = FakeRun::ok();
        delete_branch(&run, Path::new("/code/api"), "armada/rate-limit").unwrap();
        assert_eq!(run.argv(), ["git", "branch", "-D", "armada/rate-limit"]);
    }

    /// **The namespace is what makes deleting safe.** A Job given a bare
    /// `rate-limit` could delete a branch a person was working on.
    #[test]
    fn a_jobs_branch_is_namespaced_away_from_the_repositorys_own() {
        assert_eq!(branch_for("rate-limit"), "armada/rate-limit");
        assert!(branch_for("main").starts_with("armada/"));
    }

    /// git ran and refused: that is a real result, so the class is the one whose
    /// documented response is *report it*, and git's own words are carried.
    #[test]
    fn a_refusal_from_git_is_a_tool_failure_carrying_gits_own_message() {
        let run = FakeRun::refusing("fatal: 'armada/x' is already checked out\n");
        let error = remove(&run, Path::new("/code/api"), Path::new("/w/x")).unwrap_err();
        assert_eq!(error.class, ErrClass::ToolFailed);
        assert_eq!(error.class.exit_code(), 1);
        assert!(error.message.contains("already checked out"), "{error}");
    }

    /// git missing is the machine being incomplete rather than the repository
    /// being wrong, so the retry is the identical command after a person fixes
    /// something Armada cannot.
    #[test]
    fn git_missing_from_path_is_an_environment_failure() {
        struct Missing;
        impl Run for Missing {
            fn call(&self, _: &RunRequest) -> Result<RunOutput, SpawnError> {
                Err(SpawnError {
                    program: "git".to_string(),
                    kind: armada_core::ctx::SpawnErrorKind::NotFound,
                    message: "No such file or directory".to_string(),
                })
            }
        }
        let error = delete_branch(&Missing, Path::new("/code/api"), "armada/x").unwrap_err();
        assert_eq!(error.class, ErrClass::Environment);
        assert_eq!(error.class.exit_code(), 6);
    }

    #[test]
    fn a_registered_worktree_is_the_one_git_lists() {
        let run = FakeRun {
            stdout: "worktree /code/api\nHEAD abc\n\nworktree /w/rate-limit\nHEAD def\n"
                .to_string(),
            ..FakeRun::ok()
        };
        assert!(is_registered(
            &run,
            Path::new("/code/api"),
            Path::new("/w/rate-limit")
        ));
        assert!(!is_registered(
            &run,
            Path::new("/code/api"),
            Path::new("/w/gone")
        ));
    }
}
