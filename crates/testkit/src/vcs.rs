//! A `Vcs` that touches no disk.
//!
//! # Why this exists rather than a temporary repository per test
//!
//! Because a suite that shells out to git for every case is a suite people stop
//! running, and because the acceptance test is hermetic by definition — no
//! process spawned, no repository touched, no network opened. Fleet's own tests
//! ask *did a worktree get created before the Drone, and what did Fleet do when
//! one could not be*; neither question needs git to answer it.
//!
//! The cases that genuinely need git's opinion — an existing branch, an
//! administrative record that outlives its directory — are tested against a
//! real repository in `adapters`, where they are five tests rather than every
//! test.
//!
//! # What it is faithful about
//!
//! Every refusal a caller has to handle, and the derivation. Paths and
//! branch names come from the same [`WorktreeSpec`] the real implementation
//! uses, so a test asserting on a path is asserting on the derivation that
//! ships rather than on a second copy of it — the second-vocabulary defect.
//!
//! It is **not** faithful about the filesystem: nothing is created, so a test
//! that wants to read a file out of a worktree wants the real one.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::sync::Mutex;

use adapter_traits::{
    Base, BaseCheckout, BaseOnTheRemote, BaseSpec, BroughtUpToDate, Change, CommitTime, Committed,
    Delivery, Landing, Merged, NotDelivered, NotMerged, Opened, Pushed, Renewed, Replied,
    RepositoryStanding, Review, Standing, UnderReview, Vcs, WhatBecameOfIt, Worktree, WorktreeSpec,
};

use crate::work_product::Holding;

/// Why the fake refused.
///
/// One variant per split the real error draws: a name already taken, the
/// machine not cooperating, and a commit git would not make. A caller that
/// handles them handles the real implementation's whole surface as far as its
/// own logic is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeVcsError {
    /// A branch of that name is already there and was refused, never reused.
    BranchExists { branch: String },
    /// A scripted failure standing in for a disk, a permission or a repository
    /// that would not answer.
    Refused { standing_in_for: &'static str },
    /// A scripted failure of the commit, which is its own case: it happens
    /// after a Job's Checks have passed, and the caller must not lose the work
    /// over it.
    NotCommitted { standing_in_for: &'static str },
    /// No ref of that name, which is what the real one raises when `base:`
    /// names a branch the repository does not have.
    NoSuchRef { r#ref: String },
}

impl fmt::Display for FakeVcsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FakeVcsError::BranchExists { branch } => {
                write!(f, "the branch `{branch}` is already there")
            }
            FakeVcsError::Refused { standing_in_for } => {
                write!(f, "refused, standing in for {standing_in_for}")
            }
            FakeVcsError::NotCommitted { standing_in_for } => {
                write!(f, "not committed, standing in for {standing_in_for}")
            }
            FakeVcsError::NoSuchRef { r#ref } => {
                write!(f, "there is no ref `{name}`", name = r#ref)
            }
        }
    }
}

impl Error for FakeVcsError {}

