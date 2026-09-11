//! Where one repository's Job records live, off the checkout entirely.
//!
//! A Judge's brief, a Drone's transcript, a Job's log, a Check's output, a
//! kept deliverable and a kept frame used to be written under the repository's
//! own `.armada/` — `.gitignore`d, but still there, and still reachable by a
//! `git clean -fdx` inside a worktree. [`root`] is what every writer and every
//! reader uses instead. Worktrees, `armada.yml` and `.armada/workflows/` are
//! not affected: a worktree is a Job's own working copy and the other two are
//! read rather than written.
//!
//! **The key is a digest, not the sanitized path.** Sanitizing a path for a
//! filesystem loses information — `/x/a/b` and `/x/a_b` both reduce to
//! `x_a_b` under the obvious replacement — and `DefaultHasher` avoids that but
//! documents its own algorithm as free to change between compiler releases,
//! which would silently orphan a repository's records on a Rust upgrade.
//! `Sha1` is already in the workspace's dependency graph and its output is a
//! published, forever-stable function of the bytes given it — taken over
//! [`crate::daemon::Host::repo_root`], canonicalized, never a Manifest's id.
//!
//! **`.armada/<kind>/` still appears one level under [`root`].** That looks
//! redundant once nothing needs hiding from `git status`, and it stays because
//! a Job's own row already spells the relative half of the path —
//! `job_step_checks.output_path`, `KeptDeliverable.path` — and [`migrating`]
//! does not rewrite a row, only moves the file the row already names.

use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};

pub mod migrating;

/// The six directories that used to sit under a repository's own `.armada/`,
/// and now sit under [`root`] instead. Named once so [`migrating`] and any
/// future reader of the whole set share one list rather than five or six
/// spellings of it.
pub const KINDS: &[&str] = &[
    "briefs",
    "transcripts",
    "logs",
    "checks",
    "deliverables",
    "frames",
];

/// Where this repository's records live, under Fleet's own data directory.
///
/// `data_dir` is the directory [`crate::runtime::machine_path`] resolves —
/// `~/Library/Application Support/Armada/`, the same directory `armada.db`
/// and `fleet.json` are already in. This does not create it; the caller
/// that already creates `data_dir` creates this alongside it.
pub fn root(data_dir: &Path, repo_root: &str) -> PathBuf {
    data_dir.join("repos").join(key(repo_root))
}

/// The one two repositories cannot share. See the module doc for why a digest
/// and not a sanitized path.
fn key(repo_root: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(repo_root.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_repositories_never_share_a_key() {
        assert_ne!(key("/repos/one/x"), key("/repos/one_x"));
    }

    #[test]
    fn the_same_repository_always_answers_the_same_key() {
        assert_eq!(key("/repos/example"), key("/repos/example"));
    }

    #[test]
    fn the_key_is_a_single_path_component() {
        let made = key("/repos/example");
        assert!(!made.is_empty() && !made.contains('/'));
    }

    #[test]
    fn root_is_under_data_dir_and_named_by_the_key() {
        let data_dir = Path::new("/tmp/armada-data");
        let made = root(data_dir, "/repos/example");
        assert_eq!(made, data_dir.join("repos").join(key("/repos/example")));
    }

    /// **`scripts/job` reads the same directory Fleet writes to, without
    /// asking Fleet** — it has to work with Fleet down or wedged, which is
    /// the whole reason it reads disk at all. That only holds if its own
    /// `hashlib.sha1` matches this one on the same input, and nothing short
    /// of running both would prove it.
    #[test]
    fn matches_the_python_reader_in_scripts_job() {
        assert_eq!(
            key("/repos/example"),
            "370e9e671d527ae62553164130fd8c11d6c66640"
        );
    }
}
