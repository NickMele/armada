//! Something that calls [`Fleet::turn`], for as long as it is held.
//!
//! # A tick, because two of the three things a turn does have no sender
//!
//! A turn rules on evidence that landed, reaps a Drone that is gone and admits
//! the next approved Job. A Drone exiting is a process event nothing here
//! awaits, and a submission arrives on the Evidence tool, which has no server
//! in front of it at M1 — so waking on a signal means inventing senders for
//! events that have none. A tick needs none, and is correct for all three.
//!
//! It is cheap where it matters: a turn while a Job is worked reads no store,
//! because the inbox is empty, the transcript has not ended and the slot is
//! full. The store read is the *idle* turn, which nothing waits on. Nor does
//! admission depend on it — `approve` dispatches inline — so this interval is
//! the latency of a **ruling** rather than of a start.
//!
//! # A Check that outlasts a tick
//!
//! `turn` holds the slot across its Check and this loop awaits each turn, so a
//! long Check stacks nothing and never runs two gates over one Job: the tick it
//! ran through is dropped, and the next turn is one interval after it finished.
//! It does hold up `kill_drone` and `kill_job`, which want that same slot —
//! Fleet's one-Job-at-a-time invariant being visible, not a property of this
//! loop. A failed turn goes to the caller's reporter, a **required** argument,
//! and ticking continues: one turn's fault is not every later Job's.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Vcs, WorkProduct};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

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
    V: Vcs + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
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