/// Version control that remembers what it was asked for and creates nothing.
///
/// **`Mutex` rather than `RefCell`, and that is not a style choice.** A Fleet
/// implements `api::Daemon`, which is `Send + Sync`, so every seam it holds has
/// to be `Sync` too — and a fake that is not cannot stand in for the real
/// adapter at the one boundary the acceptance test drives. The three fakes in
/// this crate each carry the same note.
#[derive(Debug, Default)]
pub struct FakeVcs {
    branches: Mutex<BTreeSet<String>>,
    created: Mutex<Vec<Worktree>>,
    refuse_next: Mutex<Option<&'static str>>,
    committed: Mutex<Vec<FakeCommit>>,
    /// What the next commit answers. Both halves are scripted rather than
    /// inferred: a fake that guessed whether a worktree it never created has
    /// anything in it would be asserting against its own guess.
    commits: Mutex<Willing>,
    /// What delivery answers. Scripted for the same reason: there is no
    /// repository here to be behind anything, and no remote to push to.
    delivery: Mutex<Delivering>,
    delivered: Mutex<Vec<Delivered>>,
    /// What the rebase leaves in the worktree, and where. Absent for every case
    /// that does not care — see [`FakeVcs::writing_into`].
    rebase_writes: Mutex<Option<(Holding, Vec<String>)>>,
    /// Whether the base branch and the remote's agree. **Its own field rather
    /// than a sixth on [`Delivering`]**, which is that type's own rule: every
    /// field there is written out at every call site, and a repository whose
    /// base is level with its remote is what all but one of them mean. See
    /// [`FakeVcs::base_apart_from_the_remote`]. Absent is `Agreed`, which is
    /// what keeps this fake's `Default` and the domain type's lack of one both
    /// intact.
    base_on_the_remote: Mutex<Option<BaseOnTheRemote>>,
    /// What the forge answers a merge with. **Its own field rather than a tenth
    /// on [`Delivering`]**, for [`base_on_the_remote`](FakeVcs::base_on_the_remote)'s
    /// reason and with a second: a merge only ever happens after the Fleet
    /// holding this fake exists, so it is scripted through `&self` and a
    /// consuming builder could not express the case at all.
    merging: Mutex<Merging>,
    /// What the forge answers a reply with. **Its own field beside
    /// [`merging`](FakeVcs::merging)**, for that field's reason: a reply is only
    /// ever written after the Fleet holding this fake exists.
    replying: Mutex<Replying>,
    /// What commit each ref is at. **Scripted**, for
    /// [`commits`](FakeVcs::commits)' reason: there is no repository here to
    /// have a history, and a fake that invented one id per name would make
    /// *the base moved* untestable, which is the case the base checkout exists
    /// to get right.
    refs: Mutex<BTreeMap<String, String>>,
    /// Every base checkout this fake has been asked for and not dropped, keyed
    /// by commit, with whether it has been marked prepared.
    ///
    /// **A map rather than a list**, so asking twice for one commit answers the
    /// same checkout — which is the property `Vcs::base_checkout` promises and
    /// the one a caller relying on the sharing has to be able to assert.
    bases: Mutex<BTreeMap<String, bool>>,
    /// Every base checkout this fake has been asked to drop, in order.
    dropped_bases: Mutex<Vec<String>>,
}

/// What this fake's forge does when asked to merge.
///
/// **A refusal is a value a test writes out**, not a string it matches on: the
/// kinds are what a caller acts on differently, and a fake that answered them
/// all with one sentence could not exercise that at all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Merging {
    /// The forge takes it. **The default**, because a test about the press is
    /// about what follows a merge.
    #[default]
    Takes,
    /// Somebody had already merged it.
    AlreadyMerged,
    /// It would not, and which kind of would-not it was.
    Refuses(NotMerged),
}

/// What this fake's forge does when it is asked to write a comment.
///
/// **A refusal is a sentence and not a kind**, unlike [`Merging`]'s, because
/// `Replied` has one refusal: nothing a caller does turns on why the comment
/// did not post, and the sentence is what reaches the Job's log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Replying {
    /// The forge takes it. **The default**, because a test about the press is
    /// about what the reply said.
    #[default]
    Takes,
    /// It would not, and this is what it said.
    Refuses(String),
}

