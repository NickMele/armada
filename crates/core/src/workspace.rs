//! Workspace resolution (PLAN.md §2.1) — the pure half.
//!
//! **A workspace is one directory tree containing a `armada.yml`, which gets its
//! own runtime state.** In practice: a checkout. The walk itself is I/O and
//! lives in `adapters`; what it *means* — one, none, or several — is a decision
//! and lives here.
//!
//! The answer must be identical from anywhere inside the tree, because
//! `workspace_id` is a hash of it. That is why the walk collects **every**
//! `armada.yml` up to the git root rather than stopping at the nearest one: an
//! accidental nested config then fails loudly instead of silently creating a
//! second owner for the same source.

use crate::error::{CharError, ConfigWhere, ErrClass};
use crate::id::{ProjectId, WorkspaceId};
use std::path::{Path, PathBuf};

/// A resolved workspace: the root, its two identities, and the config file to
/// cite in errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    /// The realpath of the directory holding this workspace's `armada.yml`.
    pub root: PathBuf,
    /// `sha1(realpath(root))[..8]`. Owns everything char creates here.
    pub id: WorkspaceId,
    /// `None` when the project is **underivable** — measured, an orphaned
    /// worktree whose parent checkout was deleted answers `fatal: not a git
    /// repository: (null)`, so there is no key to recompute (PLAN.md §2.2).
    /// char treats that as `project: null` rather than as an error, because
    /// `project_id` owns nothing: `--project` scoping stops working for that
    /// worktree while `--all`, the workspace id, and every reclaim keep working.
    pub project: Option<ProjectId>,
    /// The path to cite in a `bad_config` — `armada.yml`, or
    /// `<declared path>/armada.yml` for a nested workspace, so the message names
    /// a file the caller can find from the repo root.
    pub config_label: String,
}

impl Workspace {
    /// Build a workspace from an already-resolved root.
    pub fn new(root: PathBuf, project: Option<ProjectId>, config_label: String) -> Self {
        Workspace {
            id: WorkspaceId::derive(&root),
            root,
            project,
            config_label,
        }
    }

    /// This workspace's `armada.yml`.
    pub fn config_path(&self) -> PathBuf {
        self.root.join("armada.yml")
    }

    /// This workspace's `.char/` (PLAN.md §4.2).
    pub fn char_dir(&self) -> PathBuf {
        self.root.join(".char")
    }
}

/// One directory holding a `armada.yml`, with the nested workspaces it declares.
///
/// The declarations travel with the candidate because the two-or-more rule
/// needs them, and reading them is the shell's job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The directory holding the `armada.yml`.
    pub dir: PathBuf,
    /// This config's `workspaces:` list, verbatim (PLAN.md §4.6).
    pub declares: Vec<String>,
}

/// Which of the collected `armada.yml` files is this workspace's, if any.
///
/// `candidates` is ordered **innermost first** — the order the walk from the
/// caller's cwd up to the git root produces. `searched` is every directory the
/// walk visited, so the zero case can name them rather than saying only that
/// nothing was found.
pub fn choose_root(candidates: &[Candidate], searched: &[PathBuf]) -> Result<PathBuf, CharError> {
    match candidates {
        [] => Err(no_config(searched)),
        [only] => Ok(only.dir.clone()),
        [innermost, ..] => {
            // Two or more is an error *unless* each outer config declares the
            // one directly inside it. Checking every adjacent pair rather than
            // only the outermost is what keeps a three-deep stack honest: a
            // root that declares its grandchild but not its child still has
            // two owners for the child's subtree.
            for pair in candidates.windows(2) {
                let (inner, outer) = (&pair[0], &pair[1]);
                if !declares(outer, &inner.dir) {
                    return Err(undeclared_nesting(&inner.dir, &outer.dir));
                }
            }
            Ok(innermost.dir.clone())
        }
    }
}

/// Whether `outer` declares `inner` in its `workspaces:` list.
fn declares(outer: &Candidate, inner: &Path) -> bool {
    let Ok(relative) = inner.strip_prefix(&outer.dir) else {
        return false;
    };
    outer
        .declares
        .iter()
        .any(|declared| Path::new(declared.trim_end_matches('/')) == relative)
}

