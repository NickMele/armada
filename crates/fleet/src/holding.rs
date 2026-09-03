//! What Fleet is holding disk for, and giving back the part that is provably
//! safe to take.
//!
//! # Provably safe, which is not the same as safe
//!
//! Five tests, and a worktree passes all five or none of it is automatic: the
//! Job is terminal, the base already reaches its branch, nothing in its
//! checkout is uncommitted, nobody is piloting it, and no Job still to run
//! depends on it. **Failing one does not make a worktree unsafe** — it makes it
//! not provably safe, a different claim, so every reason is carried on
//! [`Holding::held`] for a person to decide on. That surface is `#385`.
//!
//! # One derivation, shared with the delete
//!
//! `adapters::standing` is the reading, and it is the one `adapters::reclaim`
//! takes on its way to deleting a branch. Nothing here forms its own opinion
//! about what merged means, and nothing matches a path against a pattern:
//! every candidate is a Job the store handed out, reached through the
//! `WorktreeSpec` that derived its checkout. A hand-run `git branch -D` over
//! `armada/*` destroyed nine branches belonging to no Job.
//!
//! # What it took goes on the Job
//!
//! A sweep on a timer nobody reads means the first surprise is a worktree gone
//! with no record of who took it.
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, WorktreeSpec};
use adapters::{BranchStanding, Reclaimed, UnmergedWork, WorktreeStanding};
use core_model::{
    Component, DependencyDirection, Envelope, FieldValue, Job, JobId, JobStatus, Level,
};

use crate::adrift::Adrift;
use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::transcript;

/// How often Fleet asks what it could give back.
///
/// A newtype rather than a bare `Duration`, like [`Noticing`], so the value
/// cannot be handed to something measuring a different quantity.
///
/// [`Noticing`]: crate::Noticing
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reclaiming(Duration);

impl Reclaiming {
    pub const fn every(interval: Duration) -> Reclaiming {
        Reclaiming(interval)
    }

    pub const fn interval(&self) -> Duration {
        self.0
    }
}

/// One test a worktree did not pass, and therefore one reason Fleet is still
/// holding it.
///
/// **Five variants for the five tests, plus the two ways the reading itself can
/// fail.** A reading that failed is held for the same reason an unmerged branch
/// is: unanswered and safe must never read alike, because only one of them can
/// be taken back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Held {
    /// The Job is still moving, so it may still need its worktree.
    NotTerminal { status: JobStatus },
    /// **A person is at an unrestricted toolset in it** — `#367`. The worktree
    /// belongs to the engineer, and it is not Fleet's to offer either.
    Piloted,
    /// The branch holds commits the base cannot reach. `commits` of them, and
    /// `tip` is what the branch is recoverable from.
    Unmerged {
        base: String,
        commits: usize,
        tip: String,
    },
    /// Nothing here could say what the branch would be merged into.
    BaseUnanswered { why: String },
    /// Files written and never committed. **The test the other four cannot
    /// make**: uncommitted work leaves a branch level with its base, so every
    /// merged-ness reading says the checkout is disposable.
    Uncommitted { files: Vec<String> },
    /// Somebody locked the checkout, which is a person saying not yet.
    Locked { reason: String },
    /// A Job that depends on this one has not finished, so it may still need
    /// what this one wrote.
    DependedOn { by: Vec<JobId> },
    /// git would not say what is in the checkout.
    Unreadable { why: String },
}

/// One Job's worktree, and every test it failed.
///
/// **An empty `held` is the whole of the safety claim.** A sweep acts on that
/// and nothing else, so a test added to this module is a test the sweep starts
/// applying with no second place to change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holding {
    pub job: JobId,
    /// Empty where every test passed.
    pub held: Vec<Held>,
}

impl Holding {
    /// Whether Fleet may take this one back without asking anybody.
    pub fn provably_safe(&self) -> bool {
        self.held.is_empty()
    }

    /// Whether a surface may offer this one to a person at all.
    ///
    /// **Piloted is the one that must not be offered**, for `#367`'s reason: a
    /// person is working in it right now, and an act that is drawn is an act
    /// somebody eventually clicks.
    pub fn offerable(&self) -> bool {
        !self.held.contains(&Held::Piloted)
    }
}