/// One thing this fake was asked to do to a Job's branch, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivered {
    /// The branch was put on top of `base`.
    BroughtUpToDate { branch: String, base: String },
    /// The branch was pushed.
    Pushed { branch: String },
    /// A pull request was opened, carrying this.
    OpenedForReview { base: String, review: Review },
    /// The forge was asked what became of the branch's pull request. **Counted
    /// as well as answered**, because the whole design of the asking is how
    /// rarely it happens — a test that could not see the calls could not tell a
    /// sweep that asks once from one that asks every turn.
    AskedWhatBecameOfIt { pull_request: String },
    /// The forge was asked who has looked at an open pull request, what ran
    /// against it and what anybody wrote on it. **Counted for
    /// [`AskedWhatBecameOfIt`](Delivered::AskedWhatBecameOfIt)'s reason**: this
    /// rides the same rotation, and a test that could not see the calls could
    /// not tell a sweep that asks once from one that asks about a merged pull
    /// request it should have stopped asking about.
    AskedWhatIsUnderReview { pull_request: String },
    /// The forge was asked to compare a pull request afresh. **Counted for
    /// [`AskedWhatBecameOfIt`](Delivered::AskedWhatBecameOfIt)'s reason and
    /// then some**: this one closes and reopens a person's pull request, so a
    /// test that could not see it could not tell once from every sweep.
    AskedToRenderAfresh { pull_request: String },
    /// The repository every worktree is cut from was asked to catch up.
    CaughtTheRepositoryUp { base: String },
    /// The forge was asked to merge a pull request. **One of the two writes to
    /// a repository Fleet did not make**, so a test that could not see it could
    /// not tell a press that merged from one that only moved a Job.
    Merged { pull_request: String },
    /// One comment was written onto a pull request. **The second of those two
    /// writes, and it carries what was said** — the rule it is under is that
    /// there is exactly one of these per press, which is a claim about the
    /// count, and the words are what proves the reply named what it took up.
    Replied {
        pull_request: String,
        saying: String,
    },
}

/// What the fake's version control looks like from the delivery side.
///
/// Public fields and no `Default` for the reason `Fittings` has neither: a test
/// writes out every one of the four, so the repository it is describing is
/// visible at the call site rather than inherited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivering {
    /// What the repository says its base is, or `None` for one that names none.
    pub base: Option<Base>,
    /// How far behind the base the branch is.
    pub standing: Standing,
    /// What bringing it up to date comes to. `None` where it is not behind.
    pub rebase: Option<BroughtUpToDate>,
    pub push: Pushed,
    pub review: Opened,
    /// What the forge says became of the pull request afterwards. `Unknown` by
    /// default, which is the answer on a machine with no forge and the one a
    /// test gets unless it says a merge happened.
    pub landed: Landing,
    /// The branch the forge says the pull request merges into. `None` is a
    /// forge that answered nothing, which is what [`Landing::Unknown`] means.
    pub base_on_the_forge: Option<String>,
    /// What the forge says about the pull request while it is still open.
    /// Unreadable by default, which is the answer on a machine with no forge —
    /// and the one every case that is not about reviews should get, so that
    /// nothing reads an approval nobody scripted.
    pub under_review: UnderReview,
    /// What closing and reopening comes to. Renewed by default, because the
    /// case a test has to write out is the one where it was left closed.
    pub renewed: Renewed,
    /// What catching the repository up comes to.
    pub repository: RepositoryStanding,
}

impl Default for Delivering {
    /// A repository on `main`, up to date, with a remote and a forge — the
    /// shape a Job that goes the whole way runs against.
    fn default() -> Delivering {
        Delivering {
            base: Some(Base::Inferred(String::from("main"))),
            standing: Standing::UpToDate,
            rebase: None,
            push: Pushed::ToTheRemote {
                remote: String::from("origin"),
                branch: String::from("armada/a-job"),
            },
            review: Opened::PullRequest {
                url: String::from("https://forge.invalid/armada/pull/1"),
            },
            // Nobody has merged it. A default that said `Merged` would have
            // every existing test's Job land the moment anything asked.
            landed: Landing::Unknown,
            base_on_the_forge: Some(String::from("main")),
            under_review: UnderReview::unreadable(),
            renewed: Renewed::Renewed,
            repository: RepositoryStanding::AlreadyHadIt {
                base: String::from("main"),
                // A commit-shaped string, because `#474` keys a proof by it and
                // a fixture that handed back an empty one would key every
                // fake's proof the same way.
                head: String::from("0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f"),
            },
        }
    }
}

/// One commit this fake said it made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeCommit {
    pub branch: String,
    pub message: String,
    pub at: CommitTime,
}

/// What the fake does when asked to commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Willing {
    /// The ordinary case: a commit is made and recorded.
    #[default]
    Yes,
    /// The worktree held nothing new. A `facts_note` Job's shape.
    NothingChanged,
    /// git refused. The work is still there and the caller has to say so.
    No(&'static str),
}

