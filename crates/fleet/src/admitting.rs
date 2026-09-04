//! Which approved Job runs next, and how many may run at once.
//!
//! Split from [`dispatch`](mod@crate::dispatch), which is what happens to a Job
//! once the answer is yes. They were one file while the answer was "the slot is
//! free"; the bound, the queue's ordering and the dependency release together
//! are a subject, and it is the one `#44`, `#48` and `#51` each arrive at.
//!
//! # The queue is a status, not a structure
//!
//! `queued` is the queue. There is no ordering held in memory that a restart
//! could lose or that could disagree with the log, so [`Fleet::next_queued`]
//! reads the board and sorts by the sequence of the approving event.
//!
//! # One predicate per question, and every reason to refuse belongs inside one
//!
//! [`Room`] answers "may another Drone start **at all**" — the machine and the
//! roster, about no Job in particular; a new machine-wide reason is a variant
//! there. **A reason belonging to one Job is a predicate of its own**:
//! [`clear_to_run`] for dependencies, `Fleet::overspent` for what it has spent,
//! asked where a Job is chosen rather than once per admission. All three are
//! shared with `serving`'s `queued_reason`, so a Board cannot say a Job is
//! blocked while Fleet is starting it.
//!
//! [`Fleet::next_queued`]: crate::Fleet

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{AdmissionHold, Job, JobId, JobStatus};
use store::Moved;

use crate::adrift::Adrift;
use crate::converging::elapsed;
use crate::coupling::{coupling, Coupling};
use crate::daemon::Fleet;
use crate::headroom::{Reading, Short};
use crate::slots::Slots;
use crate::sub_dispatch::{children_standing, waiting_on_children};

/// Whether Fleet may start another Drone, and what stops it where it may not.
///
/// **Every variant but the first folds to `waiting_on_resources`** on the
/// Board, which is the only label `job-statuses.toml` gives a `queued` Job
/// short of anything. The distinction between them is the operator's.
///
/// # Which one is short now reaches a person, through [`Room::hold`]
///
/// `queued_reason` still reduces this to [`Room::granted`]. What changed is
/// that `get_capacity` serves this value unreduced, beside `Slots::cap` and
/// `Slots::count` — the three things this doc asked for, and no new read.
///
/// **The order is still the catch, and it is deliberately not relaxed.** The
/// bound is asked first so a Fleet at its cap pays nothing for a reading, which
/// makes `Bound` and `Machine` exclusive: "the cap is spent *and* the disk is
/// full" is not a state this can report. What is served is what stops admission
/// now, and the next thing is served once that clears. Relaxing the order would
/// put three processes behind every read of a Fleet that is already full.
///
/// Doctor's System stats panel, which `settings.toml`'s headroom row names as
/// the other reader of these numbers, does not exist either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Room {
    /// There is room.
    Yes,
    /// The concurrency bound is spent. `settings.concurrency-cap`.
    Bound,
    /// The machine has too little of something. `crate::headroom`.
    Machine(Short),
}

impl Room {
    pub(crate) fn granted(&self) -> bool {
        matches!(self, Room::Yes)
    }

    /// The one thing holding the next Drone back, in the registry's own
    /// spelling, or `None` where nothing is.
    ///
    /// **The domain enum, not a word written here.** `enum-verbs.toml` carries
    /// the verb each of these renders as, and a spelling minted in this file
    /// would be the second vocabulary the registry exists to prevent. A new
    /// variant of [`Room`] adds an arm here and a row there, and nothing else —
    /// `ipc::FleetCapacity` carries the spelling rather than a closed set, so
    /// the wire does not move for a fifth reason.
    pub(crate) fn hold(&self) -> Option<AdmissionHold> {
        match self {
            Room::Yes => None,
            Room::Bound => Some(AdmissionHold::ConcurrencyBound),
            Room::Machine(Short::Cpu) => Some(AdmissionHold::Cpu),
            Room::Machine(Short::Memory) => Some(AdmissionHold::Memory),
            Room::Machine(Short::Disk) => Some(AdmissionHold::Disk),
        }
    }
}

/// The last machine reading and when it was taken, whether or not it read.
///
/// **A failed reading is recorded too.** Without that, a machine that will not
/// answer is asked again on every admission and every Board row, which is three
/// processes per ask; with it, a failure costs no more than a success.
pub(crate) struct Polled {
    at: core_model::Timestamp,
    saw: Option<Reading>,
}

