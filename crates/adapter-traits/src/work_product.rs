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

impl Change {
    /// Every variant, so [`from_wire`](Change::from_wire) is a search over the
    /// set rather than a second `match` that could fall behind the first.
    pub const ALL: [Change; 8] = [
        Change::Added,
        Change::Modified,
        Change::Deleted,
        Change::Renamed,
        Change::Copied,
        Change::TypeChanged,
        Change::Conflicted,
        Change::Unreadable,
    ];

    /// The stored spelling, which is also what the wire carries.
    ///
    /// **One spelling, not two.** A footprint kept past the Drone that made it
    /// is written into the store by `fleet` and read back out of it by the
    /// same crate that puts it on the wire, so a column and a JSON field that
    /// disagreed would be one reading rendered two ways.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Change::Added => "added",
            Change::Modified => "modified",
            Change::Deleted => "deleted",
            Change::Renamed => "renamed",
            Change::Copied => "copied",
            Change::TypeChanged => "type_changed",
            Change::Conflicted => "conflicted",
            Change::Unreadable => "unreadable",
        }
    }

    /// Read a stored value back. `None` where the value is not one this build
    /// knows, which the reader refuses rather than dropping the file: a
    /// footprint missing a row reads as work that was never done.
    pub fn from_wire(value: &str) -> Option<Change> {
        Change::ALL
            .iter()
            .copied()
            .find(|change| change.as_wire() == value)
    }
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

/// How many lines one file gained and lost.
///
/// **The pair is one measurement.** Additions and deletions come from one walk
/// of one file's patch, so a file is counted or it is not — a binary file, or
/// one whose patch would not build, has neither number rather than a zero for
/// the half that could be had. That is why this is a struct and not two
/// independent options: the state where one arrived and the other did not
/// cannot be written down.
///
/// **Zero is a measurement.** A rename that moved a file and edited nothing is
/// `0` and `0`, and a reader that took an absent count for zero would call that
/// the same as a file nobody could count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCount {
    added: u32,
    deleted: u32,
}

impl LineCount {
    pub fn of(added: u32, deleted: u32) -> LineCount {
        LineCount { added, deleted }
    }

    pub fn added(&self) -> u32 {
        self.added
    }

    pub fn deleted(&self) -> u32 {
        self.deleted
    }
}

/// One file a Job changed, with what it cost in lines where that could be read.
///
/// **[`ChangedFile`] and a count, rather than a second spelling of the pair.**
/// The path and the kind are the same fact the live reading carries, and a
/// second struct restating them is where the two would come to disagree about
/// what a deletion is called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountedFile {
    file: ChangedFile,
    lines: Option<LineCount>,
}

impl CountedFile {
    pub fn new(file: ChangedFile, lines: Option<LineCount>) -> CountedFile {
        CountedFile { file, lines }
    }

    pub fn path(&self) -> &str {
        self.file.path()
    }

    pub fn change(&self) -> Change {
        self.file.change()
    }

    /// What the file gained and lost, or **nothing where it could not be
    /// counted** — a binary file, or a patch that would not build. Absent is
    /// not zero: see [`LineCount`].
    pub fn lines(&self) -> Option<LineCount> {
        self.lines
    }
}

/// The files a Job changed, each with what it cost in lines.
///
/// **The expensive reading of the two, and it is a separate type so that the
/// live one cannot accidentally become it.** [`Changed`] is what a watcher gets
/// and it has no counts to draw; nothing that holds a `Changed` can spend the
/// count by mistake, because the value is not in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counted {
    files: Vec<CountedFile>,
}

impl Counted {
    /// Record what an implementation found, in the order it found it.
    pub fn of(files: Vec<CountedFile>) -> Counted {
        Counted { files }
    }

    pub fn files(&self) -> &[CountedFile] {
        &self.files
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
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

    /// The same files, each with the lines it gained and lost.
    ///
    /// **Measured, and it is the same walk that renders the patch.** All four
    /// routes to a per-file count were timed and they cost the same thing: the
    /// diff library builds one file's count by running the xdiff that would
    /// have produced its text, and the one call that skips the per-file work
    /// answers totals only. Against this repository, release build: 1.3ms over
    /// 6 files and 105 lines, 25ms over 104 files and 7.7k, 90ms over 414 files
    /// and 59k — where [`changed_files`](WorkProduct::changed_files) is a delta
    /// walk costing under a microsecond over the same three.
    ///
    /// **So this is called once, at the transition that ends a Job**, and never
    /// on the live seam: `fleet`'s watcher reads every two seconds inside a
    /// 250ms turn, and 90ms of that turn is a third of Fleet's budget spent on
    /// a number nobody can read changing that fast.
    ///
    /// A failure is an error rather than an empty answer, for
    /// [`changed_files`](WorkProduct::changed_files)'s reason. A file whose
    /// count alone could not be read is a [`CountedFile`] with no
    /// [`LineCount`] — absent, not zero — because losing the path would be
    /// losing the record to save the annotation on it.
    fn counted_files(&self, worktree: &Worktree) -> Result<Counted, Self::Error>;

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