impl FakeVcs {
    pub fn new() -> FakeVcs {
        FakeVcs::default()
    }

    /// Seed a branch somebody else already made, so a collision can be
    /// exercised without a repository.
    pub fn with_existing_branch(self, branch: impl Into<String>) -> FakeVcs {
        self.branches
            .lock()
            .expect("not poisoned")
            .insert(branch.into());
        self
    }

    /// Make the next creation fail as a machine would.
    ///
    /// The argument is what it stands in for, and it is a fixed string chosen
    /// at the call site rather than a message assembled from data — a fake's
    /// failure is a script, not a diagnosis.
    pub fn refuse_next(&self, standing_in_for: &'static str) {
        *self.refuse_next.lock().expect("not poisoned") = Some(standing_in_for);
    }

    /// Every worktree this fake said it made, in order. **Nothing removes an
    /// entry**, for the same reason nothing removes a worktree.
    pub fn created(&self) -> Vec<Worktree> {
        self.created.lock().expect("not poisoned").clone()
    }

    /// Put a ref at a commit, so a base can be resolved without a repository.
    ///
    /// Called a second time with a different id, this is a base branch that
    /// moved — which is the only way to reach the second base checkout, and so
    /// the only way to test that a moved base does not photograph the old one.
    pub fn with_ref_at(self, r#ref: impl Into<String>, commit: impl Into<String>) -> FakeVcs {
        self.refs
            .lock()
            .expect("not poisoned")
            .insert(r#ref.into(), commit.into());
        self
    }

    /// Move a ref after the Fleet holding this fake exists.
    pub fn move_ref_to(&self, r#ref: impl Into<String>, commit: impl Into<String>) {
        self.refs
            .lock()
            .expect("not poisoned")
            .insert(r#ref.into(), commit.into());
    }

    /// Every base checkout this fake is holding, by commit, with whether it has
    /// been marked prepared.
    pub fn bases(&self) -> BTreeMap<String, bool> {
        self.bases.lock().expect("not poisoned").clone()
    }

    /// The commits this fake has been asked to drop a base checkout for.
    pub fn dropped_bases(&self) -> Vec<String> {
        self.dropped_bases.lock().expect("not poisoned").clone()
    }

    /// Say a base checkout has finished `setup.requires`, the way writing the
    /// marker into a real one does.
    ///
    /// **On the fake rather than inferred from a `prepare` call**, because
    /// nothing here runs a command: preparation is Fleet's, and what this fake
    /// owes it is somewhere to record that it happened.
    pub fn base_is_prepared(&self, commit: &str) {
        self.bases
            .lock()
            .expect("not poisoned")
            .insert(commit.to_string(), true);
    }

    /// Make every commit answer `NothingToCommit`, as a Job that wrote no file
    /// would.
    pub fn with_nothing_to_commit(self) -> FakeVcs {
        *self.commits.lock().expect("not poisoned") = Willing::NothingChanged;
        self
    }

    /// Make every commit fail as git refusing one would.
    pub fn refusing_to_commit(self, standing_in_for: &'static str) -> FakeVcs {
        *self.commits.lock().expect("not poisoned") = Willing::No(standing_in_for);
        self
    }

    /// Every commit this fake said it made, in order.
    pub fn committed(&self) -> Vec<FakeCommit> {
        self.committed.lock().expect("not poisoned").clone()
    }

    /// Script what the repository looks like from the delivery side.
    pub fn delivering(self, delivering: Delivering) -> FakeVcs {
        *self.delivery.lock().expect("not poisoned") = delivering;
        self
    }

    /// Say that somebody has merged, or closed, the pull request — **after the
    /// Job that opened it has finished**, which is the only order the real
    /// thing happens in.
    ///
    /// `&self` and not `self`, unlike every other setter here: the fake is
    /// inside a Fleet by the time a person could merge anything, so a
    /// consuming builder could not express the case at all.
    pub fn now_landed(&self, landed: Landing) {
        self.delivery.lock().expect("not poisoned").landed = landed;
    }

    /// Say what the forge says about the open pull request: who has looked at
    /// it, what ran against it, what anybody wrote.
    ///
    /// `&self` for [`now_landed`](FakeVcs::now_landed)'s reason — nothing is
    /// reviewed until the Job that opened the pull request has finished, by
    /// which time the fake is inside a Fleet.
    pub fn now_under_review(&self, under_review: UnderReview) {
        self.delivery.lock().expect("not poisoned").under_review = under_review;
    }

    /// How many times the forge has been asked what is happening on an open
    /// pull request. **Counted apart from the merge question**, because the two
    /// ride one rotation and a test proving the sweep did not grow a second
    /// loop is counting these against those.
    pub fn times_asked_what_is_under_review(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::AskedWhatIsUnderReview { .. }))
    }

