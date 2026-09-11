//! Getting a finished Job's work back to the branch it was cut from: the base
//! it merges into, catching up to it, the push, and the pull request.
//!
//! # A separate trait, so [`Vcs`](crate::Vcs) keeps having no push
//!
//! `Vcs` is what creates a Job's worktree and commits into it, and its whole
//! shape is that a holder of it cannot publish anything. Adding a push there
//! would put the capability one method away from every caller that only wanted
//! a worktree. Delivery is held by the one caller that is allowed to reach a
//! remote, and it is a different trait so that the two cannot be confused.
//!
//! # The ordinary outcomes are variants, not errors
//!
//! A repository with no remote, and a base branch that moved and now
//! conflicts, are both ordinary. Neither is a failed Job — the Checks passed
//! either way — so each is a variant of what the call *returned* rather than
//! something a caller has to catch. What is left is a tool that would not run,
//! which is [`NotDelivered`] and has one shape.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::{Remark, UnderReview, Worktree};

/// The branch a Job's work merges into, and where the name came from.
///
/// **Both halves matter to a person reading a line about it.** A base that was
/// declared is the repository stating its own answer; an inferred one is
/// Armada's best reading of a repository that did not say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Base {
    /// Named by `base:` in the Manifest.
    Declared(String),
    /// Nothing named one, so this is what the repository looks like it means:
    /// what a clone recorded as the remote's default, or the conventional name.
    Inferred(String),
}

impl Base {
    /// The branch name, which is all git needs.
    pub fn name(&self) -> &str {
        match self {
            Base::Declared(name) | Base::Inferred(name) => name,
        }
    }

    /// Whether the repository said so itself.
    pub fn was_declared(&self) -> bool {
        matches!(self, Base::Declared(_))
    }
}

/// Where a Job's branch stands against its base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// The base holds nothing the branch does not already have. **Nothing
    /// follows from this**, and a caller that announced it would be announcing
    /// a no-op.
    UpToDate,
    /// `commits` landed on the base that the branch has not got.
    Behind { commits: usize },
}

/// Where the local base branch stands against the one the forge has.
///
/// **A different question from [`Standing`], and the one nothing was asking.**
/// `Standing` compares the Job's branch with the base *on this machine*, which
/// is what a rebase needs. A pull request is opened against the base *on the
/// remote*, and nothing compared those two — so a base carrying commits the
/// remote has not got produced a pull request that carried them as well, under
/// a Job that never touched the files in them.
///
/// **Behind is carried alongside ahead** even though only ahead makes a pull
/// request wrong. They are one `graph_ahead_behind` call, and a base behind its
/// remote is a rebase that used a stale reading — worth saying in the same
/// breath rather than discovered separately later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseOnTheRemote {
    /// Nothing to say: they hold the same commits, or the base tracks no
    /// remote branch at all and there is no second reading to compare with.
    Agreed,
    /// They hold different commits. `remote` is the tracking branch's own name
    /// — `origin/main` — because that is what a person types to look at it.
    Apart {
        remote: String,
        /// Commits the local base has that the remote's has not got. **These
        /// are the ones a pull request would carry.**
        ahead: usize,
        /// Commits the remote's base has that the local one has not got.
        behind: usize,
    },
}

/// What catching up to the base came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BroughtUpToDate {
    /// The branch is on the base and there is nothing to resolve.
    Clean { base: String, commits: usize },
    /// The branch is on the base, and the work it carried across landed with
    /// conflicts still in the files. **Work rather than a failure** — resolving
    /// one needs judgement about the code.
    Conflicted { base: String, files: Vec<String> },
    /// Replaying the branch's own commits conflicted, so it was **put back
    /// exactly as it was**. Nothing moved and nothing is half-done.
    PutBack { base: String, files: Vec<String> },
}

/// What became of the push.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pushed {
    /// On the remote, under the branch's own name.
    ToTheRemote { remote: String, branch: String },
    /// **Ordinary, not a failure.** The repository has no remote, so the branch
    /// is the work and a person merges it where it is.
    NoRemote,
}

