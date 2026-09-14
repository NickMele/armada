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
//! [`Room`] answers "may another Drone start **at all**" — the bound and
//! memory, about no Job in particular; a new machine-wide reason is a variant
//! there. **A reason belonging to one Job is a predicate of its own**:
//! [`clear_to_run`] for dependencies, `Fleet::overspent` for what it has spent,
//! `Fleet::frozen_by` for a freeze, asked where a Job is chosen rather than once
//! per admission. All four are
//! shared with `serving`'s `queued_reason`, so a Board cannot say a Job is
//! blocked while Fleet is starting it. **Disk is a fifth, of the same kind**:
//! [`Fleet::admit_next`] asks it per Job, of that Job's own repository's
//! volume rather than the machine's — `#987`.
//!
//! [`Fleet::next_queued`]: crate::Fleet

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, AdmissionHold, Component, Envelope, FieldValue, Job, JobId, JobStatus, Level, Target,
};
use store::Moved;

use crate::adrift::Adrift;
use crate::converging::elapsed;
use crate::coupling::{coupling, Coupling};
use crate::daemon::Fleet;
use crate::headroom::{Bytes, Reading, Short};
use crate::slots::Slots;
use crate::sub_dispatch::{children_standing, waiting_on_children};
use crate::superseding::{siblings_of, still_needed, Landed, StillNeeded};

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
    /// Disk per repository volume, asked lazily and cleared with `saw` — see
    /// [`Fleet::disk_free_at`].
    volumes: BTreeMap<PathBuf, Option<Bytes>>,
}

