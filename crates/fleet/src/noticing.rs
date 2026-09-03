//! Noticing that somebody merged a Job's pull request.
//!
//! # Noticing, not merging
//!
//! Armada opens a pull request and a person merges it — `docs/scope.md`, and
//! the `human_always` gate on every shipped workflow's last step. Nothing here
//! merges, and no method could: [`Delivery::landed`] reads, and the only thing
//! written is the Job's own record.
//!
//! # One pull request per sweep, and never on the turn interval
//!
//! The loop ticks four times a second and asking the forge is a process. An
//! open pull request needs asking rarely and a merged one never again, so this
//! asks about **one** Job every [`Noticing`] interval and rotates: ten open at
//! a minute apiece is each asked every ten minutes, and a merge recorded
//! leaves the rotation for good. The cost is one blocking call on the
//! interval, which is the shape `caught_up_onto` has for a far longer rebase.
//!
//! # The repository root, not the Job's worktree
//!
//! A Job's worktree is reclaimed long before anybody merges its work, so
//! asking from there would answer `Unknown` for exactly the Jobs this exists
//! for. The question is asked from the repository every worktree is cut from,
//! carrying the branch the record holds.

use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Landing, Vcs, WorkProduct};
use core_model::{JobId, Timestamp};

use crate::adrift::Adrift;
use crate::converging::elapsed;
use crate::daemon::Fleet;
use ipc::Settled;

/// How often the forge is asked about one pull request.
///
/// A newtype rather than a bare `Duration`, like [`Polling`], so the value
/// cannot be handed to something measuring a different quantity.
///
/// [`Polling`]: crate::Polling
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Noticing(Duration);

impl Noticing {
    pub const fn every(interval: Duration) -> Noticing {
        Noticing(interval)
    }

    pub const fn interval(&self) -> Duration {
        self.0
    }
}

/// A pull request that settled, on the turn Fleet read that it had.
///
/// **Once per Job and never again**, because the answer is written down and a
/// settled Job leaves the rotation. A surface that saw this twice for one Job
/// would be watching Fleet re-ask a question it had already answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Noticed {
    pub job: JobId,
    /// What became of it. Only ever [`Landing::Merged`] or
    /// [`Landing::ClosedUnmerged`] — the other two are not news and are not
    /// recorded, so they never reach here.
    pub landed: Landing,
}

/// Where the rotation stands, and when it last ran.
///
/// **In memory and never written down.** A cursor that outlived the process
/// would name a position in a list that has since changed, and losing it costs
/// one restart of the rotation over a set that is small by construction.
#[derive(Clone, Debug, Default)]
pub(crate) struct Sweep {
    pub(crate) last: Option<Timestamp>,
    pub(crate) next: usize,
}

/// What the record's state says on the wire, where it says anything.
///
/// **The one mapping**, read by the event, by the Job detail and by the Board
/// row. A second one written out beside any of those is how the same three
/// states come to render two ways.
///
/// `None` is a state the wire has no word for, and it has none by design:
/// `Open` and `Unknown` are the absence of news, and are never stored, so
/// nothing that reads the record reaches them.
pub fn settled(landed: &Landing) -> Option<Settled> {
    match landed {
        Landing::Merged { .. } => Some(Settled::Merged),
        Landing::ClosedUnmerged { .. } => Some(Settled::ClosedUnmerged),
        Landing::Open { .. } | Landing::Unknown => None,
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
    /// Ask the forge about one Job's pull request, if it is time to.
    ///
    /// **`None` is nearly every turn**, and it is three different silences: too
    /// soon since the last ask, no pull request left to ask about, and an
    /// answer that was not news. None of the three is worth a line anywhere —
    /// what a person wants to hear about is a merge, and that is the only thing
    /// this returns.
    ///
    /// **Held rather than raised where the forge is unreachable.** A machine
    /// with no `gh`, or one nobody has signed in on, answers [`Landing::Unknown`]
    /// forever, and a turn that failed over it would make every Job's next turn
    /// wait on a repository nobody can reach.
    pub(crate) async fn notice_a_merge(&self) -> Result<Option<Noticed>, Adrift> {
        let Some(asking) = self.due_to_ask().await? else {
            return Ok(None);
        };
        // The Job's own worktree is long gone by the time anybody merges, so
        // the question is asked from the repository it was cut from, and about
        // the address rather than the branch — which the merge usually deleted.
        let landed = self.vcs().landed(&self.host().repo_root, &asking.url);
        if !landed.is_settled() {
            return Ok(None);
        }
        self.store()
            .lock()
            .await
            .record_landed(&asking.job_id, &landed)
            .map_err(Adrift::Writing)?;
        let noticed = Noticed {
            job: asking.job_id.clone(),
            landed,
        };
        // The row whole, for `JobStepAdvanced`'s reason: a client told only
        // that something happened would have to re-read the Job it was just
        // told about, which is the reload the event exists to stop.
        self.published_landing(&noticed).await?;
        Ok(Some(noticed))
    }

    /// The one Job to ask about this turn, or why there is not one.
    ///
    /// The clock is the injected one, so a test decides when a sweep comes due
    /// rather than waiting for it — the shape `machine_reading` already has.
    async fn due_to_ask(&self) -> Result<Option<store::Unsettled>, Adrift> {
        let now = self.now();
        let mut sweep = self.sweeping().lock().await;
        if let Some(last) = sweep.last.as_ref() {
            if elapsed(last, &now) < self.noticing().interval() {
                return Ok(None);
            }
        }
        // Stamped whether or not there is anything to ask about, so a board of
        // finished Jobs with no remote does not run this query four times a
        // second for the rest of the process's life.
        sweep.last = Some(now);
        let waiting = self
            .store()
            .lock()
            .await
            .pull_requests_unsettled()
            .map_err(Adrift::Reading)?;
        if waiting.is_empty() {
            return Ok(None);
        }
        let at = sweep.next % waiting.len();
        sweep.next = at + 1;
        Ok(waiting.into_iter().nth(at))
    }

    /// Tell whoever is watching, with the Job's row as it now stands.
    ///
    /// **Nothing is published where the state is not one of the two settled
    /// ones**, which is unreachable from [`notice_a_merge`](Fleet::notice_a_merge)
    /// and is written as a `let else` rather than a fallback so that a variant
    /// added to [`Landing`] later cannot quietly publish an empty address.
    async fn published_landing(&self, noticed: &Noticed) -> Result<(), Adrift> {
        let (Some(state), Landing::Merged { url } | Landing::ClosedUnmerged { url }) =
            (settled(&noticed.landed), &noticed.landed)
        else {
            return Ok(());
        };
        let job = self.load(&noticed.job).await?;
        // The row whole, for `JobStepAdvanced`'s reason: a client told only
        // that something happened would have to re-read the Job it was just
        // told about, which is the reload the event exists to stop. Built the
        // way that publish builds it — from the record alone, with no reason,
        // no queued reason and no slot, none of which a Job that finished some
        // time ago has anything to say about.
        let mut summary = ipc::JobSummary::from(&job);
        summary.landed = Some(state);
        self.publish(ipc::Event::JobLanded(ipc::JobLanded {
            job: summary,
            pull_request: url.clone(),
            settled: state,
            at: (&self.now()).into(),
        }));
        Ok(())
    }
}