/// What became of the pull request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opened {
    /// It is open, and the address is what a person clicks.
    PullRequest { url: String },
    /// One is already open for this branch. A redispatched Job pushing again
    /// finds its own.
    AlreadyOpen { url: String },
    /// Nothing reached a remote, so there is nothing to open one from.
    NothingPushed,
    /// Nothing on this machine can open one. **Not a failure of the Job** — the
    /// branch is pushed and a person opens it by hand.
    NoTool { why: String },
}

/// What became of the pull request after it was opened. **The question a
/// person actually has about finished work**, and the one the record could not
/// answer: a merge Fleet did not perform is only ever knowable by asking.
///
/// **Fleet may merge, and what stays a person's is the decision** —
/// [`Delivery::merge`] is the act a person pressed for. This is the other way a
/// pull request settles, and there is no third: either Armada did it and knows,
/// or somebody did it on the forge and Armada finds out on a sweep.
///
/// **Every failure is [`Unknown`](Landing::Unknown)**, which is the rule
/// [`Opened`] already keeps one call earlier: no tool, not signed in, no such
/// pull request and a forge that would not answer are one absence here, because
/// nothing follows differently from any of them. A caller records nothing and
/// asks again later.
///
/// **`Unknown` is not `Open`.** A merge that Armada could not read about must
/// not render as a pull request still waiting for somebody — that is the same
/// sentence as the one the record already got wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Landing {
    /// Somebody merged it. **The end of the asking** — this never changes back,
    /// so a caller that records it never asks again.
    Merged { url: String },
    /// It is open and nobody has merged it yet — and, because that is the only
    /// state in which the question means anything, what the forge is comparing
    /// it against. A merged pull request's render is nobody's problem, and this
    /// is where that is said in the type rather than in a comment.
    Open { url: String, rendering: Rendering },
    /// It was closed and never merged. Also terminal, and a different sentence:
    /// the work was published and turned down.
    ClosedUnmerged { url: String },
    /// Nothing on this machine could say.
    Unknown,
}

impl Landing {
    /// Whether this is an answer worth writing down. **`Open` is not** — it is
    /// the state a pull request is in from the moment it exists, so recording
    /// it would store the absence of news.
    pub fn is_settled(&self) -> bool {
        matches!(
            self,
            Landing::Merged { .. } | Landing::ClosedUnmerged { .. }
        )
    }
}

/// Everything one ask of the forge answers about a pull request that was
/// opened. **One call and six answers.**
///
/// `#427` asked for the base question to ride the merge question rather than
/// sweep on its own, because `gh pr view` answers both in the same breath and
/// building them apart would be two processes where one would do. So this is
/// what [`Delivery::landed`] returns, and what it carries has grown from the
/// two things a person wants to know about work they cannot see — did it land,
/// and what does it land into — to what a person reads about it before it has:
/// its number, its title, and whether the forge can merge it as it stands.
/// Riding the same call is what `#427`'s argument already was; asking a second
/// time for a value this one already has would be the process this call exists
/// to spend once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhatBecameOfIt {
    /// Whether anybody merged it.
    pub landing: Landing,
    /// The branch it merges into, as the forge names it — not as this machine
    /// infers it, because the forge is the one rendering the comparison.
    ///
    /// **`None` is a forge that did not answer**, which is exactly
    /// [`Landing::Unknown`]'s case and never any other.
    pub base: Option<String>,
    /// The forge's own number for it. **`None` is `Unknown`'s case again** —
    /// a forge that answered anything at all named a number, so this is never
    /// absent beside a `landing` other than [`Landing::Unknown`].
    pub number: Option<u64>,
    /// Its title, as the forge holds it right now.
    ///
    /// **Not the title Armada opened it with.** `Review::title` is what
    /// [`Delivery::open_for_review`] sent; a person may have edited the pull
    /// request's own title since, and this is a later read of the same forge
    /// answering about the same address. `None` on [`Landing::Unknown`], for
    /// [`number`](WhatBecameOfIt::number)'s reason.
    pub title: Option<String>,
    /// Whether the forge can merge it into its base as it stands.
    ///
    /// **A third answer, and not a guess at one of the other two.** A forge
    /// still computing the answer says so rather than picking one, and
    /// [`Mergeable::Unreadable`] is that word carried rather than collapsed
    /// into a conflict nobody has found yet.
    pub mergeable: Mergeable,
}