    /// Say what the forge does when it is asked to merge.
    ///
    /// `&self` for [`now_landed`](FakeVcs::now_landed)'s reason: nothing merges
    /// a pull request until the Job that opened it is at its gate, by which
    /// time the fake is inside a Fleet.
    pub fn merging(&self, merging: Merging) {
        *self.merging.lock().expect("not poisoned") = merging;
    }

    /// Say what the forge does when it is asked to write a comment.
    ///
    /// `&self` for [`merging`](FakeVcs::merging)'s reason.
    pub fn replying(&self, replying: Replying) {
        *self.replying.lock().expect("not poisoned") = replying;
    }

    /// Every comment this fake was asked to write, in order. **The words and
    /// not the count**, because the rule the caller is under is one reply per
    /// press saying what was taken up, and a count alone proves only the first
    /// half of it.
    pub fn replies(&self) -> Vec<String> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter_map(|it| match it {
                Delivered::Replied { saying, .. } => Some(saying.clone()),
                _ => None,
            })
            .collect()
    }

    /// How many times the forge has been asked to merge. **The write this fake
    /// exists to make visible**, so a test asserting that a refused press
    /// merged nothing is asserting on the whole run rather than on a status.
    pub fn times_asked_to_merge(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::Merged { .. }))
    }

    /// Say that closing and reopening the pull request will leave it closed.
    ///
    /// `&self` for [`now_landed`](FakeVcs::now_landed)'s reason, and the one
    /// outcome worth scripting: the renewal that works changes nothing a
    /// caller can see, and the one that fails is the case the guard exists for.
    pub fn unable_to_reopen(&self) {
        self.delivery.lock().expect("not poisoned").renewed = Renewed::LeftClosed {
            why: String::from("the forge would not reopen it"),
        };
    }

    /// Say what bringing the repository up to the merged branch comes to.
    pub fn repository_standing(&self, standing: RepositoryStanding) {
        self.delivery.lock().expect("not poisoned").repository = standing;
    }

    /// How many times the forge has been asked to compare a pull request
    /// afresh. **The one call here a person sees happen**, so a test that
    /// asserts once is asserting the whole of the rule.
    pub fn times_asked_to_render_afresh(&self) -> usize {
        self.counted(|it| matches!(it, Delivered::AskedToRenderAfresh { .. }))
    }

    /// Every branch the repository was asked to catch up to, in order.
    pub fn repository_caught_up_to(&self) -> Vec<String> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter_map(|it| match it {
                Delivered::CaughtTheRepositoryUp { base } => Some(base.clone()),
                _ => None,
            })
            .collect()
    }

    fn counted(&self, which: impl Fn(&Delivered) -> bool) -> usize {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter(|it| which(it))
            .count()
    }

    /// How many times the forge has been asked what became of a pull request.
    /// **The whole design of the asking is how rarely it happens**, so this is
    /// what a test about the rotation counts.
    pub fn times_asked_what_became_of_it(&self) -> usize {
        self.delivered
            .lock()
            .expect("not poisoned")
            .iter()
            .filter(|done| matches!(done, Delivered::AskedWhatBecameOfIt { .. }))
            .count()
    }

    /// A rebase that writes these files, into the worktree this
    /// [`Holding`](crate::Holding) is a handle on.
    ///
    /// **What a conflicted rebase does.** `git rebase --autostash` puts markers
    /// into the files it could not merge, and markers are content: a footprint
    /// taken after one reads differently from a footprint taken before, on the
    /// same paths. Nothing here says the rebase conflicted — the answer is
    /// [`Delivering::rebase`]'s to script, and this is what it left behind.
    ///
    /// The two fakes are otherwise independent, and this is the one place they
    /// are not. It is needed because a test cannot see *when* Fleet takes a
    /// step's baseline unless the rebase between the two readings moves
    /// something.
    /// Say that the base branch on this machine holds commits the remote's has
    /// not got, or the other way round.
    ///
    /// `Agreed` without this, because a base level with its remote is the
    /// ordinary case and a fake that defaulted to skew would put a caveat into
    /// every pull request every other test reads.
    pub fn base_apart_from_the_remote(self, remote: &str, ahead: usize, behind: usize) -> FakeVcs {
        *self.base_on_the_remote.lock().expect("not poisoned") = Some(BaseOnTheRemote::Apart {
            remote: remote.to_string(),
            ahead,
            behind,
        });
        self
    }

    pub fn writing_into(self, holding: Holding, files: &[&str]) -> FakeVcs {
        *self.rebase_writes.lock().expect("not poisoned") =
            Some((holding, files.iter().map(|path| path.to_string()).collect()));
        self
    }

    /// Everything this fake was asked to do to a branch, in order. **Nothing
    /// removes an entry**, so a test asserting that no push happened is
    /// asserting on the whole run rather than on the last call.
    pub fn delivered(&self) -> Vec<Delivered> {
        self.delivered.lock().expect("not poisoned").clone()
    }
}

