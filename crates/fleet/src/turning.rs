//! One turn of the loop, what it came to, and the thing that asks for one every
//! quarter of a second.
//!
//! # A tick, because two of the three things a turn does have no sender
//!
//! A turn rules on evidence that landed, reaps a Drone that is gone and admits
//! the next approved Job. A Drone exiting is a process event nothing here
//! awaits, and a submission arrives on the Evidence tool, which has no server in
//! front of it at M1 — so waking on a signal means inventing senders for events
//! that have none. A tick needs none and is correct for all three.
//!
//! It is cheap where it matters: a turn over a Job being worked reads no store,
//! and admission does not wait on the tick — `approve` dispatches inline — so
//! this interval is the latency of a **ruling** rather than of a start.
//!
//! # A Check that outlasts a tick
//!
//! This loop awaits each turn, so a long Check stacks nothing: the tick it ran
//! through is dropped. **What it holds is one Job's slot and not Fleet's**, so a
//! quarter of an hour of `cargo nextest` holds up that Job's own kills and its
//! own Drone's tool calls and nothing else's. What it still delays is every
//! Job's next *turn* — this is one task walking the roster in order — which is
//! `#50`'s remaining cost and is not measured. A failed turn is reported and
//! ticking continues either way: one turn's fault is not every later Job's.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::JobId;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

use crate::adrift::Adrift;
use crate::converging::Wandering;
use crate::daemon::Fleet;
use crate::delivery::Delivered;
use crate::drone::Aftermath;
use crate::evidence::Decline;
use crate::gate::Ruling;
use crate::resume::Roused;
use crate::scope::Drifting;
use crate::silence::Quiet;
use crate::working::Working;

/// What one turn of the loop did for one working Drone. Every field is
/// ordinarily empty.
#[derive(Debug)]
pub struct Worked {
    /// Whose turn this was.
    pub job: JobId,
    /// The gate's answer to a submission that had landed.
    pub ruled: Option<Ruling>,
    /// The gate having been asked and refused. **Never present beside
    /// `ruled`**, and carried rather than swallowed: an absence is exactly what
    /// a person cannot tell from a Judge still thinking.
    pub declined: Option<Decline>,
    /// What followed from a Drone that had gone.
    pub after: Option<Aftermath>,
    /// What became of a finished Job's branch. Present on the turn that
    /// finished one and empty on every other, like the two fields above it.
    pub delivered: Option<Delivered>,
    /// Work seen outside the step's declared scope, on the turn it was first
    /// seen. **It fails nothing here** — the Drone may declare again, and the
    /// gate reads the footprint for itself when the step ends.
    pub drifting: Option<Drifting>,
    /// How far the thrashing chain got with the step being worked. Empty on
    /// every turn of a step inside its norms, which is nearly all of them.
    pub wandering: Option<Wandering>,
    /// What the liveness vigil did about a Drone that had stopped speaking.
    /// Empty on every turn of a Drone that is speaking, which is nearly all of
    /// them.
    pub quiet: Option<Quiet>,
    /// The Job that came back to `running` because its Drone took a turn after
    /// a person redirected it. Empty on every turn no redirect is outstanding
    /// on, which is nearly all of them.
    pub roused: Option<Roused>,
}

impl Worked {
    pub(crate) fn on(job: JobId) -> Worked {
        Worked {
            job,
            ruled: None,
            declined: None,
            after: None,
            delivered: None,
            drifting: None,
            wandering: None,
            quiet: None,
            roused: None,
        }
    }
}

/// What one turn of the loop did, over every Drone that was working.
///
/// **A list, because a turn is no longer about one Job.** It was a flat struct
/// of options while there was one slot, and the fields read as facts about
/// Fleet; they are facts about a Drone, and this is where that stopped being a
/// distinction without a difference.
#[derive(Debug, Default)]
pub struct Turned {
    /// One entry per Drone that was in the roster when the turn began, in Job
    /// id order. Empty on an idle turn.
    pub each: Vec<Worked>,
    /// The Jobs admitted because the bound had room. Ordinarily empty, and
    /// longer than one only where several came free at once.
    pub admitted: Vec<JobId>,
    /// Jobs escalated because an upstream they wait on ended badly. **A list
    /// and not an `Option`**: one upstream ending releases every dependent that
    /// named it, so the turn a Job fails is the turn all of them move.
    ///
    /// **On the turn and not on a [`Worked`]**, which is where `#50` put it
    /// back. A stranded dependent is `queued` and has no Drone, so it has no
    /// slot for the walk to have visited — and the walk itself reads the whole
    /// board once rather than once per working Job.
    pub stranded: Vec<JobId>,
}