impl WhatBecameOfIt {
    /// Nothing on this machine could say anything at all. **The one absence**,
    /// for [`Landing::Unknown`]'s reason: no tool, not signed in, no such pull
    /// request and a forge that would not answer are one silence here, and a
    /// caller records nothing and asks again later.
    pub fn unknown() -> WhatBecameOfIt {
        WhatBecameOfIt {
            landing: Landing::Unknown,
            base: None,
            number: None,
            title: None,
            mergeable: Mergeable::Unreadable,
        }
    }
}

/// Whether the forge can merge a pull request into its base as it stands.
///
/// **Not a `bool`.** A forge that has not finished computing the answer says
/// so, and folding that third word into either `Yes` or `No` would be a guess
/// this machine has no grounds for — the same argument [`crate::WhatPeopleSaid`]
/// and [`crate::WhatTheForgeRan`] already make for their own `Unreadable`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mergeable {
    /// The forge would merge it, unassisted, right now.
    Yes,
    /// The branch and its base disagree, in the forge's own reading —
    /// [`NotMerged::Conflicted`]'s reading, taken before anybody pressed.
    No,
    /// The forge has not said, or did not answer at all.
    Unreadable,
}

/// Whether the forge is comparing a pull request against the commit its branch
/// actually sits on.
///
/// A forge pins the base at the commit the pull request was opened from and
/// does not move it when the base branch advances underneath. So a pull request
/// that was right when it was opened renders other people's commits as this
/// Job's work once anything else merges — `#427` read 65 files beside the 28 its
/// Job wrote, and only a close and reopen moved it.
///
/// **This is not the defect a pull request's body already warns about.** A base
/// that was ahead of its remote *at open time* is a Job carrying commits it did
/// not write, and the remedy there is a push — [`BaseOnTheRemote`]. This is a
/// base that moved *after*, and no sentence in the body changes what the forge
/// renders beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rendering {
    /// The commit the forge pinned is the one the branch was written on top of,
    /// so the file list beside it is this Job's work and nothing else.
    AsWritten,
    /// The pinned commit has been superseded, so what is rendered beside the
    /// pull request is somebody else's work as well as this Job's.
    FromASupersededBase {
        /// What the forge is comparing against.
        pinned: String,
        /// What it should be comparing against — where the branch and the base
        /// actually part company.
        written_on: String,
    },
    /// Nothing on this machine could say which of the two it is.
    ///
    /// **Not stale**, and the distinction is the whole reason this is three
    /// variants rather than a `bool`: a pull request nobody can check is left
    /// alone rather than nudged, because the nudge closes and reopens it.
    Unreadable,
}

/// What came of asking the forge to merge a pull request.
///
/// **Two ways it is merged and only one of them is this call's doing.** A
/// person who pressed the button on a pull request somebody else had already
/// merged is not looking at a failure — the work is where the press was trying
/// to put it — and every caller does the same thing afterwards either way.
///
/// **It carries nothing.** What it merged into, what the base is on now and
/// whether the forge agrees are read with [`Delivery::landed`], which is the
/// same question a sweep asks about a pull request nobody pressed anything
/// for. A merge that answered them here would be a second reading of one fact,
/// and the two would come to disagree about a Job that was both noticed and
/// pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Merged {
    /// The forge took it.
    Taken,
    /// Somebody had already merged it. **Not a refusal.**
    AlreadyMerged,
}

/// Why the forge would not merge, in the kinds a person does something
/// different about.
///
/// **Not one shape with a sentence in it**, which is the arrangement
/// [`NotDelivered`] has and the reason it has it: there, no caller matches on
/// what went wrong. Here every kind sends a person somewhere else — an
/// administrator, the branch, the failing check — and a button that answered
/// all of them with one line would be a button that failed silently in five
/// different ways.
///
/// Every variant carries what the forge said, verbatim, because the kind is
/// what to do next and the sentence is why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotMerged {
    /// The base branch is protected, and merging it needs something this
    /// operator cannot supply from a command line — a review, an
    /// administrator, a rule.
    Protected { said: String },
    /// The branch and its base disagree. Nothing merges until somebody
    /// resolves it, and resolving one needs judgement about the code.
    Conflicted { said: String },
    /// A check the forge requires has not passed. **Not one of Armada's** —
    /// those already passed, or the Job would not be at a gate.
    ChecksNotPassed { said: String },
    /// It is closed, already gone, or there is no such pull request. The
    /// record names an address the forge will not act on.
    NotOpen { said: String },
    /// Nothing on this machine could ask: no tool, or nobody signed in.
    NoTool { said: String },
    /// The forge refused and said something this vocabulary has no name for.
    ///
    /// **Never folded into the five above.** A guess about which kind a
    /// sentence is would send a person to fix the wrong thing, and the honest
    /// answer is the sentence itself.
    Refused { said: String },
}