impl Delivery for FakeVcs {
    fn base(
        &self,
        _worktree: &Worktree,
        declared: Option<&str>,
    ) -> Result<Option<Base>, NotDelivered> {
        // Declared beats scripted, so a test can assert the key is honoured
        // without describing a repository at all.
        if let Some(declared) = declared {
            return Ok(Some(Base::Declared(declared.to_string())));
        }
        Ok(self.delivery.lock().expect("not poisoned").base.clone())
    }

    fn standing(&self, _worktree: &Worktree, _base: &Base) -> Result<Standing, NotDelivered> {
        Ok(self.delivery.lock().expect("not poisoned").standing)
    }

    fn base_on_the_remote(
        &self,
        _worktree: &Worktree,
        _base: &Base,
    ) -> Result<BaseOnTheRemote, NotDelivered> {
        Ok(self
            .base_on_the_remote
            .lock()
            .expect("not poisoned")
            .clone()
            .unwrap_or(BaseOnTheRemote::Agreed))
    }

    fn bring_up_to_date(
        &self,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<BroughtUpToDate, NotDelivered> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::BroughtUpToDate {
                branch: worktree.branch().to_string(),
                base: base.name().to_string(),
            });
        // Before the answer is returned, because that is when git does it: the
        // worktree has already been written to by the time the caller reads
        // what the rebase came to.
        if let Some((holding, files)) = self.rebase_writes.lock().expect("not poisoned").as_ref() {
            holding.wrote(
                &files
                    .iter()
                    .map(|path| (path.as_str(), Change::Modified))
                    .collect::<Vec<(&str, Change)>>(),
            );
        }
        Ok(self
            .delivery
            .lock()
            .expect("not poisoned")
            .rebase
            .clone()
            .unwrap_or(BroughtUpToDate::Clean {
                base: base.name().to_string(),
                commits: 0,
            }))
    }

    fn push(&self, worktree: &Worktree) -> Result<Pushed, NotDelivered> {
        let pushed = self.delivery.lock().expect("not poisoned").push.clone();
        if pushed != Pushed::NoRemote {
            self.delivered
                .lock()
                .expect("not poisoned")
                .push(Delivered::Pushed {
                    branch: worktree.branch().to_string(),
                });
        }
        Ok(pushed)
    }

    fn open_for_review(
        &self,
        _worktree: &Worktree,
        base: &Base,
        review: &Review,
    ) -> Result<Opened, NotDelivered> {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::OpenedForReview {
                base: base.name().to_string(),
                review: review.clone(),
            });
        Ok(self.delivery.lock().expect("not poisoned").review.clone())
    }

    fn landed(&self, _in_repo: &str, pull_request: &str) -> WhatBecameOfIt {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::AskedWhatBecameOfIt {
                pull_request: pull_request.to_string(),
            });
        let delivery = self.delivery.lock().expect("not poisoned");
        WhatBecameOfIt {
            landing: delivery.landed.clone(),
            base: delivery.base_on_the_forge.clone(),
        }
    }

    fn under_review(&self, _in_repo: &str, pull_request: &str) -> UnderReview {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::AskedWhatIsUnderReview {
                pull_request: pull_request.to_string(),
            });
        self.delivery
            .lock()
            .expect("not poisoned")
            .under_review
            .clone()
    }

    fn merge(&self, _in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged> {
        // Recorded before the answer, because the forge is reached either way:
        // a refused merge is a call that happened and a test asserting nothing
        // was merged is asserting about the Job, not about the process.
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::Merged {
                pull_request: pull_request.to_string(),
            });
        match self.merging.lock().expect("not poisoned").clone() {
            Merging::Takes => Ok(Merged::Taken),
            Merging::AlreadyMerged => Ok(Merged::AlreadyMerged),
            Merging::Refuses(why) => Err(why),
        }
    }

    fn replied(&self, _in_repo: &str, pull_request: &str, saying: &str) -> Replied {
        // Recorded before the answer, for `merge`'s reason: the forge is
        // reached either way, and a reply the forge would not take is still a
        // call that happened.
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::Replied {
                pull_request: pull_request.to_string(),
                saying: saying.to_string(),
            });
        match self.replying.lock().expect("not poisoned").clone() {
            Replying::Takes => Replied::Posted,
            Replying::Refuses(why) => Replied::NotPosted { why },
        }
    }

    fn rendered_afresh(&self, _in_repo: &str, pull_request: &str) -> Renewed {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::AskedToRenderAfresh {
                pull_request: pull_request.to_string(),
            });
        self.delivery.lock().expect("not poisoned").renewed.clone()
    }

    fn caught_the_repository_up(&self, _in_repo: &str, base: &str) -> RepositoryStanding {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::CaughtTheRepositoryUp {
                base: base.to_string(),
            });
        self.delivery
            .lock()
            .expect("not poisoned")
            .repository
            .clone()
    }
}

