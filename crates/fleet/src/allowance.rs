//! What a Job is allowed to spend, and the one predicate that says it has.
//!
//! `docs/concepts/machine.md`, Budget, argues why the cap is per Job, why it is
//! two numbers rather than one, and why it refuses the next dispatch rather than
//! stopping a Drone. `docs/spikes/005-what-does-a-job-cost.md` is the measurement.
//!
//! **Wall clock is the spike's third signal and is not here.**
//! `settings.drone-job-timeout` already bounds a Job's wall clock at
//! Kit-to-Manifest scope, so a second ceiling would be two answers to one
//! question; `store::DroneSpend::ran_ms` is recorded anyway for when somebody
//! enforces it.
//!
//! **Quota is not a fourth.** Spike 5 settled it: the rate-limit event carries a
//! window and a status and no quantity.

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
/// spent. Which of the two it was travels beside that label as
/// `core_model::BudgetHold`, exactly as [`Short`](crate::headroom::Short) does
/// as `AdmissionHold` — and it travels because reading it off the figures alone
/// asks a person to know there are two ceilings before they can see which one
/// caught them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overspent {
    /// Past `settings.budget-cost-cap-per-job`, as [`Allowance::at`] resolved
    /// it for this Job. The remedy is a number: raise the cap on this Job, on
    /// the repository, or accept that this Job costs what it costs.
    Cost,
    /// Past `settings.budget-turn-cap-per-job`, as [`Allowance::at`] resolved
    /// it for this Job. **Two remedies and not one**, which is the correction
    /// this variant carries: a Job that is turning and getting nowhere was not
    /// askable as written and wants a redispatch, and a Job that has finished
    /// its work and is held out of a cheap last step wants the number moved.
    /// The type cannot tell those apart and does not try — what it says is
    /// which ceiling, so that both acts are reachable.
    ///
    /// The reading that only the brief could fix shipped with this variant and
    /// was falsified by Job `01M22TYSAE0023MADDP5ZQEYGW`: 393 turns against
    /// 300, every Check passed, `summarise` never run, and a redispatch would
    /// have thrown away the work rather than finished it.
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
    /// **Three tiers, and this is the only place their order is written**: the composition root's
    /// constant — `self` — then `armada.yml`'s `drone.cost_cap_micros_per_job` or
    /// `drone.turn_cap_per_job`, then the Job's own column. `Some(0)` is a cap and `None` an
    /// absence: capped at zero a Job starts nothing, holding one Job or one repository.
    ///
    /// **The turns tier reverses a decision.** Spike 5's three runs of one Job spread 2.31x on
    /// price while turns held at 7, 7 and 4 — until Job `01M22TYSAE0023MADDP5ZQEYGW` finished,
    /// passed every Check, and stopped at 393 turns against 300 with a cheap `summarise` unrun.
    ///
    /// **Live at every tier, frozen at none** — a cap frozen at creation, as `quiet_after_seconds`
    /// is, would reach every Job but the one that needs it.
    pub fn at(self, manifest: &Manifest, job: &Job) -> Allowance {
        let repository = match manifest.cost_cap_micros() {
            Some(micros) => Micros(u64::from(micros)),
            None => self.cost,
        };
        let turns = match manifest.turn_cap() {
            Some(turns) => u64::from(turns),
            None => self.turns,
        };
        Allowance {
            cost: match job.cost_cap_micros() {
                Some(micros) => Micros(micros),
                None => repository,
            },
            turns: match job.turn_cap() {
                Some(capped) => capped,
                None => turns,
            },
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
/// **Cost is the last figure seen and turns are the sum — measured, not assumed.**
/// `docs/spikes/004-transcript-idle-session.ndjson` has two terminating lines: `num_turns`
/// reads 3 then 2, and the second line's `modelUsage` sums both invocations (input 10 = 6 + 4,
/// output 444 = 271 + 173), its `total_cost_usd` reconstructing exactly. So cost is the running
/// total and turns are per invocation — summing costs would bill the first invocation twice,
/// and the last turn count alone would report a two-turn Drone that took five.
///
/// `ran` is Fleet's own clock, not the stream's — the harness reports a duration per terminating
/// line that Armada does not carry, and wall clock is what a person means either way.
///
/// **A stream with no terminating line answers [`None`], never nought** — a Drone signalled
/// mid-run has no price, not a price of nothing. Job `01M21BKVPW002DC0ATD1X9T0VF` had two,
/// reading $5.28 against a $5 cap.
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
    /// **The one place a slot is stood down.** Every ending used to pair
    /// [`Working::stood_down`] with [`record_spend`](Fleet::record_spend) for
    /// itself, and `crate::dispatch`'s did not — billing a finished or
    /// attempts-exhausted Job for every Drone but its last (`#398`). The
    /// pairing lives here instead of three call sites, and
    /// [`Working::stood_down`] is called from nowhere else.
    ///
    /// **The order is the fold's, not this method's** — `Working::stood_down`
    /// drains the pipe before it folds, since the terminating line carrying
    /// `total_cost_usd` is the last thing a Drone says.
    ///
    /// **A spend that will not write is returned and the ending is not undone** — the process is
    /// already gone, and the two callers that can return it decide whether the failure stops them.
    pub(crate) async fn stood_down_paying(&self, at_work: Working) -> Result<StoodDown, Adrift> {
        let stood_down = at_work.stood_down(&self.now()).await;
        self.record_spend(&stood_down.job, &stood_down.drone, &stood_down.spent)
            .await?;
        Ok(stood_down)
    }

    /// Write down what the Drone in the slot has spent, before the Job moves.
    ///
    /// **Called at [`reap`](Fleet::reap)'s step-boundary acts, Drone still in the slot.** A client
    /// re-reads the Job on the event saying it moved, so a figure written after that publish is one
    /// nothing goes back for. A Job whose detail drew $4.12 against a Fleet answering $5.28 was
    /// short by its last Drone for exactly that reason.
    ///
    /// **A running total, safe because of the upsert.** [`Working::spent`] is honest at whatever
    /// instant it is taken; [`stood_down_paying`](Fleet::stood_down_paying)'s post-drain fold is
    /// the only one that can be final, and replaces this on the same row.
    ///
    /// **A fold that names nothing is not written** — an adopted Drone's terminating line went
    /// into a pipe with no reader, so writing a zero fold would overwrite the figure a prior
    /// Fleet left, unrecoverably.
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
    /// answers `Some` for, and `serving`'s `queued_reason` labels it
    /// `over_budget` from it — one answer, not two sentences about one Job.
    ///
    /// **Only a `queued` Job is asked** — a running Job's Drone cannot be
    /// stopped by this, and a terminal Job starts nothing; the read costs
    /// one query, paid only where the answer is actionable.
    ///
    /// **The cap is resolved here and not held anywhere**, so raising one on
    /// a Job already over it takes effect at the next turn, not the next
    /// restart.
    pub(crate) async fn overspent(&self, job: &Job) -> Result<Option<Overspent>, Adrift> {
        if job.status() != JobStatus::Queued {
            return Ok(None);
        }
        let spent = self.spend_of(job.id()).await?;
        Ok(self.allowance_for(job).exceeded_by(&spent))
    }
}
