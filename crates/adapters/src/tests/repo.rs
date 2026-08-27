//! A real git repository that exists for one test and is gone after it.
//!
//! # Why a real one
//!
//! Because what is under test is git's own opinion — whether a branch name is
//! taken, whether an administrative record survives a directory being deleted,
//! whether the outer checkout can see the worktree nested inside it. None of
//! those are computed by this crate, so a fake would be asserting against this
//! crate's guess at what git does. v1's most-defended test made the same call
//! for the same reason.
//!
//! Everything that does **not** need git's opinion is tested without one: the
//! path and branch derivation are unit tests in `adapter-traits`, and every
//! crate above this one fakes the whole trait through `testkit`. This file is
//! the small remainder, not the default.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use git2::{Repository, Signature};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A repository under the system temp directory, removed on drop.
pub struct TempRepo {
    root: PathBuf,
}

impl TempRepo {
    /// A repository with one commit, which is the least a branch can point at.
    pub fn with_a_commit() -> TempRepo {
        let repo = TempRepo::empty();
        repo.commit_everything("the first commit");
        repo
    }

    /// A repository initialised and never committed to.
    pub fn empty() -> TempRepo {
        let root = std::env::temp_dir().join(format!(
            "armada-adapters-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("a temporary directory");
        // Symlinked temp directories are ordinary on macOS, and a path that is
        // not the one git will report back turns every path assertion into a
        // guess. Canonicalise once, here.
        let root = root.canonicalize().expect("a real path");
        // `main` by name rather than by whoever's `init.defaultBranch` is set:
        // a base branch is now something reclaim looks for, and a test that
        // reads the machine's git config is a test that fails elsewhere.
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        Repository::init_opts(&root, &options).expect("a repository");
        TempRepo { root }
    }

    /// The working directory, absolute and canonical.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The working directory as the string a [`WorktreeSpec`] takes.
    ///
    /// [`WorktreeSpec`]: adapter_traits::WorktreeSpec
    pub fn root_str(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }

    pub fn open(&self) -> Repository {
        Repository::open(&self.root).expect("the repository")
    }

    /// Write a file, relative to the working directory.
    pub fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the parent directory");
        }
        std::fs::write(path, contents).expect("the file");
    }

    /// Stage everything git can see and commit it.
    ///
    /// The signature is a fixed literal rather than the machine's git config:
    /// this repository is public, and a test that reads whoever is at the
    /// keyboard is a test that fails on somebody else's machine as well as
    /// naming them.
    pub fn commit_everything(&self, message: &str) {
        let repo = self.open();
        let mut index = repo.index().expect("the index");
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .expect("staged everything");
        index.write().expect("the index written");
        let tree = repo
            .find_tree(index.write_tree().expect("a tree"))
            .expect("the tree");
        let who = Signature::now("armada", "armada@example.invalid").expect("a signature");
        let parents = match repo.head().ok().and_then(|h| h.peel_to_commit().ok()) {
            Some(parent) => vec![parent],
            None => Vec::new(),
        };
        let borrowed: Vec<&git2::Commit<'_>> = parents.iter().collect();
        repo.commit(Some("HEAD"), &who, &who, message, &tree, &borrowed)
            .expect("a commit");
    }

    /// Write one file and commit only that file.
    ///
    /// [`TempRepo::commit_everything`] stages the whole tree, and libgit2
    /// refuses to stage `.armada/worktrees/<job>/` once a worktree is nested
    /// under it. These cases move the base branch on **while a Job's worktree
    /// exists**, which is the whole scenario, so the pathspec is the only way to
    /// express it.
    pub fn commit_one(&self, relative: &str, contents: &str, message: &str) {
        self.write(relative, contents);
        self.git(&["add", "--", relative]);
        self.git(&[
            "-c",
            "user.name=armada",
            "-c",
            "user.email=armada@example.invalid",
            "commit",
            "-m",
            message,
        ]);
    }

    /// Run `git` in this repository and assert it worked.
    ///
    /// The command line rather than the library, for the reason
    /// [`TempRepo::status`] gives — and because the delivery cases set up a
    /// remote, which is a thing a person does with `git remote add`.
    pub fn git(&self, args: &[&str]) -> String {
        let run = std::process::Command::new("git")
            .args(["-C", &self.root_str()])
            .args(args)
            .output()
            .expect("git on PATH — a check nothing can run is a check that does not exist");
        assert!(
            run.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        String::from_utf8_lossy(&run.stdout).trim().to_string()
    }

    /// A bare repository beside this one, wired up as `origin`.
    ///
    /// **Never a real remote.** A push in a test that reached a network would
    /// be a test that needs a credential and touches somebody's account.
    pub fn with_a_bare_remote(&self) -> PathBuf {
        let bare = self.root.with_extension("remote.git");
        Repository::init_bare(&bare).expect("a bare repository");
        let bare = bare.canonicalize().expect("a real path");
        self.git(&["remote", "add", "origin", &bare.to_string_lossy()]);
        bare
    }

    /// What a person's `git status --porcelain` reports, one entry per line.
    ///
    /// **The command-line git, not the library**, and the difference is the
    /// finding: see [`TempRepo::status_via_the_library`]. The question these
    /// tests ask — can the outer checkout see the worktree nested inside it —
    /// is a question about what a person is shown, so it is asked of the thing
    /// that shows them.
    pub fn status(&self) -> Vec<String> {
        let run = std::process::Command::new("git")
            .args(["-C", &self.root_str(), "status", "--porcelain"])
            .output()
            .expect("git on PATH — a check nothing can run is a check that does not exist");
        assert!(run.status.success(), "git status failed");
        String::from_utf8_lossy(&run.stdout)
            .lines()
            .map(|line| line[3..].to_string())
            .collect()
    }

    /// The same question asked of the library, which answers differently.
    ///
    /// libgit2 does **not** report a directory holding a `.git` entry as
    /// untracked — it reads it as a repository of its own and drops it. The
    /// command line reports `?? .armada/`. Neither is wrong; they are answering
    /// slightly different questions, and anything that comes to depend on the
    /// library's untracked/ignored split needs to know that before it trusts
    /// it for a guard.
    pub fn status_via_the_library(&self) -> Vec<String> {
        let repo = self.open();
        let mut options = git2::StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);
        let statuses = repo.statuses(Some(&mut options)).expect("a status");
        let paths = statuses
            .iter()
            .filter_map(|entry| entry.path().map(str::to_string))
            .collect();
        paths
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
        let _ = std::fs::remove_dir_all(self.root.with_extension("remote.git"));
    }
}
