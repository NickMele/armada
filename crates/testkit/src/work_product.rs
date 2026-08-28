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
//!
//! # The worktree moves, because a step boundary needs it to
//!
//! A [`Since`] is what one step inherited from the last, and the case it exists
//! for is a step that added nothing to it. So the scripted list is mutable and
//! every file carries a fingerprint — [`FakeWorkProduct::wrote`] is a Drone
//! putting bytes on disk. What a footing taken from this fake holds is
//! [`inherited`](FakeWorkProduct::inherited)'s to say, and by default nothing.

use std::error::Error;
use std::fmt;
use std::sync::Mutex;

use adapter_traits::{Change, Changed, ChangedFile, Patch, Since, Was, WorkProduct, Worktree};

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

/// One file in the fake worktree: what happened to it, and which writing put
/// it there. The fingerprint is a counter rather than a hash — what a
/// [`Since`] compares is identity, and a counter has that and nothing else.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Wrote {
    path: String,
    change: Change,
    written: u64,
}

/// A work product that is whatever the test said it is.
///
/// `Mutex` rather than `RefCell` for the reason `FakeVcs` gives: a Fleet is
/// `Sync`, so the seams it holds have to be.
#[derive(Debug, Default)]
pub struct FakeWorkProduct {
    changed: Mutex<Vec<Wrote>>,
    patch: Mutex<String>,
    /// How many writings have happened. The next one's fingerprint.
    writings: Mutex<u64>,
    /// Whether a footing taken from this worktree carries what is in it. See
    /// this module — off by default, and the reason is what the scripted list
    /// means to the tests that already existed.
    inherits: Mutex<bool>,
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

    /// A worktree in which these files changed. **Every one modified**, which
    /// is the kind that says the least: a test that cares which kind says so
    /// with [`FakeWorkProduct::changing`].
    pub fn changed(paths: &[&str]) -> FakeWorkProduct {
        FakeWorkProduct::changing(
            &paths
                .iter()
                .map(|path| (*path, Change::Modified))
                .collect::<Vec<(&str, Change)>>(),
        )
    }

    /// A worktree in which these files changed, each in a stated way. The
    /// constructor for a test whose subject is the kind rather than the count.
    pub fn changing(files: &[(&str, Change)]) -> FakeWorkProduct {
        let fake = FakeWorkProduct {
            patch: Mutex::new(
                files
                    .iter()
                    .map(|(path, _)| format!("--- a/{path}\n+++ b/{path}\n"))
                    .collect(),
            ),
            ..FakeWorkProduct::default()
        };
        fake.wrote(files);
        fake
    }

    /// A Drone putting bytes on disk. Each call is a fresh writing, so a file
    /// named here reads as changed *since* any [`Since`] minted before it — and
    /// a file left out keeps the fingerprint it already had.
    ///
    /// Takes `&self` because the fake is behind a Fleet by the time a test wants
    /// the worktree to move.
    pub fn wrote(&self, files: &[(&str, Change)]) {
        let mut writings = self.writings.lock().expect("not poisoned");
        *writings += 1;
        let written = *writings;
        let mut changed = self.changed.lock().expect("not poisoned");
        for (path, change) in files {
            changed.retain(|was| was.path != *path);
            changed.push(Wrote {
                path: path.to_string(),
                change: *change,
                written,
            });
        }
    }

    /// Everything scripted so far is work an earlier step did.
    ///
    /// **The one way a fake worktree has a history**, and the switch a test of
    /// the step boundary turns on: after it, a step that writes nothing of its
    /// own is credited with nothing, which is the defect `diff_nonempty` was
    /// missing.
    pub fn inherited(self) -> FakeWorkProduct {
        *self.inherits.lock().expect("not poisoned") = true;
        self
    }

    /// The patch text a Judge will be handed. Scripted, because what a Judge
    /// reads is the thing under test rather than git's rendering of it.
    pub fn showing(self, patch: &str) -> FakeWorkProduct {
        *self.patch.lock().expect("not poisoned") = patch.to_string();
        self
    }

    /// Make every reading fail as an unopenable repository would.
    pub fn refusing(standing_in_for: &'static str) -> FakeWorkProduct {
        FakeWorkProduct {
            refuse: Mutex::new(Some(standing_in_for)),
            ..FakeWorkProduct::default()
        }
    }

    /// Every worktree path this fake was asked about **for a decision or a
    /// view**, in order. A gate that never asked is a gate that decided
    /// `diff_nonempty` without looking.
    ///
    /// [`already_there`](WorkProduct::already_there) is deliberately not in it.
    /// It is read once when a step begins, by the boundary rather than by the
    /// gate or the live view, and counting it here would make the footprint's
    /// throttle look as though it had read a repository it never opened.
    pub fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl FakeWorkProduct {
    /// The reading every method starts from: the refusal, then the list.
    fn read(&self, worktree: &Worktree) -> Result<Vec<Wrote>, FakeDiffRefused> {
        self.asked
            .lock()
            .expect("not poisoned")
            .push(worktree.path().to_string());
        match *self.refuse.lock().expect("not poisoned") {
            Some(standing_in_for) => Err(FakeDiffRefused { standing_in_for }),
            None => Ok(self.changed.lock().expect("not poisoned").clone()),
        }
    }
}

impl WorkProduct for FakeWorkProduct {
    type Error = FakeDiffRefused;

    fn already_there(&self, _worktree: &Worktree) -> Result<Since, Self::Error> {
        if let Some(standing_in_for) = *self.refuse.lock().expect("not poisoned") {
            return Err(FakeDiffRefused { standing_in_for });
        }
        let changed = self.changed.lock().expect("not poisoned").clone();
        if !*self.inherits.lock().expect("not poisoned") {
            return Ok(Since::the_branch_started());
        }
        Ok(Since::was_there(
            changed
                .into_iter()
                .map(|was| Was::new(was.path, was.written.to_string()))
                .collect(),
        ))
    }

    fn changed_files(&self, worktree: &Worktree, since: &Since) -> Result<Changed, Self::Error> {
        Ok(Changed::of(
            mine(self.read(worktree)?, since)
                .into_iter()
                .map(|was| ChangedFile::new(was.path, was.change))
                .collect(),
        ))
    }

    /// Empty where the step wrote nothing, and the scripted text otherwise.
    ///
    /// The real one narrows the patch to the step's own files; this cannot,
    /// because the text is scripted rather than rendered. What it does keep is
    /// the property the gate turns on — a step that produced nothing hands the
    /// Judge nothing.
    fn patch(&self, worktree: &Worktree, since: &Since) -> Result<Patch, Self::Error> {
        let mine = mine(self.read(worktree)?, since);
        if mine.is_empty() {
            return Ok(Patch::of(String::new()));
        }
        Ok(Patch::of(self.patch.lock().expect("not poisoned").clone()))
    }
}

/// The files that are this step's own: not there, or not as they were, when it
/// began.
fn mine(changed: Vec<Wrote>, since: &Since) -> Vec<Wrote> {
    changed
        .into_iter()
        .filter(|was| !since.covers(&was.path, &was.written.to_string()))
        .collect()
}