impl Vcs for FakeVcs {
    type Error = FakeVcsError;
    type CommitError = FakeVcsError;

    fn create_worktree(&self, spec: &WorktreeSpec) -> Result<Worktree, Self::Error> {
        if let Some(standing_in_for) = self.refuse_next.lock().expect("not poisoned").take() {
            return Err(FakeVcsError::Refused { standing_in_for });
        }
        let branch = spec.branch();
        if !self
            .branches
            .lock()
            .expect("not poisoned")
            .insert(branch.clone())
        {
            return Err(FakeVcsError::BranchExists { branch });
        }
        let made = Worktree::at(spec.worktree_path(), branch);
        self.created
            .lock()
            .expect("not poisoned")
            .push(made.clone());
        Ok(made)
    }

    /// **`None` where no ref was scripted at all**, which is a repository that
    /// names no base — the same silence the real one answers with, and not the
    /// error a declared-but-missing branch raises.
    fn base_commit(
        &self,
        _repo_root: &str,
        declared: Option<&str>,
    ) -> Result<Option<String>, Self::Error> {
        let refs = self.refs.lock().expect("not poisoned");
        match declared {
            Some(name) => {
                refs.get(name)
                    .cloned()
                    .map(Some)
                    .ok_or_else(|| FakeVcsError::NoSuchRef {
                        r#ref: name.to_string(),
                    })
            }
            None => Ok(refs.values().next().cloned()),
        }
    }