/// One Job whose disk came back on this turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GaveBack {
    pub job: JobId,
    pub reclaimed: Reclaimed,
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Every worktree Fleet is holding disk for, and why each one is held.
    ///
    /// **Ordered by Job id and complete**, including the ones this Fleet would
    /// reclaim on its own — a caller that filtered to the held ones would be
    /// reading a list the sweep is about to change, and the two answers must
    /// come from one call. `#385` draws this, and reads
    /// [`Holding::offerable`] rather than deciding for itself.
    ///
    /// A Job with no checkout and no branch left is not in the answer at all.
    /// There is nothing to give back and nothing to hold.
    pub async fn worktrees_held(&self) -> Result<Vec<Holding>, Adrift> {
        let (loaded, _) = self.every_job().await?;
        let mine: Vec<&Job> = loaded
            .jobs
            .iter()
            // **This repository's Jobs only.** One store serves the machine and
            // a `WorktreeSpec` is derived from *this* Fleet's root, so another
            // Manifest's Job would name a path under a repository it never ran
            // in. `armada clean` selects on the same column for the same reason.
            .filter(|job| job.owner_manifest_id().as_str() == self.manifest().id().as_str())
            .collect();
        let mut holding = Vec::new();
        for job in &mine {
            if let Some(one) = self.holding_of(job, &mine) {
                holding.push(one);
            }
        }
        Ok(holding)
    }

    /// Reclaim every worktree that passes all five tests, if a sweep is due.
    ///
    /// **Empty on nearly every turn**, and empty is three different silences:
    /// too soon since the last sweep, nothing held at all, and nothing held
    /// that was provably safe. None is worth a line — what a person wants to
    /// hear about is disk that came back, and that is all this returns.
    ///
    /// A fault reclaiming one Job does not stop the next: the answer is per
    /// Job, and one repository refusing to open is the only thing that would
    /// make every one of them fail the same way.
    pub(crate) async fn reclaim_what_is_safe(&self) -> Result<Vec<GaveBack>, Adrift> {
        if !self.due_to_sweep().await {
            return Ok(Vec::new());
        }
        let held = self.worktrees_held().await?;
        let mut gave_back = Vec::new();
        for one in held.iter().filter(|one| one.provably_safe()) {
            if let Some(taken) = self.gave_back(&one.job) {
                gave_back.push(taken);
            }
        }
        Ok(gave_back)
    }

    /// Whether enough time has passed, stamping the sweep either way.
    ///
    /// The clock is the injected one, so a test decides when a sweep comes due
    /// rather than waiting for it — the shape `notice_a_merge` already has.
    async fn due_to_sweep(&self) -> bool {
        let now = self.now();
        let mut last = self.swept().lock().await;
        if let Some(before) = last.as_ref() {
            if elapsed(before, &now) < self.reclaiming().interval() {
                return false;
            }
        }
        *last = Some(now);
        true
    }

    /// Every test one Job's worktree failed, or `None` where there is nothing
    /// to give back.
    ///
    /// **The status tests come before the git ones and the git reading is taken
    /// anyway**, because a person looking at a piloted Job's worktree wants to
    /// know what is in it. What the order buys is that a Job whose worktree git
    /// cannot be asked about still reports the reasons that do not need git.
    fn holding_of(&self, job: &Job, board: &[&Job]) -> Option<Holding> {
        let spec = WorktreeSpec::for_job(&self.host().repo_root, job.id().as_str()).ok()?;
        let stands = adapters::standing(&spec, self.manifest().base()).ok()?;
        if stands.empty_handed() {
            return None;
        }
        let mut held = Vec::new();
        match job.status() {
            JobStatus::Piloted => held.push(Held::Piloted),
            status if !status.is_terminal() => held.push(Held::NotTerminal { status }),
            _ => {}
        }
        match &stands.branch {
            BranchStanding::Ahead { base, commits, tip } => held.push(Held::Unmerged {
                base: base.clone(),
                commits: *commits,
                tip: tip.clone(),
            }),
            BranchStanding::Unanswered { why, .. } => {
                held.push(Held::BaseUnanswered { why: why.clone() })
            }
            BranchStanding::Absent | BranchStanding::Merged { .. } => {}
        }
        match &stands.worktree {
            WorktreeStanding::Dirty { files } => held.push(Held::Uncommitted {
                files: files.clone(),
            }),
            WorktreeStanding::Locked { reason } => held.push(Held::Locked {
                reason: reason.clone(),
            }),
            WorktreeStanding::Unreadable { why } => {
                held.push(Held::Unreadable { why: why.clone() })
            }
            WorktreeStanding::Absent | WorktreeStanding::Clean => {}
        }
        let waiting = dependents_still_running(job.id(), board);
        if !waiting.is_empty() {
            held.push(Held::DependedOn { by: waiting });
        }
        Some(Holding {
            job: job.id().clone(),
            held,
        })
    }

    /// Take one Job's disk back and write down that it was taken.
    ///
    /// **[`UnmergedWork::Keep`] even here**, where the branch has already been
    /// read as merged. The setting is not a parameter on this seam — a live
    /// Fleet must never be the thing that destroys work nobody has taken — and
    /// a second reading that disagreed with the first would be caught by it
    /// rather than acted on.
    fn gave_back(&self, job: &JobId) -> Option<GaveBack> {
        let spec = WorktreeSpec::for_job(&self.host().repo_root, job.as_str()).ok()?;
        let reclaimed = adapters::reclaim(&spec, self.manifest().base(), UnmergedWork::Keep)
            .inspect_err(|cause| self.noted_unreclaimed(job, &cause.why))
            .ok()?;
        self.noted_reclaimed(job, &reclaimed);
        Some(GaveBack {
            job: job.clone(),
            reclaimed,
        })
    }

    /// Write into the Job's own log that Fleet took its disk.
    ///
    /// **The Job's log and not Fleet's stdout.** Fleet's console is the
    /// operator's and is not where anybody reads a Job, so a sweep that only
    /// reported there would leave the Job's own record saying nothing about
    /// what happened to it — which is the surprise this whole line exists to
    /// prevent.
    fn noted_reclaimed(&self, job: &JobId, reclaimed: &Reclaimed) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "fleet reclaimed this job's worktree, every safety test having passed",
        )
        .in_job(job.as_ulid().clone())
        .with_field("worktree", FieldValue::Str(said_of(&reclaimed.worktree)))
        .with_field("branch", FieldValue::Str(said_of_branch(&reclaimed.branch)));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }

    fn noted_unreclaimed(&self, job: &JobId, why: &str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "a worktree every safety test passed was not given back",
        )
        .in_job(job.as_ulid().clone())
        .with_field("cause", FieldValue::Str(why.to_string()));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}