impl Polled {
    pub(crate) fn taken(at: core_model::Timestamp, saw: Option<Reading>) -> Polled {
        Polled { at, saw }
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
    /// Whether Fleet may start another Drone right now.
    ///
    /// **The one predicate.** [`Fleet::admit_next`] opens with it and
    /// `serving`'s `queued_reason` answers `waiting_on_resources` from it, so
    /// there is no second answer for a Board to disagree with admission by.
    ///
    /// **The bound is asked first because it is free.** `Slots::room` reads a
    /// map; the machine reading costs three processes. A Fleet already at its
    /// cap never pays for a reading it would not have acted on.
    ///
    /// The roster is passed in rather than taken, because `admit_next` is
    /// already holding it — taking it here would deadlock, and the lock order
    /// `crate::slots` states is roster first and everything else after.
    pub(crate) async fn room_for_another(&self, slots: &mut Slots) -> Room {
        if !slots.room() {
            return Room::Bound;
        }
        match self
            .machine_reading()
            .await
            .and_then(|reading| self.headroom().short_of(&reading))
        {
            Some(short) => Room::Machine(short),
            None => Room::Yes,
        }
    }

    /// What the machine had left, read again only once the last reading is
    /// older than the poll interval.
    ///
    /// **The clock is the injected one**, so a test decides when a reading goes
    /// stale rather than waiting for it to. The reading itself is taken on a
    /// blocking thread: it is three processes and about eighty milliseconds,
    /// which is not something to hold a runtime worker for.
    async fn machine_reading(&self) -> Option<Reading> {
        let now = self.clock().now();
        let mut polled = self.polled().lock().await;
        if let Some(last) = polled.as_ref() {
            if elapsed(&last.at, &now) < self.polling().interval() {
                return last.saw;
            }
        }
        let machine = Arc::clone(self.machine());
        let saw = tokio::task::spawn_blocking(move || machine.read())
            .await
            .ok()
            .flatten();
        *polled = Some(Polled::taken(now, saw));
        saw
    }

    /// Start every approved Job there is room for.
    ///
    /// **[`Fleet::room_for_another`] is the whole of what "room" means here**,
    /// and it is the same predicate `queued_reason` answers
    /// `waiting_on_resources` from — one answer, because a Board saying a Job is
    /// blocked while Fleet is starting it is worse than a Board saying nothing.
    ///
    /// **The roster lock is held across the loop, and nothing else is.** Two
    /// admissions running at once would each read the same `queued` Job as next
    /// and dispatch it twice; holding the roster is what makes admission one act.
    /// It is released the moment the last Drone is spawned — a slot is held for
    /// as long as a Job is worked, and this for as long as one is *started*.
    ///
    /// Admission stops at the first Job that will not start. The failure is
    /// returned, its Job is left `escalated` by `dispatch`, and the next turn
    /// asks again — Fleet does not walk on down the queue to find one that works,
    /// because the reason the first failed is ordinarily the disk.
    ///
    /// **Never from a `Commands` method, and none does** — `#428`, then `#456`
    /// for the six that freed a place and filled it in one breath. This runs a
    /// whole [`crate::dispatch`], so on a handler's future a client that stops
    /// waiting takes the command and its timeout away together; where the command
    /// freed a place, the Job that then never starts is the *next* one in the
    /// queue and nobody is watching it. [`Fleet::turn`] and [`Fleet::reconcile`]
    /// are the only callers.
    pub(crate) async fn admit_next(&self) -> Result<Vec<JobId>, Adrift> {
        let mut slots = self.slots().lock().await;
        let mut admitted = Vec::new();
        while self.room_for_another(&mut slots).await.granted() {
            let Some(job) = self.next_queued().await? else {
                break;
            };
            let job_id = job.id().clone();
            let slot = slots.opened_for(&job_id);
            // The slot exists and counts against the bound before the dispatch
            // that fills it runs, so a `next_queued` inside this same loop
            // cannot hand back the Job being started. Nothing else can be
            // holding this lock — the roster is held, and the Job had none.
            let mut working = slot.lock().await;
            if let Err(cause) = self.dispatch(job, &mut working).await {
                drop(working);
                // Nothing started, so nothing is being worked, so the bound is
                // not spent. Left in place it would be a Job with no Drone
                // holding a place in the roster for ever.
                slots.closed(&job_id);
                return Err(cause);
            }
            drop(working);
            admitted.push(job_id);
        }
        Ok(admitted)
    }

    /// The Job that has been waiting longest, by when it was approved.
    ///
    /// Ordered by the sequence of the event that put it at `queued`, not by id
    /// and not by creation: two Jobs created in either order can be approved in
    /// the other, and the person who approved first is entitled to expect
    /// theirs to run first.
    ///
    /// A row that would not rebuild is not dispatchable, and is reported by
    /// every read that returns a list rather than being silently completed
    /// here.
    async fn next_queued(&self) -> Result<Option<Job>, Adrift> {
        let (loaded, _) = self.every_job().await?;
        let standing: BTreeMap<JobId, JobStatus> = loaded
            .jobs
            .iter()
            .map(|job| (job.id().clone(), job.status()))
            .collect();
        // Read before the loop consumes the board, because every waiting Job
        // asks the same question of it.
        let children = children_standing(&loaded.jobs);
        let mut waiting = Vec::new();
        for job in loaded.jobs {
            if job.status() != JobStatus::Queued {
                continue;
            }
            // Approved and still waiting on a peer. It is skipped rather than
            // reordered — the queue is by approval and this is not a Job's turn
            // being taken, it is a Job that has nothing to work against yet.
            if !clear_to_run(&job, &standing) {
                continue;
            }
            // Approved, and waiting on Jobs it created rather than on peers.
            // **The wait that frees the slot instead of holding it**: this Job
            // gave its Drone up so its children could have one, and starting it
            // again before they finish is the deadlock that stand-down avoids
            // arriving one turn later. `crate::sub_dispatch`.
            //
            // Beside `clear_to_run` and before the budget for the same reason
            // it is: both are a Job whose turn has not come, and neither has
            // spent anything on this attempt.
            if waiting_on_children(&job, &children) {
                continue;
            }
            // Approved and already past what it may spend. **Skipped and not
            // escalated**: nothing has gone wrong with the work, and what
            // clears this is a person raising the cap rather than a person
            // ruling on the Job. It stays `queued`, and the Board says
            // `over_budget` from this same call — see `crate::allowance`, and
            // `QueuedReason::OverBudget` for why a reason that only a person
            // can clear is still a reason to wait.
            //
            // It is asked after `clear_to_run` because a Job blocked on an
            // upstream has not spent anything yet on this attempt, and the
            // dependency is the older fact.
            if self.overspent(&job).await?.is_some() {
                continue;
            }
            waiting.push((self.approved_at(job.id()).await?, job));
        }
        waiting.sort_by_key(|(seq, _)| *seq);
        Ok(waiting.into_iter().next().map(|(_, job)| job))
    }

    /// When the Job was released to run, as the log's own sequence.
    ///
    /// A Job with no such event was **created** at `queued` — a sub-dispatch,
    /// approved as part of its parent — and is therefore older than anything
    /// that had to be approved on its own.
    async fn approved_at(&self, job_id: &JobId) -> Result<i64, Adrift> {
        let events = self
            .store()
            .lock()
            .await
            .events_for(job_id)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        Ok(events
            .iter()
            .rev()
            .find(|event| {
                matches!(
                    event.moved(),
                    Moved::Job {
                        to: JobStatus::Queued,
                        ..
                    }
                )
            })
            .map(|event| event.seq())
            .unwrap_or(i64::MIN))
    }
}

/// Whether every Job this one waits on has finished, and finished well.
///
/// `pub(crate)` because `serving` renders the same fact as a label. **One
/// answer, not two** — a Board saying a Job is blocked while admission
/// disagrees is worse than a Board saying nothing.
///
/// **No terminal weaker than `completed_success` or `superseded`.** A dependent
/// admitted after a failed upstream would do its work against a base that never
/// landed, which is the half-landed upstream the linked-DAG shape exists to
/// prevent. A superseded upstream is not that case: the work landed outside the
/// Job, so the base is there and only the record has nothing to say.
///
/// The three-way weighing, and which peer failed, are
/// [`coupling`](crate::coupling::coupling)'s — this is the yes-or-no admission
/// reads, and it is the same call `serving` labels a Board row from.
pub(crate) fn clear_to_run(job: &Job, standing: &BTreeMap<JobId, JobStatus>) -> bool {
    matches!(coupling(job, standing), Coupling::Clear { .. })
}