impl NotMerged {
    /// A sentence for a person, built beside the value rather than by whichever
    /// crate caught it — the shape [`NotDelivered::said`] takes.
    pub fn said(&self) -> String {
        let (why, said) = match self {
            NotMerged::Protected { said } => ("the base branch is protected", said),
            NotMerged::Conflicted { said } => ("the branch conflicts with its base", said),
            NotMerged::ChecksNotPassed { said } => {
                ("a check the forge requires has not passed", said)
            }
            NotMerged::NotOpen { said } => ("the pull request is not open", said),
            NotMerged::NoTool { said } => ("nothing on this machine could ask the forge", said),
            NotMerged::Refused { said } => ("the forge refused", said),
        };
        let mut out = String::from("the merge did not happen — ");
        out.push_str(why);
        out.push_str(": ");
        out.push_str(said);
        out
    }

    /// The kind, as one word, for a wire code and a log field.
    ///
    /// **Spelled once**, so the code a client dispatches on and the field a
    /// person reads in the log cannot come to name the same refusal two ways.
    pub fn kind(&self) -> &'static str {
        match self {
            NotMerged::Protected { .. } => "branch_protected",
            NotMerged::Conflicted { .. } => "conflicted",
            NotMerged::ChecksNotPassed { .. } => "checks_not_passed",
            NotMerged::NotOpen { .. } => "not_open",
            NotMerged::NoTool { .. } => "no_tool",
            NotMerged::Refused { .. } => "refused",
        }
    }
}

/// What came of bringing a pull request's branch up to a base that moved,
/// **in place of** closing and reopening it — `#663`.
///
/// **Every conflict leaves the branch exactly as it was.** Nothing is reading
/// the result the way a Drone reads [`BroughtUpToDate::Conflicted`]: the Job
/// is already done, so a conflict here is answered by restoring the branch
/// untouched and telling a person, never by leaving markers in a checkout
/// nobody is about to open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeptCurrent {
    /// The branch was rebased onto `onto` — the base's tip at the moment of
    /// the attempt — and pushed with `--force-with-lease`.
    Rebased { onto: String, commits: usize },
    /// The rebase conflicted, so the branch was left exactly as it was.
    /// `onto` is the base's tip this was attempted against, carried so a
    /// caller can tell "already tried this base and it conflicted" from "the
    /// base moved again" without retrying on every sweep.
    Conflicted { onto: String, files: Vec<String> },
    /// The branch this pull request derived is gone — nothing to rebase, and
    /// nothing this call can do about it.
    NoBranch,
    /// A tool would not run.
    NotDelivered(NotDelivered),
}

/// Where the repository every worktree is cut from stands, after a merge was
/// noticed.
///
/// **Fast-forward or nothing.** This runs in a repository a person is working
/// in, so every reading that is not "clean, on the branch that merged, and one
/// fast-forward behind it" is [`LeftAlone`](RepositoryStanding::LeftAlone) with
/// the reason said out loud.
///
/// **Two of the three name a commit and the third cannot**, which is what stops
/// `#474` proving a tree nobody committed. See
/// [`caught_up_to`](RepositoryStanding::caught_up_to).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepositoryStanding {
    /// It moved, by how much, and onto what. `head` is the commit the base is
    /// on **now** and not the merge commit the forge named, so a merge that
    /// arrived behind two others is proved once, at the tip.
    MovedOn {
        base: String,
        commits: usize,
        head: String,
    },
    /// It already had what merged, and what it is on. Ordinary, and the answer
    /// whenever a person pulled before Armada got to it — which is not a person
    /// having run anything, so it still names a commit.
    AlreadyHadIt { base: String, head: String },
    /// Nothing was touched, and why — a person's uncommitted work, a checkout
    /// that is not on the base, no remote, or a history that would not
    /// fast-forward. **No commit, and that is the point.**
    LeftAlone { why: String },
}

