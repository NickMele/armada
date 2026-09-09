//! What a Job is allowed to spend, and the one predicate that says it has.
//!
//! `docs/concepts/machine.md`, Budget, is where this is argued: why the cap is
//! per Job, why it is two numbers rather than one, and why it refuses the next
//! dispatch rather than stopping a Drone. `docs/spikes/005-what-does-a-job-cost.md`
//! is the measurement under all three.
//!
//! Two things the concept page cannot say, because they are about this code:
//!
//! **Wall clock is the spike's third signal and is not here.**
//! `settings.drone-job-timeout` already bounds a Job's wall clock at
//! Kit-to-Manifest scope, so a second ceiling would be two answers to one
//! question. `store::DroneSpend::ran_ms` is recorded anyway, so that row has a
//! figure when somebody enforces it.
//!
//! **Quota is not a fourth.** Spike 5 settled it: the rate-limit event carries
//! a window and a status and no quantity.
//!
//! **The dollars tier and the turns do not**, which is the one place the pair
//! comes apart. [`Allowance::at`] carries the argument.

use adapter_traits::{AgentHarness, Delivery, DroneEvent, Vcs, WorkProduct};
use config::Manifest;
use core_model::{DroneId, Job, JobId, JobStatus};
use store::{DroneSpend, Spend};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::working::{StoodDown, Working};

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

    /// A figure that already arrived in this unit — off the wire, or off the
    /// Job's own column. **Beside [`Micros::dollars`] rather than replacing
    /// it**: a caller writing a ceiling down says what it means in the unit it
    /// decided in, and a caller carrying one across a boundary does not
    /// convert.
    pub const fn of(count: u64) -> Micros {
        Micros(count)
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
    /// Past `settings.budget-cost-cap-per-job`, as [`Allowance::at`] resolved
    /// it for this Job. The remedy is a number: raise the cap on this Job, on
    /// the repository, or accept that this Job costs what it costs.
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

    /// What this one Job may spend, given what Fleet is running with.
    ///
    /// # Three tiers, and this is the only place their order is written
    ///
    /// The composition root's constant — `self` — then `armada.yml`'s
    /// `drone.cost_cap_micros_per_job`, then the Job's own column, each
    /// deferring upward where it states nothing. **`Some(0)` is a cap and
    /// `None` is an absence**: capped at zero a Job starts nothing, which holds
    /// one Job, or one repository, without stopping the Fleet.
    ///
    /// # The turns do not tier, and that is a decision
    ///
    /// Spike 5 priced three identical successful runs of one Job at $0.063,
    /// $0.087 and $0.146 — 2.31x on cache warmth — while their turns held at 7,
    /// 7 and 4. So a Job over the dollar cap often just started cold and the
    /// remedy is the number; over the turn cap it is going in circles, and
    /// raising the number buys more circles. **A lever exists for the reading
    /// whose remedy is a number.** The two stay one type and one
    /// `exceeded_by` — what tiers is the value, not the pair.
    ///
    /// # Live at every tier, and frozen at none
    ///
    /// A Job past its cap is refused at every admission until a number moves,
    /// so a cap frozen at creation — as a step's `quiet_after_seconds` is —
    /// would reach every Job but the one that needs it.
    pub fn at(self, manifest: &Manifest, job: &Job) -> Allowance {
        let repository = match manifest.cost_cap_micros() {
            Some(micros) => Micros(u64::from(micros)),
            None => self.cost,
        };
        Allowance {
            cost: match job.cost_cap_micros() {
                Some(micros) => Micros(micros),
                None => repository,
            },
            turns: self.turns,
        }
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
///
/// **A stream with no terminating line answers [`None`], never nought.** Cost
/// is carried on that line alone, so a Drone that was signalled mid-run has no
/// price rather than a price of nothing — and a Drone that ran for five minutes
/// and cost `0` is a sentence nothing should be able to write. Job
/// `01M21BKVPW002DC0ATD1X9T0VF` had two of them, and read as $5.28 against a $5
/// cap while having spent more than it could say.
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
            spend.cost_micros = Some(*cost_micros);
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
    /// End the Drone in a slot and write down what it spent, in that order.
    ///
    /// **The one place a slot is stood down.** Every deliberate ending used to
    /// pair [`Working::stood_down`] with [`record_spend`](Fleet::record_spend)
    /// for itself, and `crate::dispatch`'s did not — so a Job that finished, or
    /// whose gate-failure attempts ran out, was billed for every Drone but its
    /// last. `#398`. The pairing is here rather than remembered at three call
    /// sites, and [`Working::stood_down`] is called from nowhere else.
    ///
    /// **The order is the fold's, not this method's.** `Working::stood_down`
    /// signals the process and drains the pipe before it folds, because the
    /// terminating line carrying `total_cost_usd` is the last thing a Drone
    /// says — a figure read before the drain is a figure read off a prefix.
    ///
    /// **A spend that will not write is returned and the ending is not undone.**
    /// The process is already gone by then; what the caller decides is whether
    /// the failure stops it, and the two that can return it do.
    pub(crate) async fn stood_down_paying(&self, at_work: Working) -> Result<StoodDown, Adrift> {
        let stood_down = at_work.stood_down(&self.now()).await;
        self.record_spend(&stood_down.job, &stood_down.drone, &stood_down.spent)
            .await?;
        Ok(stood_down)
    }

    /// Write down what the Drone in the slot has spent, before the Job moves.
    ///
    /// **[`reap`](Fleet::reap)'s ordering, at the acts that reach a step
    /// boundary with the Drone still in the slot.** A client re-reads the Job
    /// on the event that says it moved, so a figure written after that publish
    /// is one nothing goes back for — and on an advance nothing does, because
    /// what follows is `drone.spawned`, which carries a row and not a move. A
    /// Job whose detail drew $4.12 against a Fleet answering $5.28 was short by
    /// its last Drone for exactly that reason.
    ///
    /// **A running total, and the upsert is what makes that safe.**
    /// [`Working::spent`] is what the Drone has cost so far, so this is honest
    /// at whatever instant it is taken; the fold in
    /// [`stood_down_paying`](Fleet::stood_down_paying) is taken after the drain
    /// and is the only one that can be final, and it replaces this on the row
    /// keyed by the same Drone.
    ///
    /// **A fold that names nothing is not written.** An adopted Drone's
    /// terminating line went into a pipe with no reader, so its fold is zero of
    /// everything — and writing that would replace the figure the Fleet before
    /// this one left, which nothing here can recover.
    pub(crate) async fn paid_so_far(&self, working: &Option<Working>) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(());
        };
        let spent = at_work.spent(&self.now());
        if spent.cost_micros.is_none() && spent.turns == 0 {
            return Ok(());
        }
        let (job, _, drone) = at_work.drone();
        self.record_spend(&job, &drone, &spent).await
    }

    /// Write down what one Drone of a Job spent.
    ///
    /// **Called from every place a Drone's run ends**, and safe there because
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
    ///
    /// **The cap is resolved here and not held anywhere**, which is what makes
    /// raising one on a Job that is already over it take effect at the next
    /// turn of the loop rather than at the next restart.
    pub(crate) async fn overspent(&self, job: &Job) -> Result<Option<Overspent>, Adrift> {
        if job.status() != JobStatus::Queued {
            return Ok(None);
        }
        let spent = self.spend_of(job.id()).await?;
        Ok(self.allowance_for(job).exceeded_by(&spent))
    }
}
