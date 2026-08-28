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
//! [`Changed`] carries which files moved and how — added, modified, deleted;
//! [`Patch`] carries what changed inside them. Two calls rather than one because the mechanical tier only ever needs
//! a count, and the bytes are read solely when a step's Judge fires — which on
//! most steps is never. A single call returning both would put the expensive
//! read behind every `diff_nonempty`.

use alloc::string::String;
use alloc::vec::Vec;

use crate::worktree::Worktree;

/// What happened to one file.
///
/// **A closed set that cannot need widening**, which is why every delta the
/// implementation can report has a variant here rather than a shared "other".
/// It is read by a person watching a Drone work, and the moment it grows a
/// variant the surface reading it has a case it was not built for — so the set
/// is drawn once, from what git itself can say, and left alone.
///
/// An untracked file is [`Change::Added`]: a Drone that wrote a new file and
/// did not stage it has added it, and the staging is not the part anyone
/// watching is asking about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// The file did not exist at the base. Staged or not.
    Added,
    /// The file existed and its contents moved.
    Modified,
    /// The file existed at the base and does not now.
    Deleted,
    /// The same content under a different path.
    Renamed,
    /// The same content under a second path, the first still there.
    Copied,
    /// A file became a directory, a symlink, or the reverse. Not a content
    /// change, and a surface that showed it as one would be wrong.
    TypeChanged,
    /// A merge left conflict markers rather than a resolved file.
    Conflicted,
    /// The file is in the diff and could not be read. **Not an empty change** —
    /// the reading of that one path failed and the rest did not.
    Unreadable,
}

/// One file the Job has changed, and what happened to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedFile {
    path: String,
    change: Change,
}

impl ChangedFile {
    pub fn new(path: String, change: Change) -> ChangedFile {
        ChangedFile { path, change }
    }

    /// Repository-relative, as the implementation read it.
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn change(&self) -> Change {
        self.change
    }
}

/// The files a Job has changed in its worktree.
///
/// **Fleet's own reading**, never a number a Drone reported. `diff_nonempty` is
/// a gating check, and a gating fact that arrives from the thing being gated is
/// not a fact.
///
/// It carries a [`Change`] per file and not only a path. The kind costs
/// nothing — the delta the implementation already walked is where it comes
/// from — and a list of bare names cannot tell a file the Drone deleted from
/// one it wrote, which is the difference a person watching is looking for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changed {
    files: Vec<ChangedFile>,
}

impl Changed {
    /// Record what an implementation found. Called by the implementation that
    /// read the repository, and by a fake standing in for one.
    pub fn of(files: Vec<ChangedFile>) -> Changed {
        Changed { files }
    }

    /// Nothing changed. Named, because it is the answer that fails a
    /// `diff_nonempty` check and a caller writing `Changed::of(Vec::new())`
    /// reads as though something went wrong.
    pub fn nothing() -> Changed {
        Changed { files: Vec::new() }
    }

    /// Every file, with its kind, in the order the implementation reported
    /// them.
    pub fn files(&self) -> &[ChangedFile] {
        &self.files
    }

    /// Repository-relative, in the order the implementation reported them.
    ///
    /// A fresh `Vec` rather than a borrow: the scope comparison takes a slice
    /// of owned paths, and the kind lives beside each path rather than in a
    /// second list that could disagree with the first.
    pub fn paths(&self) -> Vec<String> {
        self.files.iter().map(|file| file.path.clone()).collect()
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
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

/// One file as it stood when a step began.
///
/// The fingerprint is the implementation's own identity for the bytes — git's
/// blob id, for the one that reads a repository. It is carried alongside the
/// path because a path alone cannot tell a file this step left untouched from
/// one it rewrote, and a step that edits what an earlier step wrote is doing
/// work.
///
/// An empty fingerprint is "no readable file at this path", which is what a
/// deletion leaves behind. Two unreadable states compare equal, which is the
/// right answer: neither is this step having written something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Was {
    path: String,
    fingerprint: String,
}

impl Was {
    pub fn new(path: String, fingerprint: String) -> Was {
        Was { path, fingerprint }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

/// What a reading of a Job's work product is measured from.
///
/// **Both readings take one and neither answers without it**, and there is no
/// default. A caller either asks about the whole branch or about one step's own
/// work, and the two are different questions that were being answered by the
/// same call — which is how a step that produced nothing was credited with an
/// earlier step's file.
///
/// # A step's footing is a reading, not a record
///
/// It is minted by [`WorkProduct::already_there`] at the instant a step begins
/// and held on the working slot for as long as that step runs. Nothing writes
/// it down. The alternative — a base commit on the step's row — is the failure
/// this project already knows by name, a field on a record that can disagree
/// with the record holding it; and a commit could not carry the answer anyway,
/// because a Drone may leave the step before it ends uncommitted and a base
/// read as `HEAD` would hand the next step its files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Since {
    already: Vec<Was>,
}

impl Since {
    /// Since the branch was cut: every step's work together. What a person
    /// reading a whole Job is shown, and never what a step is gated on.
    pub fn the_branch_started() -> Since {
        Since {
            already: Vec::new(),
        }
    }

    /// Whatever was already in the worktree, as the implementation read it.
    /// Called by that implementation and by a fake standing in for one.
    ///
    /// An empty list is the same value [`the_branch_started`](Since::the_branch_started)
    /// makes, and correctly so: a step that begins with nothing there inherits
    /// nothing. The two names exist because the call sites are asking different
    /// questions and a reader of either should be able to tell which.
    pub fn was_there(already: Vec<Was>) -> Since {
        Since { already }
    }

    /// Whether this path, holding these bytes, was already there when the step
    /// began — and is therefore not this step's work.
    ///
    /// **Both halves.** A path that was there and whose bytes have moved is
    /// this step editing what an earlier one wrote, which is work.
    pub fn covers(&self, path: &str, fingerprint: &str) -> bool {
        self.already
            .iter()
            .any(|was| was.path == path && was.fingerprint == fingerprint)
    }

    /// Whether anything was inherited at all. Where nothing was, an
    /// implementation can skip fingerprinting entirely — there is nothing to
    /// subtract.
    pub fn is_empty(&self) -> bool {
        self.already.is_empty()
    }
}

/// Reading a Job's work product out of its worktree.
pub trait WorkProduct {
    /// Errors this implementation can raise. Named by the implementation, so a
    /// caller can tell a repository it could not open from a worktree with
    /// nothing in it — which are the same empty answer and opposite meanings.
    type Error;

    /// What is in the worktree now, in the form a later reading measures
    /// against. Called once, at the instant a step begins.
    ///
    /// **It writes nothing.** A fingerprint is computed, never stored, so this
    /// stays a capability that only reads — a gate that could commit could
    /// satisfy `diff_nonempty` on the Drone's behalf.
    fn already_there(&self, worktree: &Worktree) -> Result<Since, Self::Error>;

    /// Which files have changed since `since`, committed or not.
    ///
    /// **A failure is never an empty answer.** A repository that will not open
    /// returns an error, so a `diff_nonempty` check cannot pass or fail on a
    /// reading that never happened.
    fn changed_files(&self, worktree: &Worktree, since: &Since) -> Result<Changed, Self::Error>;

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
    fn patch(&self, worktree: &Worktree, since: &Since) -> Result<Patch, Self::Error>;
}
