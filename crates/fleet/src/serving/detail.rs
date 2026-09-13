//! One Job in full: the read Bridge makes on every open of one.
//!
//! **Split out of [`serving`](mod@crate::serving) when that file crossed the
//! 900-line rule.** This is one quarter of it and one read — the biggest thing
//! in the crate's redaction layer — and it is still that layer: every value
//! below is put onto an `ipc` DTO by hand, `crate::serving`'s own rule.
//!
//! The trait method is next door and delegates here, because `Queries` is one
//! `impl` block and a trait implementation cannot be split across two.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{JobDelivery, JobDetail, JobId, JobReview, JobSpend};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::footprint::kept;
use crate::wire::{step_facts, step_moves};

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
    /// One Job in full, folded from its log like every other read.
    ///
    /// # The footprint read is spent only where there is one to read
    ///
    /// This call is made on every open of a Job, which is the argument that put
    /// the history and the patch on routes of their own. A footprint is neither
    /// — it is a path and a word per file — and it is written at the terminal
    /// transition, so a Job that is still going has none. Asking only for a
    /// Job that has stopped is what keeps an open of a running Job costing
    /// exactly what it cost before, and `footprint` absent on one of them is
    /// the truth rather than an omission.
    ///
    /// **The wait a redirect left is on this read and on no other**, because it
    /// is held in the slot rather than written down — `Fleet::redirect_awaited`.
    pub(crate) async fn job_detail(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let reason = self
            .last_reason(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let (ran, flagged, moves, ran_every_attempt, judged_every_attempt, frames) = {
            let store = self.store().lock().await;
            let ran = store
                .step_checks(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // Every attempt's, for `judged_every_attempt`'s reason below: a
            // flag that refused an earlier run belongs to that run's record.
            let flagged = store
                .step_gaming_flags_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // The rows `get_job_events` serves, narrowed to the step moves.
            // **Read on every open, unlike the history**: one entry per run of
            // a step rather than a row per move, so a rail can say `Attempt 1
            // refused` without the unbounded read `history.rs` keeps off this.
            let moves =
                step_moves(&store, job.id()).map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // **Every attempt's rows, beside the latest-only `ran` above.**
            // `ran` stays latest-only because `why_stuck` below reads it as
            // that; `step_facts` wants every run's Checks and Judge answers
            // stamped with the attempt they belong to, which these two give.
            let ran_every_attempt = store
                .step_checks_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let judged_every_attempt = store
                .step_judgments_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // Every attempt's, for `ran_every_attempt`'s reason: a person
            // comparing the run that was handed back against the one that
            // passed needs both, and the rail stamps each with its run.
            let frames = store
                .step_frames_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            (
                ran,
                flagged,
                moves,
                ran_every_attempt,
                judged_every_attempt,
                frames,
            )
        };
        // The plans are read with the footprint and only with it: they are what
        // it is measured against, and a running Job has neither — its live
        // reading is marked from the slot, where the step being watched is the
        // step that declared.
        let recorded = match job.status().is_terminal() {
            false => None,
            true => {
                let store = self.store().lock().await;
                let kept = store
                    .footprint(job.id())
                    .map_err(|why| self.refusal(Adrift::Reading(why)))?;
                let plans = match kept.is_some() {
                    false => Vec::new(),
                    true => store
                        .step_plans(job.id())
                        .map_err(|why| self.refusal(Adrift::Reading(why)))?,
                };
                kept.map(|footprint| (footprint, plans))
            }
        };
        // Read for every Job, not only a finished one. Delivery is written when
        // the Job *enters* its delivering step — `crate::landing`'s module
        // doc, since #520 — so a Job holding at its handoff gate has a record
        // here well before it reaches a terminal status, and that is exactly
        // the read the gate wants: the pull request it is meant to review.
        let came_to = self
            .store()
            .lock()
            .await
            .delivery_for(job.id())
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        // Absent rather than three nulls: a Job with no delivering step, or one
        // that has not reached its delivering step yet, is not a Job whose
        // branch came to nothing, and the surface says different sentences for
        // the two.
        let delivery = match came_to.is_empty() {
            true => None,
            false => {
                // **Never a forge call.** `get_job` is read on every
                // open of a Job; what Fleet's own rotation last read
                // live is served from memory — `Sweep::pr_detail` — and
                // absent here is a pull request the rotation has not
                // reached yet, or one that has already settled.
                let pull_request_detail = match &came_to.pull_request {
                    Some(url) => self.sweeping().lock().await.pr_detail.get(url).cloned(),
                    None => None,
                };
                Some(JobDelivery {
                    commit: came_to.commit,
                    pushed: came_to.pushed,
                    pull_request: came_to.pull_request,
                    pull_request_detail,
                    landed: came_to.landed.as_ref().and_then(crate::noticing::settled),
                    unpushed: came_to.unpushed,
                })
            }
        };
        // Read for every Job, like `delivery` above and for the same reason:
        // a workflow with no delivering step still stops for a person at its
        // own gate, and the review area wants what Fleet composed there just
        // as much as a gate that opens a pull request does. `#665`.
        let composed = self
            .store()
            .lock()
            .await
            .review_for(job.id())
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        let review = match (
            composed.why,
            composed.outcome,
            composed.risks,
            composed.evidence,
        ) {
            (Some(why), Some(outcome), Some(risks), Some(evidence)) => Some(JobReview {
                why,
                outcome,
                risks,
                evidence,
            }),
            // **All four or none**, `store::Review::is_empty`'s own shape: a
            // row that somehow held three of the four would still be a Job
            // that has not reached a gate, for anything a surface could do
            // with it.
            _ => None,
        };
        let queued = self.queued_reason(&job).await?;
        // **Read for every Job, unlike the footprint above.** That one is
        // absent until a Job finishes; this one is what a person watching a
        // running Job wants most, and it is one indexed query. The cap
        // travels with the figure because neither half is readable alone.
        // **The allowance this Job is held to, not the installation's.** A
        // person who raised this Job's cap reads the figure they set beside the
        // spend, and a detail still drawing the machine-wide number would say
        // the Job was over budget on a Job admission is about to start.
        let allowance = self.allowance_for(&job);
        let spent = self
            .spend_of(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let spend = Some(JobSpend {
            cost_micros: spent.cost_micros,
            cost_cap_micros: allowance.cost().count(),
            unpriced: spent.unpriced,
            turns: spent.turns,
            turn_cap: allowance.turns(),
            ran_ms: spent.ran_ms,
            drones: spent.drones,
        });
        // Before `step_facts`, which consumes the Check runs: the
        // classification reads them to answer whether an override is available,
        // and reading them twice would be a second answer to one question.
        let stuck = self.why_stuck(&job, reason.as_ref(), &ran).await;
        // A read, and only ever a read — `crate::overlap` says why it is
        // reachable from here and from nothing on the dispatch path.
        let overlaps = self
            .write_scope_overlaps(&job)
            .await
            .map_err(|why| self.refusal(why))?;
        let mut detail = JobDetail::of(
            &job,
            reason.as_ref(),
            queued.reason,
            queued.budget,
            self.resumption(&job),
            &step_facts(
                self.aloft(),
                self.underway(),
                self.served_by(&job)
                    .map_err(|why| self.refusal(why))?
                    .records_root(),
                &job,
                ran_every_attempt,
                judged_every_attempt,
                flagged,
                frames,
                &moves,
            ),
            recorded
                .as_ref()
                .map(|(footprint, plans)| kept(footprint, plans)),
            self.redirect_awaited(job.id()).await,
            self.question_awaited(job.id()).await,
            stuck.as_ref(),
            overlaps,
            delivery,
            spend,
            review,
        );
        // After the constructor for `show_again`'s reason: the setting is a
        // store read, the waiting command is on the slot, and what a refused
        // row may be answered with is the Manifest's and the harness's.
        detail.when_blocked = Some(self.when_blocked_of(job.id()).await);
        detail.command_waiting = self.command_awaited(job.id()).await;
        detail.judge_question = self.judge_question_of(job.id()).await;
        detail.when_refused = Some(self.when_refused_of(job.id()).await);
        detail.allowed_commands = self
            .allowed_of(job.id())
            .await
            .iter()
            .map(ipc::AllowedCommandRow::from)
            .collect();
        detail.repository_allowed_commands = self
            .repository_allowed(&self.served_by(&job).map_err(|why| self.refusal(why))?)
            .await
            .iter()
            .map(ipc::AllowedCommandRow::from)
            .collect();
        detail.model_override = self.model_override_of(job.id()).await;
        // The row nested here is built inside `JobDetail::of`, so its counts are
        // filled from the same reading as the plan beside it.
        let plan = self
            .plan_of(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        detail.job.tasks = plan.as_ref().map(|plan| plan.counts().into());
        detail.work_plan = plan.as_ref().map(ipc::WorkPlan::from);
        if let Some(stuck) = detail.stuck.as_mut() {
            for refused in &mut stuck.refused {
                let command = (refused.tool == "Bash").then(|| refused.detail.clone());
                let offered = self
                    .offers_after(&job, &refused.tool, command.as_deref())
                    .await;
                refused.offers = offered.answers;
                refused.withheld = offered.withheld;
                refused.rules = offered.rules;
                refused.suggested_rule = offered.suggested_rule;
            }
        }
        // After the constructor, because it is read off the worktree and the
        // Manifest as well as the record — `crate::showing_again`.
        detail.show_again = Some(
            self.showing_again_of(&job)
                .await
                .map_err(|why| self.refusal(why))?,
        );
        Ok(detail)
    }
}
