//! What a Job is allowed to spend, and the one predicate that says it has.
//!
//! # It is per Job, because the Job is what a person approved
//!
//! A Drone belongs to a workflow step, so a four-step Job is four Drones. A
//! per-Drone ceiling on that Job is four times the number anybody thought they
//! set, and it bounds nothing about the thing that was approved. So the cap is
//! held against the Job, and the sum lives in `store::job_drone_spend` because
//! it has to be spent down across Drones that never meet.
//!
//! # It refuses the next dispatch. It does not stop a Drone that is spending
//!
//! `cost_micros` arrives on the final result line of a Drone's session and
//! nowhere else — there is no mid-session figure on the stream — so a cap can
//! decline to start the next thing and cannot interrupt the current one.
//!
//! **That is the whole of what it does, and every place a person sets it says
//! so.** A ceiling that reads as "spending stops here" and means "nothing new
//! starts after here" is worse than no ceiling, because it is believed. The
//! case it does catch is the one that matters: a runaway is a sequence of Jobs
//! rather than a single Drone, and the sequence is exactly what admission sees.
//!
//! # Two signals, because dollars alone cannot be set to a useful number
//!
//! `docs/spikes/005-what-does-a-job-cost.md` measured three identical,
//! identically successful runs of one Job at $0.063, $0.087 and $0.146 — a
//! 2.31x spread on cache warmth alone, with almost none of it attributable to
//! the work. A dollar ceiling tight enough to catch a runaway kills a healthy
//! Job that started cold. The same three runs turned 7, 7 and 4 times.
//!
//! So the dollar cap is deliberately wide and the turn cap is what catches what
//! a wide ceiling misses. [`Overspent`] says which of the two it was, because
//! "over budget" and "took four times as many turns as any other run" are
//! different findings and a person acts differently on each.
//!
//! **Wall clock is the third signal the spike names and it is not here**, for
//! the reason a second vocabulary is a defect: `settings.drone-job-timeout`
//! already bounds a Job's wall clock, at Kit-to-Manifest scope. It is
//! unenforced today, and enforcing it is that row's work rather than this one's
//! — `store::DroneSpend::ran_ms` is recorded against every Drone so the figure
//! is there when somebody does it.
//!
//! # Quota is not a fourth signal
//!
//! Spike 5 settled it and the owner's call stands: the rate-limit event on a
//! Drone's stream carries a window and a status and no quantity, so there is no
//! number to hold a Job back against.

use adapter_traits::{AgentHarness, Delivery, DroneEvent, Vcs, WorkProduct};
use core_model::{DroneId, Job, JobId, JobStatus};
use store::{DroneSpend, Spend};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// A quantity of money, held as millionths of a dollar and named in the unit it
/// was decided in.
///
/// **An integer, like [`crate::headroom::Bytes`].** A cap compared as a float
/// answers differently on two machines for the same spend, and the figure
/// arrives as an integer anyway: `adapters::transcript` turns `total_cost_usd`
/// into micros at the boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Micros(u64);

impl Micros {
    pub const fn dollars(dollars: u64) -> Micros {
        Micros(dollars * 1_000_000)
    }

    pub const fn count(&self) -> u64 {
        self.0
    }
}

/// Which of a Job's two ceilings it has gone past.
///
/// **Both fold to `QueuedReason::OverBudget` on the Board**, which is the only
/// label `job-statuses.toml` gives a `queued` Job held back by what it has
/// spent. The distinction between them is the operator's, exactly as
/// [`Short`](crate::headroom::Short)'s is — and unlike that one it reaches a
/// person, because the Job's detail carries what was spent beside what was
/// allowed and the figure that is over is visible in the pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overspent {
    /// Past `settings.budget-cost-cap-per-job`. The remedy is a number: raise
    /// the cap, or accept that this Job costs what it costs.
    Cost,
    /// Past `settings.budget-turn-cap-per-job`. The remedy is usually the
    /// brief: a Job that turns and turns was not askable as written.
    Turns,
}

impl std::fmt::Display for Overspent {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(match self {
            Overspent::Cost => "cost",
            Overspent::Turns => "turns",
        })
    }
}

/// What one Job may spend before Fleet stops starting Drones on it.
///
/// **No `Default`**, for [`Concurrency`](crate::Concurrency)'s reason: the
/// numbers are a decision somebody made and wrote down, and a type that
/// supplies them lets a caller not make it. `crates/armada/src/serve.rs`
/// resolves them and carries the argument for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allowance {
    cost: Micros,
    turns: u64,
}

