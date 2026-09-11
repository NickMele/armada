//! Noticing what became of a Job's pull request, and what follows.
//!
//! # Noticing, not deciding
//!
//! **Fleet may merge, and the decision is what stays a person's** — a press at
//! a `human_always` gate, `crate::merging`. This module is the other way one
//! settles: somebody merged it on the forge, only knowable by asking. Nothing
//! reached from here writes to a pull request; both methods it calls read.
//!
//! # One pull request per sweep, and never on the turn interval
//!
//! The loop ticks four times a second and asking the forge is a process. An
//! open pull request needs asking rarely and a merged one never again, so this
//! asks about **one** Job every [`Noticing`] interval and rotates: ten open at a
//! minute apiece is each asked every ten minutes, and a merge leaves for good.
//! The cost is one blocking call on the interval — `caught_up_onto`'s shape —
//! and a second on the turns that find one open, in `crate::under_review`.
//!
//! # What a merge moves, and what it does not
//!
//! `#337` named four outcomes. The record and the row shipped in `#360`; the
//! repository is brought up to what merged here; and `crate::proving` runs the
//! Checks against the tree that leaves — **keyed by the commit, not by this
//! Job**, since two Jobs merging in a minute are two merges into one commit.
//! Every question is asked from the repository root, never a Job's worktree.

use std::collections::BTreeMap;
use std::time::Duration;

