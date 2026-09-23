//! Which checkout of a served repository a server is held on — `#1577`, and
//! `docs/concepts/fleet.md`, *Servers*, for why a repository has more than one
//! and each needs a span of its own.
//!
//! **The only way to a [`Checkout`] is [`Checkout::main`] or
//! [`Checkout::beside`]**, and the second one looks at the disk. There is no
//! constructor that takes a path and believes it, which is what stops a span
//! being claimed against a directory that is not a checkout of the repository
//! the span is sized from — `docs/practices/rust.md`, section 2.
//!
//! **git's own on-disk contract, read as files**: a linked worktree's `.git`
//! is a file naming `<repo>/.git/worktrees/<name>`, and that directory's
//! `HEAD` names the branch. Two reads and no process, which is
//! `crate::drifting`'s line — a git *operation* would be `adapters`'.

use std::path::{Path, PathBuf};

use adapter_traits::WorktreeSpec;

use crate::repositories::Served;

/// A checkout a server may be held on: this repository's main checkout, or a
/// worktree of it that is nobody's Job.
#[derive(Clone)]
pub(crate) struct Checkout {
    served: Served,
    path: String,
    branch: Option<String>,
}

/// Why a path is not a checkout a server may be held on.
#[derive(Debug)]
pub(crate) enum NotACheckout {
    /// Not a worktree of this repository — a directory that is not there, one
    /// that is not a checkout at all, or one belonging to another repository.
    Elsewhere { path: String, root: String },
    /// A Job's worktree. Reachable, but not this way: a Job's server is held
    /// for the Job and stopped when it ends, which a path cannot say.
    AJobs { path: String },
}

impl std::fmt::Display for NotACheckout {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotACheckout::Elsewhere { path, root } => write!(
                out,
                "`{path}` is not a checkout of {root} — name the repository's root, or a \
                 worktree of it that `git worktree list` shows"
            ),
            NotACheckout::AJobs { path } => write!(
                out,
                "`{path}` is a Job's worktree, so name the Job instead — its server is held \
                 for the Job and stopped when the Job ends"
            ),
        }
    }
}

impl Checkout {
    /// This repository's main checkout.
    ///
    /// **Its root verbatim, never canonicalised.** The root is the key a
    /// claim, a `Holder` and a records directory were already written under,
    /// and a second spelling of one directory is a second span.
    pub(crate) fn main(served: Served) -> Checkout {
        let path = served.root().to_string();
        Checkout {
            served,
            path,
            branch: None,
        }
    }

    /// A worktree of this repository at `path`, **confirmed on disk**. The
    /// root itself answers as [`Checkout::main`] rather than being refused.
    pub(crate) fn beside(served: Served, path: &str) -> Result<Checkout, NotACheckout> {
        let elsewhere = || NotACheckout::Elsewhere {
            path: path.to_string(),
            root: served.root().to_string(),
        };
        let here = std::fs::canonicalize(path).map_err(|_| elsewhere())?;
        let root = PathBuf::from(served.root());
        let canonical_root = std::fs::canonicalize(&root).unwrap_or(root);
        if here == canonical_root {
            return Ok(Checkout::main(served));
        }
        if jobs_worktrees(served.root()).is_some_and(|parent| here.starts_with(parent)) {
            return Err(NotACheckout::AJobs {
                path: path.to_string(),
            });
        }
        let administration = administration_of(&here).ok_or_else(elsewhere)?;
        if !administration.starts_with(canonical_root.join(".git").join("worktrees")) {
            return Err(elsewhere());
        }
        let branch = branch_at(&administration);
        Ok(Checkout {
            served,
            path: here.to_string_lossy().into_owned(),
            branch,
        })
    }

    pub(crate) fn served(&self) -> &Served {
        &self.served
    }

    /// The directory the server runs in, and the key its span is claimed
    /// under.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// The branch checked out there. **Absent for the main checkout**, which a
    /// person moves between branches under a running server — its path and the
    /// commit it came up on are what say which build is answering.
    pub(crate) fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    /// Whether this is the repository's own root.
    fn is_the_root(&self) -> bool {
        self.path == self.served.root()
    }

    /// Where this checkout's server records go, under `.armada/servers/`.
    /// `main` for the root, so nothing a Fleet already has on disk moves.
    pub(crate) fn under(&self) -> String {
        match self.is_the_root() {
            true => String::from("main"),
            false => format!("checkouts/{}", named(&self.path, self.served.root())),
        }
    }

    /// How a record a crashed Fleet left behind says whose server this was.
    pub(crate) fn whose(&self) -> String {
        match self.is_the_root() {
            true => String::from("main-checkout"),
            false => format!("checkout {}", self.path),
        }
    }
}

/// `<root>/.armada/worktrees`, canonicalised — where every Job's checkout is.
///
/// **Derived rather than spelled**, so this cannot come to disagree with where
/// `crate::dispatch` cuts one. The id is a placeholder: the parent is the same
/// whichever Job it is.
fn jobs_worktrees(root: &str) -> Option<PathBuf> {
    let parent = WorktreeSpec::for_job(root, "any").ok()?.worktree_parent();
    Some(std::fs::canonicalize(&parent).unwrap_or_else(|_| PathBuf::from(parent)))
}

/// The directory git keeps a linked worktree's administrative files in, named
/// by the `.git` *file* at its root. `None` where `.git` is a directory, which
/// is a repository's own root rather than a linked worktree.
fn administration_of(checkout: &Path) -> Option<PathBuf> {
    let said = std::fs::read_to_string(checkout.join(".git")).ok()?;
    let named = said.lines().find_map(|line| line.strip_prefix("gitdir:"))?;
    let named = PathBuf::from(named.trim());
    Some(std::fs::canonicalize(&named).unwrap_or(named))
}

/// The branch checked out, from the administrative directory's own `HEAD`.
/// `None` on a detached HEAD, which is a fact and not a failure.
fn branch_at(administration: &Path) -> Option<String> {
    let head = std::fs::read_to_string(administration.join("HEAD")).ok()?;
    let named = head.trim().strip_prefix("ref: refs/heads/")?;
    match named.is_empty() {
        true => None,
        false => Some(named.to_string()),
    }
}

/// A directory name for this checkout: the path below the repository root
/// where it is inside it, else the whole path, with every run of anything that
/// is not a letter or a digit turned into one `-`.
///
/// **The whole path and not its last component**, because two worktrees named
/// `main` under two parents are two checkouts, and one directory of records
/// would interleave them.
fn named(path: &str, root: &str) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let mut out = String::new();
    for ch in relative.chars() {
        match ch.is_ascii_alphanumeric() {
            true => out.push(ch.to_ascii_lowercase()),
            false => {
                if !out.ends_with('-') {
                    out.push('-');
                }
            }
        }
    }
    let trimmed = out.trim_matches('-');
    match trimmed.is_empty() {
        true => String::from("checkout"),
        false => trimmed.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::named;

    #[test]
    fn a_records_name_is_the_whole_path_below_the_root() {
        assert_eq!(
            named("/repos/armada/.claude/worktrees/1577-span", "/repos/armada"),
            "claude-worktrees-1577-span"
        );
    }

    #[test]
    fn two_worktrees_of_the_same_name_under_two_parents_are_two_names() {
        assert_eq!(named("/a/main", "/repos/armada"), "a-main");
        assert_eq!(named("/b/main", "/repos/armada"), "b-main");
    }
}