impl Polled {
    pub(crate) fn taken(at: core_model::Timestamp, saw: Option<Reading>) -> Polled {
        Polled {
            at,
            saw,
            volumes: BTreeMap::new(),
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
    /// Whether Fleet may start another Drone, of no Job in particular.
    ///
    /// **`get_capacity`'s alone now.** It bundles a disk reading of the one
    /// configured volume, which is the wrong question per Job — see
    /// [`Fleet::room_for`] and [`Fleet::volume_is_short`], which
    /// `admit_next` and `queued_reason` share instead.
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

    /// The bound and memory, with no Job's volume asked. **Shared by
    /// [`Fleet::admit_next`] and `queued_reason`**, both folded with
    /// [`Fleet::volume_is_short`], so the two can never disagree.
    pub(crate) async fn room_for(&self, slots: &mut Slots) -> Room {
        if !slots.room() {
            return Room::Bound;
        }
        match self.machine_reading().await {
            Some(reading) if reading.memory().spare() < self.headroom().memory_spare() => {
                Room::Machine(Short::Memory)
            }
            _ => Room::Yes,
        }
    }

    /// What the machine had left, read again only once the last reading is
    /// older than the poll interval.
    ///
    /// **The clock is the injected one**, so a test decides when a reading goes
    /// stale rather than waiting for it to. The reading itself is taken on a
    /// blocking thread: it is three processes and about eighty milliseconds,
    /// which is not something to hold a runtime worker for.
    pub(crate) async fn machine_reading(&self) -> Option<Reading> {
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
    /// **[`Fleet::room_for`] and [`Fleet::volume_is_short`]**, the same two
    /// `queued_reason` folds — a short volume skips its Job, not the pass.
    ///
    /// **The roster lock is held across the loop, and nothing else is.** Two
    /// admissions running at once would each read the same `queued` Job as next
    /// and dispatch it twice; holding the roster is what makes admission one act.
    /// It is released the moment the last Drone is spawned — a slot is held for
    /// as long as a Job is worked, and this for as long as one is *started*.
    ///
    /// Admission stops only at a real `dispatch` failure — never at a Job
    /// merely held back, which is skipped. The Job is left `escalated`.
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
        // Jobs this pass found short of their own repository's volume, so
        // `next_queued` does not hand the same one back — `#987`.
        let mut short_on_volume: Vec<JobId> = Vec::new();
        loop {
            if !self.room_for(&mut slots).await.granted() {
                break;
            }
            let Some(job) = self.next_queued(&short_on_volume).await? else {
                break;
            };
            let job_id = job.id().clone();
            // **Before the slot, not after.** A Job with nothing left to do
            // should never hold a place in the roster, and the reading is what
            // decides whether it has anything left. It is also the last moment
            // this can be asked: the next line commits a slot and the one after
            // it starts a Drone, and `crate::superseding` says why a Drone that
            // finds the work already done is not a source anybody may act on.
            if self.superseded_by_a_sibling(&job).await? {
                continue;
            }
            // Its own volume, not the machine's — skip it, not the pass.
            if self.volume_is_short(&job).await {
                short_on_volume.push(job_id);
                continue;
            }
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

    /// Whether the disk is short on this Job's own repository's volume —
    /// `served.root()`, where `WorktreeSpec` cuts one. Unserved is `false`.
    pub(crate) async fn volume_is_short(&self, job: &Job) -> bool {
        let Ok(served) = self.served_by(job) else {
            return false;
        };
        let volume = PathBuf::from(served.root());
        let floor = self.headroom().disk_floor();
        self.disk_free_at(&volume)
            .await
            .is_some_and(|free| free < floor)
    }

    /// Free bytes on `volume`, cached with the machine reading — one `df`
    /// per volume per poll interval, not one per queued Job. `#987`.
    async fn disk_free_at(&self, volume: &Path) -> Option<Bytes> {
        self.machine_reading().await;
        let mut polled = self.polled().lock().await;
        let entry = polled.as_mut().expect("machine_reading just set it");
        if let Some(cached) = entry.volumes.get(volume) {
            return *cached;
        }
        let machine = Arc::clone(self.machine());
        let owned = volume.to_path_buf();
        let key = owned.clone();
        let free = tokio::task::spawn_blocking(move || machine.disk_free_at(&owned))
            .await
            .ok()
            .flatten();
        entry.volumes.insert(key, free);
        free
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
    ///
    /// `short_on_volume` is this pass's own — Jobs already found short of
    /// their repository's disk, so a Job skipped once is not handed back.
    async fn next_queued(&self, short_on_volume: &[JobId]) -> Result<Option<Job>, Adrift> {
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
            if job.status() != JobStatus::Queued || short_on_volume.contains(job.id()) {
                continue;
            }
            // A frozen repository starts nothing, whoever put the Job here —
            // `crate::freezing`. First, because only a person lifts it.
            if !self.frozen_by(&job).is_empty() {
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

    /// Whether a Job from the same reading landed this one's work while it
    /// waited, and the Job was closed for it.
    ///
    /// **Answers `false` for almost every Job, and costs nothing to do so.** A
    /// Job with no `proposal_id`, or none whose siblings have landed, never
    /// reaches the call — which is every hand-entered Job, every sub-dispatch,
    /// and every proposal that stayed one Job. `crate::superseding` holds the
    /// reading itself and the rule that silence runs the Job.
    async fn superseded_by_a_sibling(&self, job: &Job) -> Result<bool, Adrift> {
        if job.proposal_id().is_none() {
            return Ok(false);
        }
        let (loaded, _) = self.every_job().await?;
        let landed: Vec<Landed> = {
            let store = self.store().lock().await;
            let mut found = Vec::new();
            for peer in siblings_of(job, loaded.jobs.iter()) {
                // **The sibling's own Evidence, and nothing derived.** What a
                // step claimed the work now does is written for exactly this
                // reader, one Job earlier than anybody expected it to be read.
                // A sibling that landed without submitting any is not evidence
                // of anything and is left out rather than guessed about.
                let Ok(evidence) = store.step_evidence(peer.id()) else {
                    continue;
                };
                let Some((_, latest)) = evidence.last() else {
                    continue;
                };
                found.push(Landed {
                    job_id: peer.id().clone(),
                    title: peer.title().as_str().to_string(),
                    claimed: latest.claimed.clone(),
                });
            }
            found
        };
        let asking = match self.proposing() {
            Ok(asking) => asking,
            // No proposer configured is no reading, and no reading runs the
            // Job. It is not a fault: a Fleet with no model is one where every
            // Job was hand-entered and none of them has a sibling.
            Err(_) => return Ok(false),
        };
        let StillNeeded::AlreadyLanded { by, because } = still_needed(job, &landed, &asking).await
        else {
            return Ok(false);
        };
        // **`Actor::Fleet`, and no reason column.** `job-statuses.toml` gives
        // `superseded` no reason storage — the status is the whole of what
        // happened — so what was read is put where a person will look for it,
        // which is the Job's own log.
        self.noted_superseded(job.id(), &by, &because);
        self.move_job(job, Target::Superseded, Actor::Fleet).await?;
        Ok(true)
    }

    /// Where a superseding says which Job landed the work, and why the reading
    /// thought so. **The only durable trace the call ever ran**, for the reason
    /// the status carries no reason of its own.
    fn noted_superseded(&self, job: &JobId, by: &JobId, because: &str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a Job from the same request had already landed this one's work, \
             and it was closed rather than dispatched",
        )
        .in_job(job.as_ulid().clone())
        .with_field("landed_by", FieldValue::Str(by.as_str().to_string()))
        .with_field("because", FieldValue::Str(because.to_string()));
        self.noted_in_the_log(job, &envelope);
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