impl Turned {
    /// The gate's answer, from whichever Drone produced one.
    ///
    /// **The convenience a one-Drone Fleet reads by**, and never what a surface
    /// should take: with the bound above one there may be a second, and this
    /// answers about the first. Every reader that has to be right about which
    /// Job walks [`Turned::each`], which carries the Job on every entry.
    pub fn ruled(&self) -> Option<&Ruling> {
        self.each.iter().find_map(|worked| worked.ruled.as_ref())
    }

    /// The gate having been asked and refused. See [`Turned::ruled`].
    pub fn declined(&self) -> Option<&Decline> {
        self.each.iter().find_map(|worked| worked.declined.as_ref())
    }

    /// What followed from a Drone that had gone. See [`Turned::ruled`].
    pub fn after(&self) -> Option<&Aftermath> {
        self.each.iter().find_map(|worked| worked.after.as_ref())
    }

    /// What became of a finished Job's branch. See [`Turned::ruled`].
    pub fn delivered(&self) -> Option<&Delivered> {
        self.each
            .iter()
            .find_map(|worked| worked.delivered.as_ref())
    }

    /// Work seen outside the declared scope. See [`Turned::ruled`].
    pub fn drifting(&self) -> Option<&Drifting> {
        self.each.iter().find_map(|worked| worked.drifting.as_ref())
    }

    /// How far the thrashing chain got. See [`Turned::ruled`].
    pub fn wandering(&self) -> Option<&Wandering> {
        self.each
            .iter()
            .find_map(|worked| worked.wandering.as_ref())
    }

    /// What the liveness vigil did. See [`Turned::ruled`].
    pub fn quiet(&self) -> Option<&Quiet> {
        self.each.iter().find_map(|worked| worked.quiet.as_ref())
    }