impl Allowance {
    pub const fn of(cost: Micros, turns: u64) -> Allowance {
        Allowance { cost, turns }
    }

    pub const fn cost(&self) -> Micros {
        self.cost
    }

    pub const fn turns(&self) -> u64 {
        self.turns
    }

    /// Which ceiling this spend is past, or `None` where it is inside both.
    ///
    /// **Cost first**, and the order decides only which one is named: dollars
    /// are what the row a person set is denominated in, so a Job over both
    /// reads as the thing they were watching.
    ///
    /// The comparison is `>=` and not `>`. A cap is what a Job may spend up to,
    /// and a Job that has spent exactly its allowance has nothing left to start
    /// a Drone with.
    pub fn exceeded_by(&self, spent: &Spend) -> Option<Overspent> {
        if spent.cost_micros >= self.cost.count() {
            return Some(Overspent::Cost);
        }
        if spent.turns >= self.turns {
            return Some(Overspent::Turns);
        }
        None
    }
}

/// What one Drone's stream came to, folded once.
///
/// **Cost is the last figure seen and turns are the sum**, which is measured
/// rather than assumed. `docs/spikes/004-transcript-idle-session.ndjson` is one
/// session with two terminating lines: `num_turns` reads 3 and then 2, while
/// the second line's `modelUsage` holds the sum of both invocations — input 10
/// = 6 + 4, output 444 = 271 + 173 — and its `total_cost_usd` reconstructs from
/// those cumulative figures exactly. So `total_cost_usd` is the session's
/// running total and `num_turns` is per invocation. Adding the costs would bill
/// the first invocation twice; taking the last turn count would report a
/// two-turn Drone that took five.
///
/// `ran` is Fleet's own clock and not the stream's: the harness reports a
/// duration per terminating line and Armada does not carry it, and the wall
/// clock is what a person means by how long a step took either way.
pub(crate) fn spent(events: &[DroneEvent], ran: std::time::Duration) -> DroneSpend {
    let mut spend = DroneSpend {
        ran_ms: ran.as_millis() as u64,
        ..DroneSpend::default()
    };
    for event in events {
        if let DroneEvent::Ended {
            turns, cost_micros, ..
        } = event
        {
            spend.cost_micros = *cost_micros;
            spend.turns += *turns as u64;
        }
    }
    spend
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
    /// Write down what one Drone of a Job spent.
    ///
    /// **Called from both places a Drone's run ends**, and safe there because
    /// the record is keyed on the Drone: `store::record_drone_spend` is an
    /// upsert, so recording the same Drone twice writes the same row twice
    /// rather than billing the Job twice. That property is the store's and not
    /// a rule a caller has to keep.
    ///
    /// **A spend that will not write is a fault and is returned.** The figure
    /// is what a cap is compared against, and one that silently failed to land
    /// would leave a Job with an allowance it can never exhaust.
    pub(crate) async fn record_spend(
        &self,
        job: &JobId,
        drone: &DroneId,
        spend: &DroneSpend,
    ) -> Result<(), Adrift> {
        self.store()
            .lock()
            .await
            .record_drone_spend(job, drone, spend)
            .map_err(Adrift::Writing)
    }

    /// What this Job has spent across every Drone that has worked it.
    pub(crate) async fn spend_of(&self, job: &JobId) -> Result<Spend, Adrift> {
        self.store()
            .lock()
            .await
            .spend_for(job)
            .map_err(Adrift::Reading)
    }

    /// Whether this Job has already spent what it was allowed, and which
    /// ceiling it went past.
    ///
    /// **The one predicate.** `admitting`'s `next_queued` skips a Job this
    /// answers `Some` for, and `serving`'s `queued_reason` labels the same Job
    /// `over_budget` from it — one answer, because a Board saying a Job is
    /// waiting on the machine while admission is holding it back for money is
    /// two different sentences about one Job.
    ///
    /// **Only a `queued` Job is asked.** A Job that is running has a Drone on
    /// it and this cannot stop that Drone; a terminal Job is not going to start
    /// another. The read costs one query and there is no reason to pay it for a
    /// row that could not act on the answer.
    pub(crate) async fn overspent(&self, job: &Job) -> Result<Option<Overspent>, Adrift> {
        if job.status() != JobStatus::Queued {
            return Ok(None);
        }
        let spent = self.spend_of(job.id()).await?;
        Ok(self.allowance().exceeded_by(&spent))
    }
}
