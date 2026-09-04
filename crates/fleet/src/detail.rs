//! One Job in full, assembled from the reads it takes.
//!
//! **The one operation on `api::Daemon` that is not a conversion.** Every other
//! method in `serving.rs` loads, delegates and redacts in under a dozen lines;
//! this one composes six store reads, the roster and the frozen workflow into a
//! single answer. It lives here so that `serving.rs` stays the trait impl,
//! which is the same argument that put `step_facts` in `wire.rs`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{JobDelivery, JobDetail, JobId, JobSpend};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::footprint::kept;
use crate::wire::{step_facts, step_moves};

/// One Job in full, folded from its log like every other read.
///
/// **The footprint read is spent only where there is one to read.** This is
/// called on every open of a Job, which is the argument that put the history and
/// the patch on routes of their own; a footprint is neither — a path and a word
/// per file — and it is written at the terminal transition. Asking only for a
/// Job that has stopped keeps an open of a running Job costing exactly what it
/// cost before, and `footprint` absent on one is the truth rather than an
/// omission.
///
/// **The wait a redirect left is on this read and on no other**, because it is
/// held in the slot rather than written down — `Fleet::redirect_awaited`.
pub(crate) async fn of<H, V, W>(fleet: &Fleet<H, V, W>, job_id: JobId) -> Result<JobDetail, Refusal>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let job = fleet
        .load(&job_id.to_domain())
        .await
        .map_err(|why| fleet.refusal(why))?;
    let reason = fleet
        .last_reason(job.id())
        .await
        .map_err(|why| fleet.refusal(why))?;
    let (ran, flagged, moves, ran_every_attempt, judged_every_attempt) = {
        let store = fleet.store().lock().await;
        let ran = store
            .step_checks(job.id())
            .map_err(|why| fleet.refusal(Adrift::Reading(why)))?;
        let flagged = store
            .step_gaming_flags(job.id())
            .map_err(|why| fleet.refusal(Adrift::Reading(why)))?;
        // The rows `get_job_events` serves, narrowed to the step moves and
        // **read on every open, unlike the history**: one entry per run of a
        // step rather than a row per move, so a rail can say `Attempt 1 refused`
        // without the unbounded read `history.rs` keeps off this.
        let moves =
            step_moves(&store, job.id()).map_err(|why| fleet.refusal(Adrift::Reading(why)))?;
        // **Every attempt's rows, beside the latest-only `ran` above**, which
        // stays latest-only because `why_stuck` below reads it as that.
        // `step_facts` wants every run's Checks and Judge answers stamped with
        // the attempt they belong to.
        let ran_every_attempt = store
            .step_checks_every_attempt(job.id())
            .map_err(|why| fleet.refusal(Adrift::Reading(why)))?;
        let judged_every_attempt = store
            .step_judgments_every_attempt(job.id())
            .map_err(|why| fleet.refusal(Adrift::Reading(why)))?;
        (ran, flagged, moves, ran_every_attempt, judged_every_attempt)
    };
    // The plans are read with the footprint and only with it: they are what it
    // is measured against, and a running Job has neither — its live reading is
    // marked from the slot, where the step being watched is the step that
    // declared.
    let recorded = match job.status().is_terminal() {
        false => None,
        true => {
            let store = fleet.store().lock().await;
            let kept = store
                .footprint(job.id())
                .map_err(|why| fleet.refusal(Adrift::Reading(why)))?;
            let plans = match kept.is_some() {
                false => Vec::new(),
                true => store
                    .step_plans(job.id())
                    .map_err(|why| fleet.refusal(Adrift::Reading(why)))?,
            };
            kept.map(|footprint| (footprint, plans))
        }
    };
    // Read on the same terms as the footprint and for the same reason: a Job
    // that has not finished has nothing here, and a read spent on every running
    // Job would buy three nulls.
    let delivery = match job.status().is_terminal() {
        false => None,
        true => {
            let came_to = fleet
                .store()
                .lock()
                .await
                .delivery_for(job.id())
                .map_err(|why| fleet.refusal(Adrift::Reading(why)))?;
            // Absent rather than three nulls: a Job that finished before Fleet
            // wrote this down is not a Job whose branch came to nothing, and the
            // surface says different sentences for the two.
            match came_to.is_empty() {
                true => None,
                false => Some(JobDelivery {
                    commit: came_to.commit,
                    pushed: came_to.pushed,
                    pull_request: came_to.pull_request,
                    landed: came_to.landed.as_ref().and_then(crate::noticing::settled),
                }),
            }
        }
    };
    let queued = fleet.queued_reason(&job).await?;
    // **Read for every Job, unlike the footprint and the delivery above.** Those
    // two are absent until a Job finishes; this one is what a person watching a
    // running Job wants most, and it is one indexed query. The cap travels with
    // the figure because neither half is readable alone.
    let allowance = fleet.allowance();
    let spent = fleet
        .spend_of(job.id())
        .await
        .map_err(|why| fleet.refusal(why))?;
    let spend = Some(JobSpend {
        cost_micros: spent.cost_micros,
        cost_cap_micros: allowance.cost().count(),
        turns: spent.turns,
        turn_cap: allowance.turns(),
        ran_ms: spent.ran_ms,
        drones: spent.drones,
    });
    // Before `step_facts`, which consumes the Check runs: the classification
    // reads them to answer whether an override is available, and reading them
    // twice would be a second answer to one question.
    let stuck = fleet.why_stuck(&job, reason.as_ref(), &ran).await;
    // A read, and only ever a read — `crate::overlap` says why it is reachable
    // from here and from nothing on the dispatch path.
    let overlaps = fleet
        .write_scope_overlaps(&job)
        .await
        .map_err(|why| fleet.refusal(why))?;
    Ok(JobDetail::of(
        &job,
        reason.as_ref(),
        queued,
        fleet.resumption(&job),
        &step_facts(
            fleet.aloft(),
            &fleet.host().repo_root,
            &job,
            ran_every_attempt,
            judged_every_attempt,
            flagged,
            &moves,
        ),
        recorded
            .as_ref()
            .map(|(footprint, plans)| kept(footprint, plans)),
        fleet.redirect_awaited(job.id()).await,
        fleet.question_awaited(job.id()).await,
        stuck.as_ref(),
        overlaps,
        delivery,
        spend,
    ))
}
