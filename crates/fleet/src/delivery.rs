//! Getting a Job's work back to the branch it merges into.
//!
//! # Rebasing is Fleet's, on every path, and it is one call
//!
//! `docs/concepts/fleet.md`, *Catching a branch up*, is the rule.
//! [`caught_up_onto`](Fleet::caught_up_onto) is the only place a boundary
//! rebases, and every path that starts, resumes or advances a step reaches it.
//!
//! # Two moments, and they used to be three
//!
//! At a **spawn** the rebase runs before the process exists and what moved
//! rides the opening brief — `crate::spawning`. At the last step the branch is
//! pushed and opened for review, with no Drone to hand anything to. The third
//! was a boundary that is not the last, where the Drone was alive and heard
//! what moved in the turn it got for the next step; a Drone belongs to a step
//! now, so every boundary that is not the last is a spawn.
//!
//! # A boundary is asked, never the Drone
//!
//! Asking the Drone whether its branch is behind would be asking it to manage
//! its own state, which `docs/concepts/drone.md` says it cannot be trusted to
//! do — and it has just submitted, so git can answer on its own. What the
//! worktree is *holding* is not checked either: the rebase carries uncommitted
//! work across and puts it back. See `adapters`' delivery module.

use adapter_traits::{
    AgentHarness, Base, BroughtUpToDate, Delivery, Opened, Pushed, Standing, Vcs, WorkProduct,
    Worktree,
};
use core_model::Job;
use verification::TheBaseMoved;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::review::review_of;