use adapter_traits::{
    AgentHarness, Delivery, Landing, Mergeable, Rendering, Renewed, RepositoryStanding,
    UnderReview, Vcs, WhatBecameOfIt, WorkProduct,
};
use core_model::{Component, Envelope, FieldValue, JobId, Level, Timestamp};

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
    /// Where the repository every worktree is cut from was left.
    ///
    /// **`None` on a close that was not a merge**, which is the whole of the
    /// rule: nothing arrived on the base branch, so there is nothing for the
    /// repository to catch up to.
    pub repository: Option<RepositoryStanding>,
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
    /// Every pull request this process has closed and reopened, and how that
    /// went.
    ///
    /// **Two rules live in this one map.** A pull request is nudged once, so a
    /// base this cannot re-pin does not close and reopen a person's work every
    /// sweep for the life of the process. And a pull request left closed by a
    /// reopen that failed is not read as somebody turning the work down — it is
    /// this sweep's own leavings, and the next sweep tries the reopen again
    /// rather than recording it.
    pub(crate) nudged: BTreeMap<String, Renewed>,
    /// What the forge last said about each open pull request, so a line is
    /// written when it changes rather than on every rotation. See
    /// [`crate::under_review`], which owns the comparison and the reasoning.
    pub(crate) reviewed: crate::under_review::Standings,
    /// Every pull request this process has already merged for itself under
    /// `auto_merge`, whether or not the forge accepted.
    ///
    /// **`nudged`'s shape and its reason**, one act along: a merge the forge
    /// will never accept — a protected base, a required review nobody gave —
    /// must not spawn a process and write a line every sweep for the life of
    /// the daemon. A person can still press, and a restart tries once more.
    ///
    /// **A set and not a map**, unlike `nudged`: what came of it is on the
    /// Job's record and in its log already, and a second copy here would be a
    /// reading that outlived the fact.
    pub(crate) merged_by_policy: std::collections::BTreeSet<String>,
    /// What this rotation last read live off each open pull request, keyed by
    /// address. `crate::serving::get_job` is the one reader, and it never
    /// asks the forge itself — see [`ipc::JobDelivery::pull_request_detail`].
    ///
    /// **Cleared the moment a pull request settles**, in `settled_landing`,
    /// so presence here always means "still open, as of the last rotation":
    /// nothing that reads this has to compare it against the record's own
    /// `landed` to tell a live reading from a stale one.
    pub(crate) pr_detail: BTreeMap<String, ipc::PullRequestDetail>,
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
        let read = self.vcs().landed(&self.host().repo_root, &asking.url);
        match &read.landing {
            // Still open, so the merge question has no news — and the second
            // question this call answers does. `#427`: the forge pins the
            // comparison at the commit the pull request was opened from, and a
            // base that has moved since renders other people's commits as this
            // Job's work.
            Landing::Open { url, rendering } => {
                self.nudged(&asking.job_id, url, rendering).await;
                // **The one state in which the second question means
                // anything**, on the turn the rotation had already reached this
                // Job — never a loop of its own. `crate::under_review`.
                let reviewed = self.read_what_is_under_review(&asking.job_id, url).await;
                // **Cached on the same turn that read it**, so `get_job` never
                // asks the forge itself — see [`Sweep::pr_detail`].
                self.remembered(url, &read, reviewed.as_ref()).await;
                return Ok(None);
            }
            Landing::Unknown => return Ok(None),
            // **This sweep's own leavings, not a person turning the work
            // down.** A reopen that failed left it closed, and recording that
            // would put the wrong sentence on the record for good — so the
            // reopen is tried again instead.
            Landing::ClosedUnmerged { url } if self.left_it_closed(url).await => {
                self.reopened(url).await;
                return Ok(None);
            }
            Landing::Merged { .. } | Landing::ClosedUnmerged { .. } => {}
        }
        self.settled_landing(&asking.job_id, read).await
    }

    /// What a settled pull request comes to: the record, the repository every
    /// worktree is cut from, the Checks over what merged, and the row.
    ///
    /// **One path, whether Fleet noticed the merge or a person pressed for
    /// it.** `crate::merging` is the second way into it, and it reaches here
    /// rather than doing the four again — a press with its own record would be
    /// a second place the same commit is caught up to and proved, and the
    /// dedupe `crate::proving` keys by the commit would be asked from two
    /// directions.
    ///
    /// **`None` on anything that has not settled**, which no caller reaches:
    /// the sweep answers the three unsettled readings above and a press reads
    /// the forge straight after merging. It is a `let else` rather than an
    /// assumption for `published_landing`'s reason — a variant added to
    /// [`Landing`] later must not quietly record the absence of news.
    pub(crate) async fn settled_landing(
        &self,
        job: &JobId,
        read: WhatBecameOfIt,
    ) -> Result<Option<Noticed>, Adrift> {
        if !read.landing.is_settled() {
            return Ok(None);
        }
        let landed = read.landing;
        // **Before the record is written, and unconditionally on this path.**
        // Both variants that pass `is_settled` name the address, and a live
        // reading left behind under it would answer `get_job` with "still
        // open" about a pull request that just stopped being one.
        if let Landing::Merged { url } | Landing::ClosedUnmerged { url } = &landed {
            self.sweeping().lock().await.pr_detail.remove(url);
        }
        self.store()
            .lock()
            .await
            .record_landed(job, &landed)
            .map_err(Adrift::Writing)?;
        let repository = match (&landed, read.base.as_deref()) {
            // **Only a merge, and only where the forge named the branch.**
            // What merged is now what everything else builds on — `#337` — and
            // a pull request that was closed put nothing on the base at all.
            (Landing::Merged { .. }, Some(base)) => {
                let standing = self.caught_the_repository_up(job, base).await;
                // **After the fast-forward and never instead of it**, because
                // what is proved is the tree the fast-forward left: a standing
                // that declined has no commit to hand over and cannot start a
                // run. `#474`, and `crate::proving` for why nothing waits on it.
                self.began_proving(job, &standing).await;
                Some(standing)
            }
            _ => None,
        };
        let noticed = Noticed {
            job: job.clone(),
            landed,
            repository,
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

    /// Ask the forge to compare an open pull request afresh, where its base has
    /// been superseded and this process has not already asked.
    ///
    /// **Nothing is returned and nothing raises.** The Job is finished and its
    /// record says everything it is going to say; what this changes is what a
    /// person is shown on the forge, which is not a fact Armada holds. A forge
    /// that would not do it is a log line and another sweep.
    ///
    /// **Once per pull request, per process.** Closing and reopening is visible
    /// to everybody watching it, so a base this cannot re-pin must not do that
    /// on a loop. The memory is [`Sweep::nudged`], which is lost on a restart —
    /// costing at most one more nudge over a set that is small by construction.
    async fn nudged(&self, job: &JobId, url: &str, rendering: &Rendering) {
        let Rendering::FromASupersededBase { pinned, written_on } = rendering else {
            return;
        };
        if self.sweeping().lock().await.nudged.contains_key(url) {
            return;
        }
        let renewed = self.reopened(url).await;
        let level = match &renewed {
            Renewed::Renewed => Level::Info,
            Renewed::LeftClosed { .. } => Level::Warn,
        };
        self.logged(
            job,
            Envelope::new(
                self.now(),
                level,
                Component::Fleet,
                self.run().clone(),
                match &renewed {
                    Renewed::Renewed => {
                        "the pull request was comparing against a base that had \
                         moved, and was reopened against the right one"
                    }
                    Renewed::LeftClosed { .. } => {
                        "the pull request was closed to re-pin its base and the \
                         forge would not reopen it"
                    }
                },
            )
            .in_job(job.as_ulid().clone())
            .with_field("pull_request", FieldValue::Str(url.to_string()))
            .with_field("pinned_at", FieldValue::Str(pinned.clone()))
            .with_field("written_on", FieldValue::Str(written_on.clone())),
        );
    }

    /// Close and reopen, and remember how it went.
    ///
    /// **The one write to the forge on this path**, and the reason
    /// [`Sweep::nudged`] exists at all: what it records is not an optimisation
    /// but the guard that stops a pull request this left closed being read as
    /// one somebody turned down.
    async fn reopened(&self, url: &str) -> Renewed {
        let renewed = self.vcs().rendered_afresh(&self.host().repo_root, url);
        self.sweeping()
            .lock()
            .await
            .nudged
            .insert(url.to_string(), renewed.clone());
        renewed
    }

    /// Remember what this turn read live off one open pull request, so
    /// `get_job` can answer from memory rather than asking the forge itself.
    ///
    /// **Every turn that reaches here writes, whether or not anything
    /// changed.** The read already happened for [`notice_a_merge`]'s own
    /// reasons; keeping the answer costs nothing further and is what lets
    /// `mergeable` flip from a forge still computing it to a real answer
    /// without waiting for something else about the pull request to move
    /// too.
    async fn remembered(&self, url: &str, read: &WhatBecameOfIt, reviewed: Option<&UnderReview>) {
        let detail = ipc::PullRequestDetail {
            number: read.number,
            title: read.title.clone(),
            mergeable: match read.mergeable {
                Mergeable::Yes => Some(true),
                Mergeable::No => Some(false),
                Mergeable::Unreadable => None,
            },
            reviews: reviewed
                .map(|reviewed| {
                    reviewed
                        .verdicts
                        .iter()
                        .map(|reviewed_by| ipc::ReviewedBy {
                            by: reviewed_by.by.as_written().to_string(),
                            verdict: reviewed_by.verdict.kind().to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        };
        self.sweeping()
            .lock()
            .await
            .pr_detail
            .insert(url.to_string(), detail);
    }

    /// Whether this process closed that pull request and could not reopen it.
    async fn left_it_closed(&self, url: &str) -> bool {
        matches!(
            self.sweeping().lock().await.nudged.get(url),
            Some(Renewed::LeftClosed { .. })
        )
    }

    /// Bring the repository every worktree is cut from up to what just merged.
    ///
    /// **One Job at a time, for `caught_up_onto`'s reason.** This writes into
    /// the one `.git` every worktree shares, so it takes `Fleet::merge_end`
    /// rather than racing a rebase that is already holding it.
    ///
    /// **Never a gate and never a failure.** The work is already merged, so
    /// nothing about the Job can turn on this — a repository a person is
    /// standing in with uncommitted work is left alone and said so.
    async fn caught_the_repository_up(&self, job: &JobId, base: &str) -> RepositoryStanding {
        let _at_the_merge_end = self.merge_end().lock().await;
        let standing = self
            .vcs()
            .caught_the_repository_up(&self.host().repo_root, base);
        let (level, wording) = match &standing {
            RepositoryStanding::MovedOn { .. } => (
                Level::Info,
                "the repository was brought up to the branch that merged",
            ),
            RepositoryStanding::AlreadyHadIt { .. } => (
                Level::Info,
                "the repository already had the branch that merged",
            ),
            RepositoryStanding::LeftAlone { .. } => (
                Level::Info,
                "the repository was left where it was after the merge",
            ),
        };
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            wording,
        )
        .in_job(job.as_ulid().clone())
        .with_field("base", FieldValue::Str(base.to_string()));
        envelope = match &standing {
            RepositoryStanding::MovedOn { commits, .. } => {
                envelope.with_field("commits", FieldValue::Int(*commits as i64))
            }
            RepositoryStanding::LeftAlone { why } => {
                envelope.with_field("left_alone", FieldValue::Str(why.clone()))
            }
            RepositoryStanding::AlreadyHadIt { .. } => envelope,
        };
        self.logged(job, envelope);
        standing
    }

    /// A line in the Job's own log. **A log line that will not write does not
    /// stop anything**, for `converging::noted`'s reason — and here there is
    /// not even a step left to stop: the Job finished before any of this ran.
    ///
    /// `pub(crate)` for its second caller, `crate::under_review`, which runs on
    /// this sweep and writes into the same Job's log for the same reason.
    pub(crate) fn logged(&self, job: &JobId, envelope: Envelope) {
        self.noted_in_the_log(job, &envelope);
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