impl RepositoryStanding {
    /// The commit a run may be made against, where this standing left one.
    ///
    /// **`None` is the whole of the refusal**, and it is a shape rather than a
    /// rule: there is no other way to reach a run, so declining on a dirty
    /// worktree is not a check somebody remembered to write.
    pub fn caught_up_to(&self) -> Option<&str> {
        match self {
            RepositoryStanding::MovedOn { head, .. }
            | RepositoryStanding::AlreadyHadIt { head, .. } => Some(head),
            RepositoryStanding::LeftAlone { .. } => None,
        }
    }
}

/// A pull request's contents, assembled before anything is opened.
///
/// **Two owned strings and no builder.** What goes in them is assembled by the
/// caller from its own record; this type exists so that what reaches the forge
/// is a value that was built somewhere a test can read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Review {
    title: String,
    body: String,
}

impl Review {
    pub fn assembled(title: impl Into<String>, body: impl Into<String>) -> Review {
        Review {
            title: title.into(),
            body: body.into(),
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

/// Why a step of delivery did not happen.
///
/// **One shape rather than an associated type.** The outcomes a caller acts on
/// differently — no remote, a conflict, no tool — are variants of the values
/// above, so what is left over is a command that would not run and the sentence
/// it gave, which no caller matches on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotDelivered {
    /// What was being attempted, as a noun a sentence can be built around.
    pub doing: &'static str,
    /// What the tool said, verbatim.
    pub said: String,
}

impl NotDelivered {
    pub fn of(doing: &'static str, said: impl Into<String>) -> NotDelivered {
        NotDelivered {
            doing,
            said: said.into(),
        }
    }

    /// A sentence for a person, built beside the value rather than by whichever
    /// crate caught it — the shape
    /// [`WorktreeSpecRefused`](crate::WorktreeSpecRefused) takes.
    pub fn said(&self) -> String {
        let mut out = String::from(self.doing);
        out.push_str(" did not happen: ");
        out.push_str(&self.said);
        out
    }
}

/// Publishing a Job's work: what it merges into, catching up to it, the push,
/// and the pull request.
///
/// # Every method is the operator's credentials, and none of them is a Drone's
///
/// Fleet runs in the operator's own environment, so a push authenticates the
/// way the operator's own shell would. A Drone's five variables carry none of
/// that and a Drone holds no type that implements this.
pub trait Delivery {
    /// The branch this Job's work merges into.
    ///
    /// `declared` is what the Manifest said, or `None`. **Inference is the
    /// fallback, never the override** — a declared base that the repository
    /// does not have is refused rather than quietly replaced with a guess.
    ///
    /// `Ok(None)` is a repository that names no base and no candidate is there,
    /// which is ordinary in a repository that has never had a default branch.
    fn base(
        &self,
        worktree: &Worktree,
        declared: Option<&str>,
    ) -> Result<Option<Base>, NotDelivered>;

    /// Where the branch stands against the base. **Asked before anything is
    /// moved**, so that a branch which is not behind costs one comparison.
    fn standing(&self, worktree: &Worktree, base: &Base) -> Result<Standing, NotDelivered>;

    /// Where the base branch stands against the one the forge would merge into.
    ///
    /// **Asked at the pull request and nowhere else.** It changes nothing about
    /// what is rebased — the base on this machine is the branch a person merges
    /// into, which is why `Nothing here fetches` is the adapter's own rule —
    /// and it is read so that the pull request can say what it is carrying.
    fn base_on_the_remote(
        &self,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<BaseOnTheRemote, NotDelivered>;

    /// Put the branch on top of the base, carrying uncommitted work across.
    ///
    /// **Uncommitted work is never destroyed.** Whatever the worktree is
    /// holding comes across with it, and where it cannot, the branch is put
    /// back where it was.
    fn bring_up_to_date(
        &self,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<BroughtUpToDate, NotDelivered>;

    /// Put the branch on the remote, under its own name.
    fn push(&self, worktree: &Worktree) -> Result<Pushed, NotDelivered>;

    /// The same push, `--force-with-lease`. **The one call this trait makes
    /// that may overwrite what is on the remote**, and the only one that may:
    /// every other write here either creates something that was not there —
    /// the ordinary [`push`](Delivery::push), a pull request — or is refused
    /// where history would move under it. This exists for the one case where
    /// moving it is the point — [`bring_up_to_date`](Delivery::bring_up_to_date)
    /// rewrote the branch, and the remote's own copy is what it is rewriting.
    /// `#663`.
    fn push_forcing(&self, worktree: &Worktree) -> Result<Pushed, NotDelivered>;

    /// Open a pull request from the branch into the base.
    fn open_for_review(
        &self,
        worktree: &Worktree,
        base: &Base,
        review: &Review,
    ) -> Result<Opened, NotDelivered>;

    /// What became of a pull request that was opened.
    ///
    /// **No `Result`, unlike every method above it.** There is nothing a caller
    /// could do differently about a forge that would not answer than about a
    /// pull request it cannot find: both mean ask again later, so both are
    /// [`Landing::Unknown`] and there is no error to handle. The same reasoning
    /// `already_open` has kept since delivery shipped, made into a type.
    ///
    /// **Asked by address and not by branch.** A branch is what
    /// [`open_for_review`](Delivery::open_for_review) had, and a merged branch
    /// is usually deleted — so the address the record kept is the only handle
    /// that still resolves once the answer is the interesting one.
    ///
    /// `in_repo` is an absolute path to run from, and **the one method here
    /// that does not take a [`Worktree`]**: a Job's own worktree is reclaimed
    /// long before anybody merges its work, so the caller passes the repository
    /// every worktree was cut from, which is not one.
    fn landed(&self, in_repo: &str, pull_request: &str) -> WhatBecameOfIt;

    /// What the people and the machines on the forge say about a pull request
    /// nobody has merged yet.
    ///
    /// **Asked only where [`landed`](Delivery::landed) answered
    /// [`Landing::Open`].** A merged pull request has left the sweep's rotation
    /// and a closed one is terminal, so neither has an answer here anybody
    /// would act on — the rule [`Rendering`] keeps by riding on that variant.
    ///
    /// **A second call and not four more fields on `landed`**, which is the
    /// reverse of `#427`'s argument for folding [`Rendering`] in, because the
    /// shape is the reverse: those were four scalars on one line beside the
    /// state, and these are three lists, one carrying whatever anybody typed
    /// into a comment box. The callers differ too — a merge pressed from a
    /// screen reads `landed` and has no use for any of this.
    ///
    /// **No `Result`**, for [`landed`](Delivery::landed)'s reason exactly: no
    /// tool, not signed in and a forge that would not answer are one silence,
    /// [`UnderReview::unreadable`].
    ///
    /// **Nothing here is a verdict and nothing here passes a Check.** See
    /// [`UnderReview`], whose module carries that argument in full.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](Delivery::landed)'s reason.
    fn under_review(&self, in_repo: &str, pull_request: &str) -> UnderReview;

    /// The comments left on individual lines of the diff — the ones
    /// [`under_review`](Delivery::under_review) does not fetch, because they
    /// are a second query per review and the sweep has one call to spend.
    ///
    /// **Asked on demand and never from the sweep.** The two callers are a
    /// person opening a Job's comments and a press taking some of them up —
    /// both already cost a process for [`under_review`](Delivery::under_review),
    /// so this is the one place fetching inline comments is affordable.
    ///
    /// **Empty rather than a `Result`.** A forge that would not answer this
    /// second question leaves a person with the comments
    /// [`under_review`](Delivery::under_review) already found and none of
    /// their code — worse than not asking would be refusing the whole read
    /// over a query that is strictly additional to it.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](Delivery::landed)'s reason.
    fn inline_remarks(&self, in_repo: &str, pull_request: &str) -> Vec<Remark>;

    /// Merge a pull request Armada opened.
    ///
    /// **The one method on this trait that writes into a repository nobody on
    /// this machine holds.** Every other act Fleet takes is confined to a
    /// worktree Fleet made; a push puts a branch somewhere nobody merges from,
    /// and this changes what everybody else builds on. A caller logs it before
    /// it happens.
    ///
    /// **Nothing here decides whether it should be merged.** That is a
    /// person's, at a human gate, and this is the act they pressed for.
    ///
    /// **What it merged into is not returned**, for [`Merged`]'s reason: the
    /// caller reads it with [`landed`](Delivery::landed), which is the same
    /// question a sweep asks about a pull request nobody pressed anything for.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](Delivery::landed)'s reason — a Job's own worktree is
    /// reclaimed long before anybody merges its work.
    fn merge(&self, in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged>;

    /// Rebase a Job's branch onto a base that has moved, and push the result —
    /// in place of closing and reopening the pull request. `#663`.
    ///
    /// **Not `#427`'s close-and-reopen.** That measured the alternatives for
    /// *rendering* a pull request afresh without changing what it carries; this
    /// changes what it carries, which is the fix for the finding that motivated
    /// it in the first place — a branch behind its base with conflicts, which
    /// only a rebase resolves.
    ///
    /// **Works whether or not a Job's own worktree is still there.** A finished
    /// Job's worktree is very often reclaimed before its pull request is
    /// merged, so this derives one from `handle` — the same worktree if it is
    /// still on disk, or a scratch checkout onto the same branch, attached and
    /// detached inside this one call, where it is not.
    ///
    /// **Called only for [`Rendering::FromASupersededBase`]**, and at most once
    /// per base — see `fleet::currency`, which holds that guard durably rather
    /// than in memory, because losing it on a restart is `#663`'s own finding.
    ///
    /// `in_repo` is the repository every worktree was cut from, `handle` is the
    /// Job's own — which is what a worktree and a branch are both derived from
    /// — and `base` is the branch the forge named as what this merges into.
    fn kept_current(&self, in_repo: &str, handle: &str, base: &str) -> KeptCurrent;

    /// The local commit a base branch is on, read without a worktree.
    ///
    /// **The cheap half of [`kept_current`](Delivery::kept_current)**, asked
    /// first so that a base which has not moved past what was already tried
    /// costs one process rather than a scratch checkout and a rebase. `None`
    /// is a base this could not read — no such branch, an unreadable
    /// repository — and a caller answers that the same as never having seen
    /// it, never as agreement.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](Delivery::landed)'s reason.
    fn base_tip(&self, in_repo: &str, base: &str) -> Option<String>;

    /// Bring the repository every worktree is cut from up to the branch that
    /// just merged.
    ///
    /// **What merged is now what everything else builds on** — `#337` — so the
    /// tree the next Job is cut from should have it. Nothing here is a gate and
    /// nothing fails a Job: every refusal is
    /// [`LeftAlone`](RepositoryStanding::LeftAlone).
    ///
    /// **Never anything but a fast-forward, and never over a person's work.**
    /// This is the one method on this trait that writes into the repository a
    /// person is standing in rather than a Job's worktree, so it declines on
    /// anything it cannot do without merging, rebasing or discarding.
    fn caught_the_repository_up(&self, in_repo: &str, base: &str) -> RepositoryStanding;
}

/// A line for a person about where the base came from. Built here so the two
/// callers that report it cannot word it two ways.
pub fn how_the_base_was_found(base: &Base) -> String {
    match base.was_declared() {
        true => "`base:` in armada.yml".to_string(),
        false => "inferred — nothing declares one".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declared_base_says_so_and_an_inferred_one_does_not() {
        assert!(Base::Declared("release".into()).was_declared());
        assert!(!Base::Inferred("main".into()).was_declared());
        assert_eq!(Base::Inferred("main".into()).name(), "main");
    }

    #[test]
    fn a_refusal_names_what_was_being_attempted() {
        let refused = NotDelivered::of("the push", "remote rejected");
        assert!(refused.said().starts_with("the push did not happen"));
        assert!(refused.said().ends_with("remote rejected"));
    }

    #[test]
    fn a_review_carries_both_halves_and_offers_no_way_to_change_them() {
        let review = Review::assembled("fix the reader", "## What was checked");
        assert_eq!(review.title(), "fix the reader");
        assert_eq!(review.body(), "## What was checked");
    }
}
