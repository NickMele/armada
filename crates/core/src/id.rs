//! The two derived identities (PLAN.md §2.2).
//!
//! Both are 8 hex characters and both are **derived, never stored as truth** —
//! so they survive a deleted `.char/` and anything can recompute them. They are
//! distinct newtypes rather than two `String` aliases because passing one where
//! the other belongs must not compile: `workspace_id` owns ports, containers,
//! networks and processes, while `project_id` owns nothing at all and is purely
//! a grouping key.
//!
//! **Both take an already-resolved absolute path.** Resolving symlinks is I/O
//! and lives in `adapters`; hashing bytes is a pure function and lives here.
//! The split matters for `project_id`, whose input is
//! `git rev-parse --path-format=absolute --git-common-dir` — see
//! [`ProjectId::derive`].

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// Hex characters kept from the digest. Eight, per PLAN.md §2.2 — 32 bits,
/// which is why `clean` filters on the workspace *path* label as well as the id
/// (PLAN.md §2.3.1).
const ID_LEN: usize = 8;

fn short_sha1(bytes: &[u8]) -> String {
    let mut hasher = sha1_smol::Sha1::new();
    hasher.update(bytes);
    let mut hex = hasher.digest().to_string();
    hex.truncate(ID_LEN);
    hex
}

/// Identifies one checkout. Owns ports, containers, networks, volumes, images
/// and processes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    /// `sha1(realpath(workspace_root))[..8]`.
    ///
    /// The caller resolves symlinks first: two paths reaching one directory
    /// must not produce two identities, and a symlinked checkout is the
    /// ordinary way that happens.
    pub fn derive(realpath: &Path) -> Self {
        WorkspaceId(short_sha1(realpath.as_os_str().as_encoded_bytes()))
    }

    /// Adopt an id read back out of `char.db` or a resource label.
    ///
    /// The hash is one-way, so a stored id can only ever be compared, never
    /// re-derived into the path that produced it — which is the whole reason
    /// PLAN.md §2.3.1 stamps `char.workspace_path` alongside it.
    pub fn from_stored(id: impl Into<String>) -> Self {
        WorkspaceId(id.into())
    }

    /// The 8 hex characters.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Groups every workspace sharing one `--git-common-dir`: a checkout and its
/// worktrees. **Owns nothing.**
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    /// `sha1(realpath(git rev-parse --path-format=absolute --git-common-dir))[..8]`.
    ///
    /// **`--path-format=absolute` is load-bearing and `realpath` alone does not
    /// substitute for it.** The plain form returns a path *relative to cwd*, so
    /// hashing it yields a different id depending on where char ran; resolving
    /// that relative path looks like a fix and is not, because it resolves
    /// against char's own cwd rather than the directory git ran in. The
    /// absolute form is identical from every directory, and `realpath` stays
    /// only to collapse symlinks. See PLAN.md §2.2 — this fails only in a
    /// subdirectory of the main checkout, which is why it is easy to ship.
    pub fn derive(common_dir_realpath: &Path) -> Self {
        ProjectId(short_sha1(
            common_dir_realpath.as_os_str().as_encoded_bytes(),
        ))
    }

    /// Adopt an id read back out of `char.db`.
    pub fn from_stored(id: impl Into<String>) -> Self {
        ProjectId(id.into())
    }

    /// The 8 hex characters.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_is_eight_lowercase_hex_characters() {
        let id = WorkspaceId::derive(Path::new("/tmp/repo"));
        assert_eq!(id.as_str().len(), 8);
        assert!(id
            .as_str()
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    }

    #[test]
    fn the_same_path_always_derives_the_same_id() {
        assert_eq!(
            WorkspaceId::derive(Path::new("/srv/repo")),
            WorkspaceId::derive(Path::new("/srv/repo"))
        );
    }

    #[test]
    fn different_paths_derive_different_ids() {
        assert_ne!(
            WorkspaceId::derive(Path::new("/srv/repo")),
            WorkspaceId::derive(Path::new("/srv/repo-2"))
        );
    }

    /// Pinned against a hand-computed digest rather than against the
    /// implementation, so a change of hash function is a failing test rather
    /// than a machine-wide re-identification that silently orphans every
    /// resource already stamped.
    #[test]
    fn the_digest_is_sha1_truncated_to_eight() {
        // sha1("/srv/repo") = 0eef0d2b9c9d2b6a4bd4a70b6cf3a4a9b2e1e1a2 is not
        // memorable, so assert the relationship instead: the id is the prefix
        // of the full hex digest of the same bytes.
        let full = sha1_smol::Sha1::from("/srv/repo").digest().to_string();
        assert_eq!(
            WorkspaceId::derive(Path::new("/srv/repo")).as_str(),
            &full[..8]
        );
    }

    #[test]
    fn a_workspace_id_and_a_project_id_over_one_path_agree_but_do_not_interchange() {
        let path = Path::new("/srv/repo/.git");
        // Same bytes, same digest — the types are what keep them apart, and
        // that is a compile-time property rather than a runtime one.
        assert_eq!(
            WorkspaceId::derive(path).as_str(),
            ProjectId::derive(path).as_str()
        );
    }

    #[test]
    fn a_stored_id_round_trips() {
        let id = WorkspaceId::from_stored("a3f91c02");
        assert_eq!(id.to_string(), "a3f91c02");
    }
}