fn no_config(searched: &[PathBuf]) -> CharError {
    let list = searched
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    CharError {
        class: ErrClass::BadConfig,
        r#where: "armada.yml".to_string(),
        message: format!("no armada.yml in any directory up to the git root: {list}"),
        next_action: Some(
            "run `char config scan` to gather evidence, then author a armada.yml at the repo root"
                .to_string(),
        ),
    }
}

fn undeclared_nesting(inner: &Path, outer: &Path) -> CharError {
    CharError::bad_config(
        ConfigWhere::Path {
            file: format!("{}/armada.yml", outer.display()),
            path: "workspaces".to_string(),
        },
        format!(
            "two armada.yml files own this tree: {} and {}",
            inner.display(),
            outer.display()
        ),
        format!(
            "declare it — `workspaces: [{}]` in the outer armada.yml — or delete the inner one",
            inner.strip_prefix(outer).unwrap_or(inner).display()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(dir: &str, declares: &[&str]) -> Candidate {
        Candidate {
            dir: PathBuf::from(dir),
            declares: declares.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn exactly_one_char_yml_is_the_workspace_root() {
        let found = [candidate("/srv/repo", &[])];
        assert_eq!(
            choose_root(&found, &[]).unwrap(),
            PathBuf::from("/srv/repo")
        );
    }

    #[test]
    fn zero_is_bad_config_and_names_the_directories_searched() {
        let searched = [
            PathBuf::from("/srv/repo/apps/site"),
            PathBuf::from("/srv/repo"),
        ];
        let err = choose_root(&[], &searched).unwrap_err();
        assert_eq!(err.class, ErrClass::BadConfig);
        assert!(err.message.contains("/srv/repo/apps/site"));
        assert!(err.message.contains("/srv/repo"));
        assert!(err.next_action.is_some(), "bad_config requires next_action");
    }

    #[test]
    fn two_undeclared_is_bad_config() {
        let found = [
            candidate("/srv/repo/apps/site", &[]),
            candidate("/srv/repo", &[]),
        ];
        let err = choose_root(&found, &[]).unwrap_err();
        assert_eq!(err.class, ErrClass::BadConfig);
        assert!(err.message.contains("two armada.yml files"));
    }

    #[test]
    fn the_innermost_wins_when_the_outer_declares_it() {
        let found = [
            candidate("/srv/repo/apps/site", &[]),
            candidate("/srv/repo", &["apps/site"]),
        ];
        assert_eq!(
            choose_root(&found, &[]).unwrap(),
            PathBuf::from("/srv/repo/apps/site")
        );
    }

    #[test]
    fn a_trailing_slash_in_the_declaration_still_matches() {
        let found = [
            candidate("/srv/repo/apps/site", &[]),
            candidate("/srv/repo", &["apps/site/"]),
        ];
        assert!(choose_root(&found, &[]).is_ok());
    }

    /// A root that declares its grandchild but not its child leaves the child's
    /// subtree with two owners — which is the state the two-or-more rule
    /// exists to reject, so checking only the outermost pair is not enough.
    #[test]
    fn every_adjacent_pair_must_be_declared_not_just_the_outermost() {
        let found = [
            candidate("/srv/repo/a/b", &[]),
            candidate("/srv/repo/a", &[]),
            candidate("/srv/repo", &["a/b"]),
        ];
        let err = choose_root(&found, &[]).unwrap_err();
        assert!(err.message.contains("/srv/repo/a"));
    }

    #[test]
    fn a_workspace_derives_its_id_from_its_root() {
        let ws = Workspace::new(PathBuf::from("/srv/repo"), None, "armada.yml".into());
        assert_eq!(ws.id, WorkspaceId::derive(Path::new("/srv/repo")));
        assert_eq!(ws.char_dir(), PathBuf::from("/srv/repo/.char"));
        assert_eq!(ws.project, None);
    }
}
