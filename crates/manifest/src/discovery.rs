//! Finding the workspace: the walk, which is I/O.
//!
//! What the walk *means* — one, none, or several — is a decision and lives in
//! [`armada_core::workspace`]. This module does the two things the core may
//! not: it reads directories, and it asks git where the repository stops.
//!
//! > Walk up from the caller's cwd to the git root, collecting **every**
//! > `armada.yml` found.
//!
//! **Collecting all of them rather than taking the nearest** is what makes an
//! accidental nested `armada.yml` fail loudly instead of silently creating a
//! second owner for the same source. **Stopping at the git root** keeps a stray
//! `armada.yml` in a parent directory from capturing an unrelated repo — and
//! means a git submodule, which has its own git root, is correctly its own
//! workspace for free.

use armada_core::config::declared_workspaces;
use armada_core::ctx::Run;
use armada_core::error::CharError;
use armada_core::id::ProjectId;
use armada_core::workspace::{choose_root, Candidate, Workspace};
use std::path::{Path, PathBuf};

use crate::{fs, git};

/// Resolve the workspace containing `cwd`.
///
/// `cwd` is passed in rather than read, because nothing below the entrypoint
/// may call `std::env::current_dir` (`ARCHITECTURE.md` §1.4). That is not
/// ceremony: `--project` and `--all` operate on workspaces that are *not* the
/// current directory, so a function that can re-derive "the workspace" from cwd
/// makes scoping to a different one require lying about cwd.
pub fn resolve(run: &impl Run, cwd: &Path) -> Result<Workspace, CharError> {
    let cwd = fs::canonical(cwd)?;
    let stop_at = git::root(run, &cwd).and_then(|root| fs::canonical(&root).ok());

    let (candidates, searched) = walk(&cwd, stop_at.as_deref());
    let root = choose_root(&candidates, &searched)?;

    let project = git::common_dir(run, &root)
        .and_then(|dir| fs::canonical(&dir).ok().or(Some(dir)))
        .map(|dir| ProjectId::derive(&dir));

    // Cited relative to the git root when there is one, so a nested workspace's
    // error names a file the caller can find from where they are standing.
    let label = stop_at
        .as_deref()
        .and_then(|git_root| root.strip_prefix(git_root).ok())
        .map(|relative| {
            if relative.as_os_str().is_empty() {
                "armada.yml".to_string()
            } else {
                format!("{}/armada.yml", relative.display())
            }
        })
        .unwrap_or_else(|| "armada.yml".to_string());

    Ok(Workspace::new(root, project, label))
}