/// The Jobs that named this one and have not finished.
///
/// **`depends_on` in the reverse direction**, which is the one direction
/// `crate::coupling` does not walk: it asks what one Job is waiting on, and
/// this asks who is waiting on it. A dependent that has itself reached a
/// terminal will never run again and needs nothing; one that has not may still
/// need what this Job wrote, and the work is on disk rather than in the record.
fn dependents_still_running(job: &JobId, board: &[&Job]) -> Vec<JobId> {
    board
        .iter()
        .filter(|other| !other.status().is_terminal())
        .filter(|other| {
            other
                .dependencies()
                .iter()
                .any(|edge| edge.direction == DependencyDirection::DependsOn && &edge.peer == job)
        })
        .map(|other| other.id().clone())
        .collect()
}

/// What became of the checkout, in one line for the Job's log.
fn said_of(worktree: &adapters::WorktreeGone) -> String {
    use adapters::WorktreeGone::*;
    match worktree {
        Removed { path } => format!("removed {path}"),
        RecordCleared { path } => format!("the record for {path} was cleared"),
        DirectoryRemoved { path } => format!("removed the directory {path}"),
        Absent { path } => format!("nothing at {path}"),
        Locked { path, reason } => format!("{path} is locked: {reason}"),
        NotRemoved { path, why } => format!("{path} was not removed: {why}"),
    }
}

/// What became of the branch. **The tip is carried on every arm that has one**,
/// because a deleted branch is recoverable from its SHA and from nothing else.
fn said_of_branch(branch: &adapters::BranchGone) -> String {
    use adapters::BranchGone::*;
    match branch {
        Deleted { branch, tip } => format!("deleted {branch} at {tip}"),
        Absent { branch } => format!("no branch {branch}"),
        Kept {
            branch,
            tip,
            base,
            commits,
        } => format!("kept {branch} at {tip}: {commits} commits {base} cannot reach"),
        KeptUnanswered { branch, tip, why } => format!("kept {branch} at {tip}: {why}"),
        NotDeleted { branch, why } => format!("{branch} was not deleted: {why}"),
    }
}
