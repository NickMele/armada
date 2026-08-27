//! A [`WorkProduct`] that reads no repository.
//!
//! The gate's question about a diff is *how many files moved*, and answering it
//! for real needs a repository with commits in it. The cases that need git's
//! own opinion live in `adapters`; everything else — a step that changed
//! nothing, a step that changed three files, a repository that would not open —
//! is scripted here.
//!
//! **The refusal is scripted separately from the empty answer**, because those
//! two are the pair the gate must never confuse: a reading that failed is not a
//! diff that was empty.

use std::error::Error;
use std::fmt;
use std::sync::Mutex;

use adapter_traits::{Changed, Patch, WorkProduct, Worktree};

/// Why the fake would not read the worktree.
///
/// One variant: a fake's failure is a script, not a diagnosis. What matters to
/// a caller is that it is an error rather than an empty answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeDiffRefused {
    pub standing_in_for: &'static str,
}

impl fmt::Display for FakeDiffRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "refused, standing in for {}", self.standing_in_for)
    }
}

impl Error for FakeDiffRefused {}

/// A work product that is whatever the test said it is.
///
/// `Mutex` rather than `RefCell` for the reason `FakeVcs` gives: a Fleet is
/// `Sync`, so the seams it holds have to be.
#[derive(Debug, Default)]
pub struct FakeWorkProduct {
    changed: Vec<String>,
    patch: String,
    refuse: Mutex<Option<&'static str>>,
    asked: Mutex<Vec<String>>,
}

impl FakeWorkProduct {
    /// A worktree in which nothing has changed. **The `diff_nonempty` failure
    /// case**, and the reason the constructor is named rather than an empty
    /// list passed to the other one.
    pub fn untouched() -> FakeWorkProduct {
        FakeWorkProduct::default()
    }

    /// A worktree in which these files changed.
    pub fn changed(paths: &[&str]) -> FakeWorkProduct {
        FakeWorkProduct {
            changed: paths.iter().map(|path| path.to_string()).collect(),
            patch: paths
                .iter()
                .map(|path| format!("--- a/{path}\n+++ b/{path}\n"))
                .collect(),
            refuse: Mutex::new(None),
            asked: Mutex::new(Vec::new()),
        }
    }

    /// The patch text a Judge will be handed. Scripted, because what a Judge
    /// reads is the thing under test rather than git's rendering of it.
    pub fn showing(mut self, patch: &str) -> FakeWorkProduct {
        self.patch = patch.to_string();
        self
    }

    /// Make every reading fail as an unopenable repository would.
    pub fn refusing(standing_in_for: &'static str) -> FakeWorkProduct {
        FakeWorkProduct {
            changed: Vec::new(),
            patch: String::new(),
            refuse: Mutex::new(Some(standing_in_for)),
            asked: Mutex::new(Vec::new()),
        }
    }

    /// Every worktree path this fake was asked about, in order. A gate that
    /// never asked is a gate that decided `diff_nonempty` without looking.
    pub fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl WorkProduct for FakeWorkProduct {
    type Error = FakeDiffRefused;

    fn changed_files(&self, worktree: &Worktree) -> Result<Changed, Self::Error> {
        self.asked
            .lock()
            .expect("not poisoned")
            .push(worktree.path().to_string());
        if let Some(standing_in_for) = *self.refuse.lock().expect("not poisoned") {
            return Err(FakeDiffRefused { standing_in_for });
        }
        Ok(Changed::of(self.changed.clone()))
    }

    fn patch(&self, worktree: &Worktree) -> Result<Patch, Self::Error> {
        self.asked
            .lock()
            .expect("not poisoned")
            .push(worktree.path().to_string());
        if let Some(standing_in_for) = *self.refuse.lock().expect("not poisoned") {
            return Err(FakeDiffRefused { standing_in_for });
        }
        Ok(Patch::of(self.patch.clone()))
    }
}
