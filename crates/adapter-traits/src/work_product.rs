//! What a Job has produced in its worktree, read by Fleet for itself.
//!
//! # Why this is not a method on [`Vcs`](crate::Vcs)
//!
//! A `Vcs` is what creates a worktree, and it is held at approval time by the
//! code that dispatches. This is held by the gate, and the gate must not be
//! able to create anything. Two capabilities, two traits, and neither call site
//! can reach the other's methods — the same split that keeps push off the
//! Drone-facing type.
//!
//! # It reads, and there is nothing here that writes
//!
//! No commit, no stage, no reset, no push, no removal. A gate that could commit
//! could satisfy `diff_nonempty` on the Drone's behalf, which would be Armada
//! producing the evidence it is meant to be checking.
//!
//! # Paths and a patch, read separately
//!
//! [`Changed`] carries which files moved; [`Patch`] carries what changed in
//! them. Two calls rather than one because the mechanical tier only ever needs
//! a count, and the bytes are read solely when a step's Judge fires — which on
//! most steps is never. A single call returning both would put the expensive
//! read behind every `diff_nonempty`.

use alloc::string::String;
use alloc::vec::Vec;

use crate::worktree::Worktree;

/// The files a Job has changed in its worktree.
///
/// **Fleet's own reading**, never a number a Drone reported. `diff_nonempty` is
/// a gating check, and a gating fact that arrives from the thing being gated is
/// not a fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changed {
    paths: Vec<String>,
}

impl Changed {
    /// Record what an implementation found. Called by the implementation that
    /// read the repository, and by a fake standing in for one.
    pub fn of(paths: Vec<String>) -> Changed {
        Changed { paths }
    }

    /// Nothing changed. Named, because it is the answer that fails a
    /// `diff_nonempty` check and a caller writing `Changed::of(Vec::new())`
    /// reads as though something went wrong.
    pub fn nothing() -> Changed {
        Changed { paths: Vec::new() }
    }

    /// Repository-relative, in the order the implementation reported them.
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}

/// What a worktree held at one instant, as content rather than as a file list.
///
/// **The baseline a step is measured against.** [`Changed`] answers what the
/// *branch* has done since it was cut, which is the right question for a Job
/// and the wrong one for a step: every step after the first one that writes
/// anything inherits its predecessor's files and passes `diff_nonempty` on
/// them. Armada shipped that, and a step that produced nothing at all advanced
/// on the previous step's scope note.
///
/// Content rather than paths, because a step that edits a file an earlier step
/// created changes nothing about which paths are listed.
///
/// **Comparable only against another reading of the same worktree by the same
/// implementation, in the same process.** The strings are opaque here — an
/// implementation says what a path holds in whatever way it can, and nothing
/// reads them except [`differs_from`](Footprint::differs_from).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Footprint {
    of: Vec<(String, String)>,
}

impl Footprint {
    /// Record what an implementation read. Sorted here rather than at the call
    /// site, so two readings that found the same worktree in a different order
    /// are the same footprint.
    pub fn of(mut entries: Vec<(String, String)>) -> Footprint {
        entries.sort();
        Footprint { of: entries }
    }

    /// A worktree holding no change at all. Named for [`Changed::nothing`]'s
    /// reason: the empty case is an answer, not a failure to read.
    pub fn nothing() -> Footprint {
        Footprint { of: Vec::new() }
    }

    /// Whether anything moved between the two readings.
    ///
    /// **The whole of what a footprint is for.** There is no accessor for the
    /// entries, so nothing can grow a rule about *what* changed out of a type
    /// that only knows *that* something did.
    pub fn differs_from(&self, before: &Footprint) -> bool {
        self.of != before.of
    }

    /// How many paths this reading found. Never compared against another
    /// footprint's — two readings can hold the same count and different files.
    pub fn len(&self) -> usize {
        self.of.len()
    }

    pub fn is_empty(&self) -> bool {
        self.of.is_empty()
    }
}

/// The patch a Job's work makes, as text.
///
/// **The Judge's whole sight of the repository.** It is pre-loaded into the
/// call rather than fetched, which is what keeps a Judge call stateless, cheap
/// and reproducible — a verifier that can go and look is a verifier whose
/// answer depends on when it looked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Patch {
    text: String,
}

impl Patch {
    /// Record what an implementation read.
    pub fn of(text: String) -> Patch {
        Patch { text }
    }

    /// The unified diff, as the implementation rendered it.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// Reading a Job's work product out of its worktree.
pub trait WorkProduct {
    /// Errors this implementation can raise. Named by the implementation, so a
    /// caller can tell a repository it could not open from a worktree with
    /// nothing in it — which are the same empty answer and opposite meanings.
    type Error;

    /// Which files the Job has changed since its branch started, committed or
    /// not.
    ///
    /// **A failure is never an empty answer.** A repository that will not open
    /// returns an error, so a `diff_nonempty` check cannot pass or fail on a
    /// reading that never happened.
    fn changed_files(&self, worktree: &Worktree) -> Result<Changed, Self::Error>;

    /// What the worktree holds right now, as content, so that two readings can
    /// be compared.
    ///
    /// **Read at a step's start and again at its gate.** The difference is what
    /// *that step* produced, which is the question `diff_nonempty` asks and the
    /// one [`changed_files`](WorkProduct::changed_files) cannot answer — it
    /// measures against the commit the branch was cut from, so it counts every
    /// earlier step's work as this one's.
    ///
    /// A failure is an error rather than an empty footprint, for
    /// [`changed_files`](WorkProduct::changed_files)'s reason: two readings that
    /// both failed would compare equal and report a step that did nothing.
    fn footprint(&self, worktree: &Worktree) -> Result<Footprint, Self::Error>;

    /// What changed in those files, as a unified diff.
    ///
    /// **Read only when a Judge fires**, which is why it is a second method:
    /// the bytes are large, most steps ask no semantic question, and a call
    /// that returned them alongside the file list would pay for them on every
    /// gate.
    ///
    /// A failure is an error rather than an empty patch, for the reason
    /// [`changed_files`](WorkProduct::changed_files) gives — the Judge must
    /// never be handed a reading that did not happen and told it is the work.
    fn patch(&self, worktree: &Worktree) -> Result<Patch, Self::Error>;
}