/// Every directory from `from` up to and including `stop_at`, and the ones
/// among them that hold a `armada.yml`.
///
/// With no git root the walk stops at the filesystem root — which is the
/// correct behaviour outside a repository, where `char config scan` is the only
/// verb that runs anyway.
fn walk(from: &Path, stop_at: Option<&Path>) -> (Vec<Candidate>, Vec<PathBuf>) {
    let mut candidates = Vec::new();
    let mut searched = Vec::new();

    let mut current = Some(from);
    while let Some(dir) = current {
        searched.push(dir.to_path_buf());
        let config = dir.join("armada.yml");
        if config.is_file() {
            let declares = std::fs::read_to_string(&config)
                .map(|text| declared_workspaces(&text))
                .unwrap_or_default();
            candidates.push(Candidate {
                dir: dir.to_path_buf(),
                declares,
            });
        }
        if Some(dir) == stop_at {
            break;
        }
        current = dir.parent();
    }

    (candidates, searched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, RunRequest, SpawnError};
    use armada_core::error::ErrClass;

    /// Answers git the way a real repository would, without needing one.
    struct FakeGit {
        toplevel: PathBuf,
        common_dir: Option<PathBuf>,
    }

    impl Run for FakeGit {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            let asks_common_dir = request.argv.iter().any(|a| a == "--git-common-dir");
            let (code, stdout) = match (asks_common_dir, &self.common_dir) {
                (true, Some(dir)) => (0, format!("{}\n", dir.display())),
                (true, None) => (128, String::new()),
                (false, _) => (0, format!("{}\n", self.toplevel.display())),
            };
            Ok(RunOutput {
                code: Some(code),
                signal: None,
                stdout,
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// The property the whole design rests on: `workspace_id` is a hash of the
    /// root, so the root must be the same answer from anywhere inside the tree.
    #[test]
    fn the_answer_is_identical_from_any_depth() {
        let scratch = scratch();
        let root = std::fs::canonicalize(scratch.path()).unwrap();
        write(&root.join("armada.yml"), "manifest:\n  version: 1\n");
        std::fs::create_dir_all(root.join("a/b/c")).unwrap();

        let git = FakeGit {
            toplevel: root.clone(),
            common_dir: Some(root.join(".git")),
        };

        let from_root = resolve(&git, &root).unwrap();
        let from_deep = resolve(&git, &root.join("a/b/c")).unwrap();
        assert_eq!(from_root.id, from_deep.id);
        assert_eq!(from_root.root, from_deep.root);
        assert_eq!(from_root.project, from_deep.project);
    }

    #[test]
    fn a_declared_nested_workspace_wins_and_is_cited_by_its_path() {
        let scratch = scratch();
        let root = std::fs::canonicalize(scratch.path()).unwrap();
        write(
            &root.join("armada.yml"),
            "manifest:\n  version: 1\n  workspaces: [apps/site]\n",
        );
        write(
            &root.join("apps/site/armada.yml"),
            "manifest:\n  version: 1\n",
        );

        let git = FakeGit {
            toplevel: root.clone(),
            common_dir: Some(root.join(".git")),
        };
        let workspace = resolve(&git, &root.join("apps/site")).unwrap();
        assert_eq!(workspace.root, root.join("apps/site"));
        assert_eq!(workspace.config_label, "apps/site/armada.yml");
    }

    #[test]
    fn an_undeclared_nested_config_is_bad_config() {
        let scratch = scratch();
        let root = std::fs::canonicalize(scratch.path()).unwrap();
        write(&root.join("armada.yml"), "manifest:\n  version: 1\n");
        write(
            &root.join("apps/site/armada.yml"),
            "manifest:\n  version: 1\n",
        );

        let git = FakeGit {
            toplevel: root.clone(),
            common_dir: Some(root.join(".git")),
        };
        let err = resolve(&git, &root.join("apps/site")).unwrap_err();
        assert_eq!(err.class, ErrClass::BadConfig);
    }

    #[test]
    fn no_config_anywhere_is_bad_config_naming_where_it_looked() {
        let scratch = scratch();
        let root = std::fs::canonicalize(scratch.path()).unwrap();
        let git = FakeGit {
            toplevel: root.clone(),
            common_dir: Some(root.join(".git")),
        };
        let err = resolve(&git, &root).unwrap_err();
        assert_eq!(err.class, ErrClass::BadConfig);
        assert!(err.message.contains(&root.display().to_string()));
    }

    /// The walk stops at the git root, so a stray `armada.yml` in a parent
    /// directory cannot capture an unrelated repository.
    #[test]
    fn a_config_above_the_git_root_is_never_collected() {
        let scratch = scratch();
        let outer = std::fs::canonicalize(scratch.path()).unwrap();
        write(&outer.join("armada.yml"), "manifest:\n  version: 1\n");
        let repo = outer.join("repo");
        std::fs::create_dir_all(&repo).unwrap();

        let git = FakeGit {
            toplevel: repo.clone(),
            common_dir: Some(repo.join(".git")),
        };
        let err = resolve(&git, &repo).unwrap_err();
        assert_eq!(
            err.class,
            ErrClass::BadConfig,
            "the outer config is not this repo's"
        );
    }

    /// An orphaned worktree: git cannot answer, and that is `project: null`
    /// rather than a failure. Every resource stays perfectly reclaimable.
    #[test]
    fn an_underivable_project_is_null_and_not_an_error() {
        let scratch = scratch();
        let root = std::fs::canonicalize(scratch.path()).unwrap();
        write(&root.join("armada.yml"), "manifest:\n  version: 1\n");

        let git = FakeGit {
            toplevel: root.clone(),
            common_dir: None,
        };
        let workspace = resolve(&git, &root).unwrap();
        assert_eq!(workspace.project, None);
        assert!(!workspace.id.as_str().is_empty());
    }

    /// A config char cannot parse still answers the nesting question — as
    /// "declares nothing" — so the failure reported is the nesting rather than
    /// a parse the caller did not ask for.
    #[test]
    fn an_unparseable_outer_config_does_not_mask_the_nesting_error() {
        let scratch = scratch();
        let root = std::fs::canonicalize(scratch.path()).unwrap();
        write(&root.join("armada.yml"), "this: [is: not: yaml\n");
        write(
            &root.join("apps/site/armada.yml"),
            "manifest:\n  version: 1\n",
        );

        let git = FakeGit {
            toplevel: root.clone(),
            common_dir: Some(root.join(".git")),
        };
        let err = resolve(&git, &root.join("apps/site")).unwrap_err();
        assert!(err.message.contains("two armada.yml files"), "{err:?}");
    }
}
