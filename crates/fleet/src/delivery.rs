//! Getting a Job's work back to the branch it merges into.
//!
//! **Rebasing is Fleet's, on every path, one call.** `docs/concepts/fleet.md`, *Catching a
//! branch up*, is the rule; [`caught_up_onto`](Fleet::caught_up_onto) is the only place a
//! boundary rebases, and every path that starts, resumes or advances a step reaches it.
//!
//! **Two moments, used to be three.** At a **spawn** it runs before the process exists and what
//! moved rides the opening brief (`crate::spawning`); on the step that delivers, entering it
//! commits, pushes and opens for review (`crate::landing`). The third was a live Drone hearing
//! what moved mid-turn; a Drone belongs to a step now, so every boundary is a spawn.
//!
//! **A boundary is asked, never the Drone** — asking it would be asking it to manage its own
//! state, which `docs/concepts/drone.md` says it cannot be trusted to do, and it has just
//! submitted, so git can answer on its own. The worktree's *holdings* are not checked either:
//! the rebase carries uncommitted work across and puts it back (`adapters`' delivery module).

use adapter_traits::{
    AgentHarness, Base, BroughtUpToDate, Changed, Delivery, Opened, Pushed, Standing, Vcs,
    WorkProduct, Worktree,
};
use core_model::{Component, Envelope, FieldValue, Job, Level, StepId};
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

impl Delivered {
    /// Why this delivery's commit never reached its remote, where
    /// [`deliver`](Fleet::deliver) returned before the push. `#691`.
    ///
    /// **`None` on every shape but one.** A clean push, a repository with no
    /// base, and a worktree with nothing new all leave [`pushed`](Self::pushed)
    /// set or leave [`caught_up`](Self::caught_up) empty — `deliver` sets
    /// `pushed` on every path except the one that returns before reaching it,
    /// so the two conditions together name that path and no other. **The one
    /// sentence**, so the Job's log, the stored record and the wire agree on
    /// what a person reads.
    pub fn unpushed_reason(&self) -> Option<String> {
        if self.pushed.is_some() {
            return None;
        }
        match self.caught_up.as_ref()? {
            BroughtUpToDate::Clean { .. } => None,
            BroughtUpToDate::Conflicted { base, .. } => Some(format!(
                "catching the branch up to `{base}` left conflicts in the files, so the commit \
                 was not pushed"
            )),
            BroughtUpToDate::PutBack { base, .. } => Some(format!(
                "the branch's own commits would not replay onto `{base}`, so they were put back \
                 exactly as they were and the commit was not pushed"
            )),
        }
    }
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
    /// **The one call** — `#150` found two paths that advanced a step without it and `#180`
    /// found a third, the shape of a missing seam rather than three oversights, so every
    /// caller reaches it rather than repeating it.
    ///
    /// **Nothing is created and nothing is discarded** — the worktree named here already
    /// exists, holding whatever a previous Drone left; a clean rebase updates it in place and
    /// a conflicted one writes markers into it, compatible with `#62`, where a restart exists
    /// precisely so the earlier attempt's work survives. `None` is a repository naming no
    /// base, or a branch not behind one — both are silence rather than an event.
    ///
    /// **One Job at a time from the rebase onward** — dispatch and the work run N-wide, this
    /// does not, because every worktree is cut from one `.git` and `Fleet::merge_end` — taken
    /// here and in `crate::landing` — lets one Job touch it at a time.
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