    /// The Job a redirect woke. See [`Turned::ruled`].
    pub fn roused(&self) -> Option<&Roused> {
        self.each.iter().find_map(|worked| worked.roused.as_ref())
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
    /// One turn: settle what landed, reap a Drone that is gone, admit the next
    /// approved Jobs if the bound has room.
    ///
    /// **Not a scheduler.** It runs the three things that can follow from the
    /// world having moved, in the one order they can follow in, over each slot
    /// in turn — and the order *within* a slot is unchanged, because every
    /// argument for it was about one Drone.
    pub async fn turn(&self) -> Result<Turned, Adrift> {
        let turned = self.turning().await;
        // **Into the Job's own log**, not only onto the reporter the loop was
        // given. That reporter is Fleet's stdout, which is the operator's
        // console and is not where anybody reads a Job — so a turn that failed
        // showed a person exactly what a decline that wrote nothing showed
        // them, which is nothing at all.
        if let Err(why) = &turned {
            self.noted_adrift(why);
        }
        turned
    }

    async fn turning(&self) -> Result<Turned, Adrift> {
        // The roster is read once and let go before any slot is taken, which is
        // the order `crate::slots` states: a Drone's tool call must not wait on
        // the roster for as long as another Drone's Check runs.
        let each = self.slots().lock().await.each();
        let mut turned = Turned::default();
        for (job, slot) in each {
            // A Job whose slot is held by its own Drone's tool call right now.
            // Waiting is right — it is that Job's own turn — and it blocks no
            // other Job, which is the whole of what the per-slot lock buys.
            let mut working = slot.lock().await;
            turned.each.push(self.turning_one(job, &mut working).await?);
        }
        // Below every slot, and outside all of them: a submission whose Job is
        // in no slot has no slot to be settled under. See `crate::settling`.
        self.stranded_submissions(&mut turned).await?;
        // After everything that can end a Job this turn and before admission:
        // a dependent whose upstream just failed must not be considered for a
        // slot it can never use. **Once for the turn, not once per Drone** — it
        // reads the whole board, and a walk per working Job would read it twice
        // to reach the same answer.
        turned.stranded = self.strand_dependents().await?;
        turned.admitted = self.admit_next().await?;
        Ok(turned)
    }

    /// The eight watchers, over one Drone's slot. **The order is the one the
    /// single slot had**, and each argument for it is about the Drone rather
    /// than about Fleet, which is why none of them needed rewriting.
    async fn turning_one(
        &self,
        job: JobId,
        working: &mut Option<Working>,
    ) -> Result<Worked, Adrift> {
        let mut worked = Worked::on(job.clone());
        // First, because the reading it takes is the one the drift check needs
        // and a turn must not open the same repository twice. It answers `None`
        // on the turns it declines to read, and the drift check then reads for
        // itself exactly as it did before this existed.
        let footprint = self.watch_footprint(working).await;
        // Before the gate, so a step whose evidence lands this turn has its
        // last live reading taken while its Drone is still the one being
        // watched — and after nothing, because the check reads a worktree and
        // must not run against a slot the gate has just cleared.
        let drifting = self.watch_scope(working, footprint.as_ref()).await;
        // **Before the vigil, because they are one question in the two
        // directions** and the answers must not be read out of order: a Drone
        // that has answered a redirect is a Drone that is speaking, and a Job
        // still `escalated` is one the vigil declines to measure at all. This
        // is what puts it back under the clock.
        let roused = self.watch_redirect(working).await?;
        // **Before the thrashing chain, because it is cheaper and more
        // specific.** A Drone that has stopped speaking is not thrashing, and
        // the chain's first stage costs a Judge call — so asking the free
        // question first is what stops Fleet paying a model to look at the work
        // of a Drone that is no longer doing any.
        let quiet = self.watch_silence(working).await?;
        // After the drift reading it consumes and before the gate, which is the
        // one place both are true: a step whose evidence lands this turn is at
        // the gate rather than thrashing, and `settle` may clear the slot.
        let wandering = self.watch_convergence(working).await?;
        let settled = self.settle(working).await?;
        worked.delivered = self.take_delivered(&job).await;
        worked.after = self.reap(working).await?;
        worked.ruled = settled.ruled;
        worked.declined = settled.declined;
        worked.drifting = drifting;
        worked.wandering = wandering;
        worked.quiet = quiet;
        worked.roused = roused;
        Ok(worked)
    }
}

/// Turn `fleet` every `every`, reporting a failed turn to `adrift`.
///
/// The Fleet is taken as an `Arc` and never by value: **the caller is expected
/// to still hold one**, because the other holder is the router, and a Fleet
/// only one of them can have is either unserved or undriven.
pub fn keep_turning<H, V, W>(
    fleet: Arc<Fleet<H, V, W>>,
    every: Duration,
    adrift: impl Fn(Adrift) + Send + 'static,
) -> Turning
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let stop = Arc::new(Notify::new());
    let asked = Arc::clone(&stop);
    let handle = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(every);
        // The tick that would have fired during a long Check is dropped rather
        // than fired late in a burst. A burst would run turns back to back over
        // a Fleet that had just done all three of the things a turn does.
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                // A stop asked for during a turn is not lost: `notify_one`
                // leaves a permit, and this arm takes it on the next pass.
                _ = asked.notified() => return,
                _ = ticker.tick() => {}
            }
            if let Err(why) = fleet.turn().await {
                adrift(why);
            }
        }
    });
    Turning {
        stop,
        handle: Some(handle),
    }
}

/// The loop, as a thing that can be stopped — or dropped.
///
/// Held by whoever composed the process. Dropping it stops the turning, which
/// is the property that makes an undriven Fleet hard to write by accident: a
/// caller that discards this has visibly discarded the loop.
pub struct Turning {
    stop: Arc<Notify>,
    /// `None` only after [`Turning::stopped`] has taken it.
    handle: Option<JoinHandle<()>>,
}

impl Turning {
    /// Stop between turns, letting the turn in flight finish.
    ///
    /// **Awaited, not signalled and forgotten.** A turn is mid-way through a
    /// pair of writes for a good part of its life, and a shutdown that returned
    /// before it finished would be a shutdown that can leave a Job's step moved
    /// and its Job not.
    pub async fn stopped(mut self) {
        self.stop.notify_one();
        if let Some(handle) = self.handle.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for Turning {
    /// The backstop, for a caller that drops rather than stops — a test, or a
    /// start that failed after this was spawned. It aborts rather than awaits,
    /// because `drop` cannot await; [`Turning::stopped`] is the clean path and
    /// is what the composition root uses.
    fn drop(&mut self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
    }
}