    /// **Answers the same checkout every time it is asked for one commit**,
    /// which is the sharing the real one promises. The `prepared` flag comes
    /// back as it was left, so a second Job on one base is told it need not
    /// prepare again.
    fn base_checkout(&self, spec: &BaseSpec) -> Result<BaseCheckout, Self::Error> {
        if let Some(standing_in_for) = self.refuse_next.lock().expect("not poisoned").take() {
            return Err(FakeVcsError::Refused { standing_in_for });
        }
        let mut bases = self.bases.lock().expect("not poisoned");
        let prepared = *bases.entry(spec.commit().to_string()).or_insert(false);
        Ok(BaseCheckout::at(spec.path(), spec.commit(), prepared))
    }

    fn drop_base_checkout(&self, spec: &BaseSpec) -> Result<(), Self::Error> {
        self.bases
            .lock()
            .expect("not poisoned")
            .remove(spec.commit());
        self.dropped_bases
            .lock()
            .expect("not poisoned")
            .push(spec.commit().to_string());
        Ok(())
    }

    fn commit_all(
        &self,
        worktree: &Worktree,
        message: &str,
        at: CommitTime,
    ) -> Result<Committed, Self::CommitError> {
        match *self.commits.lock().expect("not poisoned") {
            Willing::NothingChanged => Ok(Committed::NothingToCommit),
            Willing::No(standing_in_for) => Err(FakeVcsError::NotCommitted { standing_in_for }),
            Willing::Yes => {
                let mut made = self.committed.lock().expect("not poisoned");
                made.push(FakeCommit {
                    branch: worktree.branch().to_string(),
                    message: message.to_string(),
                    at,
                });
                Ok(Committed::Made {
                    commit: format!("{:040x}", made.len()),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";

    fn spec(job: &str) -> WorktreeSpec {
        WorktreeSpec::for_job("/repos/armada", job).expect("a legal spec")
    }

    #[test]
    fn it_derives_the_same_path_and_branch_the_real_one_would() {
        let made = FakeVcs::new().create_worktree(&spec(JOB)).unwrap();
        assert_eq!(made.path(), spec(JOB).worktree_path());
        assert_eq!(made.branch(), spec(JOB).branch());
    }

    #[test]
    fn a_seeded_branch_is_refused_rather_than_reused() {
        let vcs = FakeVcs::new().with_existing_branch(format!("armada/{JOB}"));
        assert_eq!(
            vcs.create_worktree(&spec(JOB)),
            Err(FakeVcsError::BranchExists {
                branch: format!("armada/{JOB}")
            })
        );
        assert!(vcs.created().is_empty());
    }

    #[test]
    fn the_same_job_twice_collides_with_itself() {
        let vcs = FakeVcs::new();
        vcs.create_worktree(&spec(JOB)).expect("the first");
        assert!(vcs.create_worktree(&spec(JOB)).is_err());
    }

    #[test]
    fn a_scripted_refusal_applies_once() {
        let vcs = FakeVcs::new();
        vcs.refuse_next("a full disk");
        assert_eq!(
            vcs.create_worktree(&spec(JOB)),
            Err(FakeVcsError::Refused {
                standing_in_for: "a full disk"
            })
        );
        assert!(vcs.created().is_empty());
        vcs.create_worktree(&spec(JOB)).expect("the retry");
    }

    #[test]
    fn nothing_it_recorded_ever_goes_away() {
        let vcs = FakeVcs::new();
        vcs.create_worktree(&spec("01AAA")).unwrap();
        vcs.create_worktree(&spec("01BBB")).unwrap();
        assert!(vcs.create_worktree(&spec("01AAA")).is_err());
        assert_eq!(vcs.created().len(), 2);
    }

    #[test]
    fn it_creates_nothing_on_disk() {
        let vcs = FakeVcs::new();
        let made = vcs.create_worktree(&spec(JOB)).unwrap();
        assert!(!std::path::Path::new(made.path()).exists());
    }
}
