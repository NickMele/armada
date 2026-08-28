//! A [`WorkProduct`] that reads no repository.
//!
//! The gate's question about a diff is *whether this step moved anything*, and
//! answering it for real needs a repository with commits in it. The cases that
//! need git's own opinion live in `adapters`; everything else — a step that
//! changed nothing, a step that changed three files, a repository that would
//! not open — is scripted here.
//!
//! **The refusal is scripted separately from the empty answer**, because those
//! two are the pair the gate must never confuse: a reading that failed is not a
//! diff that was empty.
//!
//! The scripted list is behind a `Mutex` and [`FakeWorkProduct::wrote`] is a
//! Drone putting bytes on disk, because a live view is read over and over and a
//! fake with one fixed answer cannot tell a Drone that is writing from one that
//! has stopped.
//!
//! Two ledgers: [`asked`](FakeWorkProduct::asked) is every reading of any kind,
//! and [`listed`](FakeWorkProduct::listed) is the file-list ones alone. See
//! `listed` for why the live view needs its own.

use std::error::Error;
use std::fmt;
use std::sync::Mutex;

use adapter_traits::{Change, Changed, ChangedFile, Footprint, Patch, WorkProduct, Worktree};

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

/// One file in the fake worktree, and what happened to it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Wrote {
    path: String,
    change: Change,
}

/// A work product that is whatever the test said it is.
///
/// `Mutex` rather than `RefCell` for the reason `FakeVcs` gives: a Fleet is
/// `Sync`, so the seams it holds have to be.
#[derive(Debug, Default)]
pub struct FakeWorkProduct {
    changed: Mutex<Vec<Wrote>>,
    patch: Mutex<String>,
    refuse: Mutex<Option<&'static str>>,
    asked: Mutex<Vec<String>>,
    listed: Mutex<Vec<String>>,
    /// Whether the work moves between one footprint and the next.
    ///
    /// A footprint is read at a step's start and again at its gate, and what
    /// `diff_nonempty` asks is whether the two differ. A fake therefore has to
    /// model time: `true` is a Drone that is working, `false` is one that found
    /// the files already there and added nothing — the defect that let a step
    /// advance on its predecessor's scope note.
    moving: bool,
    readings: Mutex<usize>,
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
            moving: true,
            ..FakeWorkProduct::default()
        };
        fake.wrote(files);
        fake
    }

    /// A worktree holding these files **before this step began**, with the step
    /// adding nothing to them.
    ///
    /// The shape of the defect: every step after the first one that writes
    /// anything inherits its predecessor's files, and a `diff_nonempty` read
    /// against the branch rather than against the step passed on them. A Job
    /// advanced through `implement` having written no code, credited with the
    /// scope note the step before it had committed.
    pub fn inherited(paths: &[&str]) -> FakeWorkProduct {
        FakeWorkProduct {
            moving: false,
            ..FakeWorkProduct::changed(paths)
        }
    }

    /// A Drone putting bytes on disk. A file named here replaces whatever was
    /// at that path, and a file left out stays as it was.
    ///
    /// Takes `&self` because the fake is behind a Fleet by the time a test wants
    /// the worktree to move.
    pub fn wrote(&self, files: &[(&str, Change)]) {
        let mut changed = self.changed.lock().expect("not poisoned");
        for (path, change) in files {
            changed.retain(|was| was.path != *path);
            changed.push(Wrote {
                path: path.to_string(),
                change: *change,
            });
        }
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
            moving: true,
            ..FakeWorkProduct::default()
        }
    }

    /// Every worktree path this fake was asked about, in order, whatever was
    /// asked. A gate that never asked is a gate that decided `diff_nonempty`
    /// without looking.
    pub fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }

    /// Every worktree path this fake was asked for a **file list**, in order —
    /// the live view a Bridge watches, and the scope check. The step's
    /// footprint readings are not in it: see this module.
    pub fn listed(&self) -> Vec<String> {
        self.listed.lock().expect("not poisoned").clone()
    }
}

impl FakeWorkProduct {
    /// The reading the file list and the patch start from: the refusal, then
    /// the list.
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

    fn changed_files(&self, worktree: &Worktree) -> Result<Changed, Self::Error> {
        self.listed
            .lock()
            .expect("not poisoned")
            .push(worktree.path().to_string());
        Ok(Changed::of(
            self.read(worktree)?
                .into_iter()
                .map(|was| ChangedFile::new(was.path, was.change))
                .collect(),
        ))
    }

    fn footprint(&self, worktree: &Worktree) -> Result<Footprint, Self::Error> {
        self.asked
            .lock()
            .expect("not poisoned")
            .push(worktree.path().to_string());
        if let Some(standing_in_for) = *self.refuse.lock().expect("not poisoned") {
            return Err(FakeDiffRefused { standing_in_for });
        }
        // The reading number stands in for the bytes. A worktree nothing
        // changed answers the same footprint every time however this counts,
        // because there is nothing in it to carry the number.
        let mut readings = self.readings.lock().expect("not poisoned");
        *readings += 1;
        let held = match self.moving {
            true => format!("reading-{readings}"),
            false => String::from("as the step found it"),
        };
        Ok(Footprint::of(
            self.changed
                .lock()
                .expect("not poisoned")
                .iter()
                .map(|was| (was.path.clone(), held.clone()))
                .collect(),
        ))
    }

    fn patch(&self, worktree: &Worktree) -> Result<Patch, Self::Error> {
        self.read(worktree)?;
        Ok(Patch::of(self.patch.lock().expect("not poisoned").clone()))
    }
}
