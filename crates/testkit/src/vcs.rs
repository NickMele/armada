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

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::sync::Mutex;

use adapter_traits::{
    Base, BroughtUpToDate, Change, CommitTime, Committed, Delivery, Landed, NotDelivered, Opened,
    Pushed, Review, Standing, Vcs, Worktree, WorktreeSpec,
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
    AskedWhatBecameOfIt { branch: String },
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
    pub landed: Landed,
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
            landed: Landed::Unknown,
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

    fn landed(&self, worktree: &Worktree) -> Landed {
        self.delivered
            .lock()
            .expect("not poisoned")
            .push(Delivered::AskedWhatBecameOfIt {
                branch: worktree.branch().to_string(),
            });
        self.delivery.lock().expect("not poisoned").landed.clone()
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