/// What happened to a Job's branch this turn.
///
/// **Every field is optional and absent means not attempted**, which is the
/// distinction a person reading this needs: a push that did not happen because
/// a conflict stopped everything before it is not a push that failed. A step
/// boundary that is not the last fills the first two and leaves the rest empty,
/// because nothing is published until a Job is finished.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Delivered {
    /// The branch it merges into. `None` where the repository names none, and
    /// then nothing was rebased and no pull request could name a target.
    pub base: Option<Base>,
    /// What catching up came to. `None` where the branch was not behind.
    pub caught_up: Option<BroughtUpToDate>,
    /// `None` where a conflict stopped everything after it.
    pub pushed: Option<Pushed>,
    pub opened: Option<Opened>,
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
    /// Bring this branch up to its base, in the worktree it is checked out in.
    ///
    /// **The one call.** `#150` found two paths that advanced a step without
    /// it and `#180` found a third, which is the shape of a missing seam rather
    /// than three oversights — so this is where the decision lives and every
    /// caller reaches it rather than repeating it.
    ///
    /// **Nothing is created and nothing is discarded.** The worktree named here
    /// is the one that already exists, holding whatever a previous Drone left
    /// in it; a clean rebase updates it in place and a conflicted one writes
    /// markers into it. That is what makes this compatible with `#62`, where a
    /// restart exists precisely so the earlier attempt's work survives.
    ///
    /// `None` is a repository that names no base, or a branch that is not
    /// behind one. Both are silence rather than an event.
    ///
    /// **One Job at a time from the rebase onward.** Dispatch and the work run
    /// N-wide; this does not, because every worktree is cut from one `.git` and
    /// `Fleet::merge_end` — taken here and in `crate::landing` — lets one Job
    /// touch it at a time. That field carries the argument and the cost.
    pub(crate) async fn caught_up_onto(
        &self,
        job_id: &core_model::JobId,
        worktree: &Worktree,
    ) -> Result<Option<TheBaseMoved>, Adrift> {
        let Some(base) = self.the_base(job_id, worktree)? else {
            return Ok(None);
        };
        if self.behind(job_id, worktree, &base)? == 0 {
            return Ok(None);
        }
        // **One Job at a time from here**, and only from here: the two reads
        // above open the repository and change nothing, and a rebase is the
        // first thing on this path that writes into the `.git` every worktree
        // shares. See `Fleet::merge_end`.
        let _at_the_merge_end = self.merge_end().lock().await;
        let moved = self
            .vcs()
            .bring_up_to_date(worktree, &base)
            .map_err(|why| Adrift::from_delivery(job_id, why))?;
        // Left where the turn that reports it will find it. A boundary and a
        // finish are two moments and one question — what happened to this Job's
        // branch — so they answer through one field rather than two.
        self.left_delivered(
            job_id,
            Delivered {
                base: Some(base),
                caught_up: Some(moved.clone()),
                ..Delivered::default()
            },
        )
        .await;
        Ok(Some(match moved {
            BroughtUpToDate::Clean { base, commits } => {
                TheBaseMoved::BroughtUpToDate { base, commits }
            }
            BroughtUpToDate::Conflicted { base, files } => TheBaseMoved::Conflicted { base, files },
            BroughtUpToDate::PutBack { base, .. } => TheBaseMoved::CouldNotFollow { base },
        }))
    }

    /// Catch the finished Job's branch up, push it, and open it for review.
    ///
    /// Called after the commit, so the worktree is clean and the branch carries
    /// the whole change. Each stage is skipped when the one before it says
    /// there is nothing to do it to — a branch that would not replay is not
    /// pushed, and a branch that reached no remote gets no pull request.
    pub(crate) async fn deliver(
        &self,
        job: &Job,
        worktree: &Worktree,
    ) -> Result<Delivered, Adrift> {
        let job_id = job.id().clone();
        let mut delivered = Delivered {
            base: self.the_base(&job_id, worktree)?,
            ..Delivered::default()
        };
        if let Some(base) = delivered.base.clone() {
            if self.behind(&job_id, worktree, &base)? > 0 {
                let moved = self
                    .vcs()
                    .bring_up_to_date(worktree, &base)
                    .map_err(|why| Adrift::from_delivery(&job_id, why))?;
                let replayed = matches!(moved, BroughtUpToDate::Clean { .. });
                delivered.caught_up = Some(moved);
                // A branch known to conflict with what it merges into is not
                // pushed. The work is committed and the worktree is held; a
                // person resolves it, and a pull request opened over it would
                // be a review request nobody can act on.
                if !replayed {
                    return Ok(delivered);
                }
            }
        }

        let pushed = self
            .vcs()
            .push(worktree)
            .map_err(|why| Adrift::from_delivery(&job_id, why))?;
        let reached_a_remote = pushed != Pushed::NoRemote;
        delivered.pushed = Some(pushed);

        delivered.opened = match (&delivered.base, reached_a_remote) {
            (Some(base), true) => Some(self.opened_for_review(job, worktree, base).await?),
            // A repository with no remote is ordinary and not an error: the
            // branch is the work, and a person merges it where it is.
            (_, false) => Some(Opened::NothingPushed),
            (None, true) => None,
        };
        Ok(delivered)
    }

    /// Assemble the pull request from the record and open it.
    async fn opened_for_review(
        &self,
        job: &Job,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<Opened, Adrift> {
        let checks = self
            .store()
            .lock()
            .await
            .step_checks(job.id())
            .map_err(Adrift::Reading)?;
        // **Read here and nowhere earlier.** It changes nothing about what was
        // rebased — the base on this machine is the branch a person merges into
        // — and it is the pull request, not the Job, that is wrong when the two
        // disagree.
        let remote = self
            .vcs()
            .base_on_the_remote(worktree, base)
            .map_err(|why| Adrift::from_delivery(job.id(), why))?;
        let review = review_of(job, &checks, base, &remote);
        self.vcs()
            .open_for_review(worktree, base, &review)
            .map_err(|why| Adrift::from_delivery(job.id(), why))
    }

    /// The branch this repository's work merges into.
    ///
    /// The Manifest's `base:` is handed down; **inference is what the adapter
    /// does when nothing was declared**, so the fallback lives beside the
    /// repository it is reading rather than here.
    fn the_base(
        &self,
        job_id: &core_model::JobId,
        worktree: &Worktree,
    ) -> Result<Option<Base>, Adrift> {
        self.vcs()
            .base(worktree, self.manifest().base())
            .map_err(|why| Adrift::from_delivery(job_id, why))
    }

    /// How many commits the base holds that the branch has not got.
    fn behind(
        &self,
        job_id: &core_model::JobId,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<usize, Adrift> {
        match self
            .vcs()
            .standing(worktree, base)
            .map_err(|why| Adrift::from_delivery(job_id, why))?
        {
            Standing::UpToDate => Ok(0),
            Standing::Behind { commits } => Ok(commits),
        }
    }
}