        // **`--force-with-lease`, unconditionally, not only where this call's
        // own rebase moved anything.** `#663` is the first thing in this
        // codebase that can reach this call a second time for one branch —
        // `crate::conflict_resolution` sends a Job's delivering step back
        // through here once a Drone has resolved a conflict — and by the time
        // the delivering step is *entered* again, the rebase that rewrote its
        // history already happened, at the earlier step's own spawn-time
        // catch-up, not here. What a plain push refuses on is exactly the
        // shape that redelivery leaves behind, so this is the one push and
        // not a choice between two: `armada/*` is a namespace nothing but
        // Fleet ever pushes to, and the lease that guards a rewrite guards an
        // ordinary push identically to a plain one — see `adapters`' own
        // test proving a brand-new branch's first push takes it the same way.
        let pushed = self
            .vcs()
            .push_forcing(worktree)
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
        // **The cheap reading, not the counted one.** `counted_files` costs the
        // patch and is already spent once, at the transition that ends a Job;
        // this is the delta walk. A worktree that will not answer leaves the
        // section saying nothing changed rather than failing a delivery whose
        // work is already committed and pushed.
        let changed = self
            .work()
            .changed_files(worktree)
            .unwrap_or_else(|_| Changed::nothing());
        let (review, structured) = review_of(job, &checks, base, &remote, &changed);
        let opened = self
            .vcs()
            .open_for_review(worktree, base, &review)
            .map_err(|why| Adrift::from_delivery(job.id(), why))?;
        // **The same composition, kept beside the Job.** The pull request
        // already carries `review`'s Markdown; this is what lets `get_job`
        // serve the sections it was built from with no second read of the
        // worktree.
        self.store()
            .lock()
            .await
            .record_review(job.id(), &crate::review::as_stored(&structured))
            .map_err(Adrift::Writing)?;
        Ok(opened)
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

    /// Compose and store the review at a gate that is not the step a workflow
    /// declares delivering.
    ///
    /// **The delivering step's own gate is [`opened_for_review`](Fleet::opened_for_review)'s.**
    /// That one composes at the step's *entry*, before its own Checks exist,
    /// so its words match what the pull request already carries — see
    /// `crate::review`'s module doc. Recomposing here, after the step's Checks
    /// are in, would serve a review the pull request no longer says, so a
    /// step this workflow declares delivering is skipped.
    ///
    /// Every other `human_always` stop has no Markdown to protect and reaches
    /// here instead — `#665`, so the review area says the same things at a
    /// gate that never opens a pull request.
    pub(crate) async fn compose_review_at_gate(
        &self,
        job: &Job,
        step: &StepId,
        worktree: &Worktree,
    ) {
        let delivers = job
            .workflow()
            .step(step)
            .is_some_and(core_model::ResolvedStep::delivers);
        if delivers {
            return;
        }
        if let Err(why) = self.compose_and_store_review(job, worktree).await {
            self.noted_review_not_composed(job, step, &why);
        }
    }

    /// The read and the write [`compose_review_at_gate`](Fleet::compose_review_at_gate)
    /// is held around. A worktree with no base still has Checks and a diff, so
    /// only the two git reads are optional.
    async fn compose_and_store_review(&self, job: &Job, worktree: &Worktree) -> Result<(), Adrift> {
        let checks = self
            .store()
            .lock()
            .await
            .step_checks(job.id())
            .map_err(Adrift::Reading)?;
        let base = self.the_base(job.id(), worktree)?;
        let remote = match &base {
            Some(base) => Some(
                self.vcs()
                    .base_on_the_remote(worktree, base)
                    .map_err(|why| Adrift::from_delivery(job.id(), why))?,
            ),
            None => None,
        };
        let changed = self
            .work()
            .changed_files(worktree)
            .unwrap_or_else(|_| Changed::nothing());
        let structured = crate::review::structured_review_of(
            job,
            &checks,
            base.as_ref(),
            remote.as_ref(),
            &changed,
        );
        self.store()
            .lock()
            .await
            .record_review(job.id(), &crate::review::as_stored(&structured))
            .map_err(Adrift::Writing)
    }

    /// Write into the Job's own log that the review area's text did not
    /// compose at this gate, and why. **Held, never raised**: the gate has
    /// already ruled and a person is about to be shown the Job either way —
    /// what this costs is a section of context, not the stop itself.
    fn noted_review_not_composed(&self, job: &Job, step: &StepId, cause: &Adrift) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the review area's own text did not compose at this gate",
        )
        .in_job(job.id().as_ulid().clone())
        .at_step(step.as_str())
        .with_field("cause", FieldValue::Str(cause.to_string()));
        self.noted_in_the_log(job.id(), &envelope);
    }
}
