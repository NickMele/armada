//! Everything Fleet is assembled from, and the assembly itself.
//!
//! **Resolved by the composition root, above this crate.** Nothing below Fleet
//! reads configuration: every dial, every path and every seam arrives here as a
//! field somebody wrote out, which is what lets a test plant one and assert on
//! what Fleet did with it.
//!
//! **Two plain structs, public fields, and no `Default` on either.** A builder
//! would let a caller stop early and a default would answer a new dial before
//! anybody had decided what it should be. Written out, adding a field is a
//! compile error at the one call site that assembles a Fleet — and
//! [`Fleet::assembled`] below is the only place the two are ever read.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, LinkLookup, Model, ModelClient, Vcs, WorkProduct};
use config::{Manifest, ResolvedWorkflow};
use core_model::WorkflowId;
use store::Store;
use tokio::sync::Mutex;

use super::Fleet;
use crate::allowance::Allowance;
use crate::clock::Clock;
use crate::converging::StepNorms;
use crate::dry_run::DryRuns;
use crate::evidence::EvidenceInbox;
use crate::gate::CheckBudget;
use crate::headroom::{Headroom, Machine, Polling};
use crate::holding::Reclaiming;
use crate::judging::{Aloft, JudgeBudget};
use crate::mint::Mint;
use crate::noticing::{Noticing, Sweep};
use crate::peer::{Drones, PeerOf};
use crate::proposals::Proposals;
use crate::silence::Liveness;
use crate::slots::{Concurrency, Slots};
use crate::underway::Underway;

/// What Fleet knows about the machine it runs on.
///
/// **Resolved once, by the composition root, and never read from this
/// process.** `crate::drone` gives the reason for the two paths: a Drone that
/// inherited Fleet's own `PATH` would find a different toolchain on two
/// machines, and a different one again after a shell profile changes. The same
/// argument makes every field here an argument rather than a lookup.
///
/// Public fields and no `Default`, so a caller writes each one out.
#[derive(Clone, Debug)]
pub struct Host {
    /// The repository every worktree is added to. Absolute.
    pub repo_root: String,
    /// Where this repository's Job records live: a Judge's brief, a Drone's
    /// transcript, a Job's log, a Check's output, a kept deliverable, a kept
    /// frame. **Never under `repo_root`.** `crate::records::root` is what
    /// resolves it, once, at the composition root — this field exists so
    /// nothing below `Host` has to resolve it again or hold a second opinion
    /// about where it is.
    pub records_root: String,
    /// What a Drone's `PATH` is set to. Fleet's choice, not Fleet's own.
    pub path: String,
    /// The home directory the agent CLI reads its credentials from. **The
    /// confinement's known floor** — see `crate::drone::HostPaths`.
    pub home: String,
    /// Who the operator is. The agent CLI will not authenticate without it,
    /// however readable its credentials are — see `crate::drone::environment`.
    pub user: String,
    /// The strict MCP configuration a Drone is bound to.
    pub mcp_config: String,
    /// The loopback port Fleet is listening on.
    ///
    /// **Held because a connection to it is what names a Drone** — see
    /// `crate::peer`, which matches a caller's port against this one as a pair.
    /// It is the same number `mcp_config` points at, resolved once by the
    /// composition root from the listener it actually bound.
    pub port: u16,
    /// Where Fleet keeps its own copy of a Job's attachments, outside every
    /// worktree. `drafted()` writes under `<attachments_dir>/<job_id>/`, and
    /// `dispatch` reads from there to seed the worktree a Drone actually sees.
    pub attachments_dir: String,
}

/// Everything Fleet is assembled from.
///
/// A plain struct with public fields rather than a builder, for the reason
/// `NewJob` gives: there is no `Default`, so a caller writes every field out
/// and cannot forget one, and adding a field is a compile error at the one call
/// site that matters.
pub struct Fittings<H, V, W> {
    pub store: Store,
    pub harness: H,
    pub vcs: V,
    pub work: W,
    pub clock: Arc<dyn Clock>,
    pub mint: Arc<dyn Mint>,
    /// Every workflow a Job may run, keyed by the `workflow_id` its definition
    /// carries. Fleet is pointed at a repository and `.armada/workflows/` may
    /// hold more than one definition — a proposal names which one it wants,
    /// and a name this map does not hold is refused at creation instead of
    /// written onto the record unverified.
    pub workflows: BTreeMap<WorkflowId, ResolvedWorkflow>,
    /// The `armada.yml` that workflow resolved against. Held because a Drone's
    /// toolbelt is built from the commands it declares.
    pub manifest: Manifest,
    pub host: Host,
    /// The range a Job's port span is claimed from. `settings.port-range-base`,
    /// `settings.port-range-ceiling` (`crate::ports::detect_ceiling` supplies
    /// the composition root's default) and `settings.port-block-granule`.
    pub port_range: crate::ports::PortRange,
    /// `settings.ad-hoc-run-log-retention`. How long a run fired by hand from
    /// the Manifest surface keeps its log — see [`mod@crate::rehearsing`].
    pub run_log_retention: std::time::Duration,
    /// How a caller is placed: which process holds the connection a tool call
    /// arrived on. **A seam so a test can plant one** — the shipped answer is
    /// [`peer::Kernel`](crate::peer::Kernel), and a fixture has no sockets to
    /// ask about.
    pub peers: Arc<dyn PeerOf>,
    /// How many Jobs Fleet may work at once. **The
    /// `settings.concurrency-cap` row, enforced** — see [`Concurrency`], which
    /// has no default for the reason none of the four dials above it does.
    pub concurrency: Concurrency,
    /// What the machine has left, asked rather than assumed. **A seam so a
    /// test can plant one**, exactly as `peers` is — the shipped answer is
    /// [`TheMachine`](crate::headroom::TheMachine) and a fixture has no machine
    /// it can hold still.
    pub machine: Arc<dyn Machine>,
    /// How much of the machine must be free before another Drone starts. **The
    /// `settings.cpu-mem-headroom-threshold-for-spawning` row, enforced** — see
    /// [`Headroom`], which has no default for [`Concurrency`]'s reason.
    pub headroom: Headroom,
    /// How stale a machine reading may be. **The
    /// `settings.fleet-health-check-resource-poll-interval` row** — see
    /// [`Polling`] for why it is a freshness bound rather than a second timer.
    pub polling: Polling,
    /// How often the forge is asked what became of one pull request. See
    /// [`Noticing`], and `crate::noticing` for why it is one Job a sweep rather
    /// than every Job a turn.
    pub noticing: Noticing,
    /// How often Fleet asks what disk it could give back. See
    /// [`Reclaiming`], and `crate::holding` for what makes a worktree one it
    /// may take without asking anybody.
    pub reclaiming: Reclaiming,
    /// What one Job may spend before Fleet stops starting Drones on it. **The
    /// `settings.budget-cost-cap-per-job` and `settings.budget-turn-cap-per-job`
    /// rows, enforced** — see [`Allowance`], which has no default for
    /// [`Concurrency`]'s reason, and `crate::allowance` for what a cap can and
    /// cannot do about a Drone already spending.
    pub allowance: Allowance,
    pub budget: CheckBudget,
    /// What a step is expected to cost before the thrashing chain looks at it.
    /// See [`StepNorms`] for why it has no default.
    pub norms: StepNorms,
    /// How long a Drone may say nothing before Fleet asks, and how many times
    /// it asks. Its own value rather than a fourth `StepNorms` number: what it
    /// bounds is the Drone being there at all, and nothing about it is measured
    /// against a step's work. See [`Liveness`].
    pub liveness: Liveness,
    /// How many times one step may ask Fleet to run its Checks. Its own value
    /// for [`Liveness`]'s reason: what it bounds is money spent answering the
    /// Drone rather than anything about the step's work. See
    /// [`DryRuns`](crate::DryRuns).
    pub dry_runs: DryRuns,
    /// What makes a Judge call. **A pointer rather than a type parameter**: the
    /// seam renders and cannot fail, so nothing about it needs to be generic.
    pub judge: Arc<dyn ModelClient + Send + Sync>,
    /// How long one Judge call may take. See [`JudgeBudget`] for why it has no
    /// default.
    pub judge_budget: JudgeBudget,
    /// How long one Job proposer call may take. **Not `judge_budget`** — the
    /// call is the same call and the wait is not: a Judge's is the only thing
    /// that can end a call nobody is watching, and a proposal has somebody who
    /// can end it themselves.
    pub proposer_budget: JudgeBudget,
    /// How long a plain command may take before Fleet answers a refusal in
    /// its own words. See [`crate::commanding::CommandBudget`], which has no
    /// default for [`JudgeBudget`]'s reason.
    pub command_budget: crate::commanding::CommandBudget,
    /// What a step naming no model of its own is judged by. **Resolved by the
    /// composition root**, like every other input here — which model is cheap
    /// is a vendor's fact, and nothing below Fleet may spell one.
    pub judge_model: Model,
    /// What a dispatch request is read by. **Its own dial and not the Judge's**
    /// — this call fires on every dispatch rather than on every criterion, so
    /// the two are raised for different reasons and at different prices.
    pub proposer_model: Model,
    /// What a request's own link resolves to, before it becomes a Job's
    /// `facts`. **A pointer rather than a type parameter**, for `judge`'s
    /// reason: rendering cannot fail, so nothing about it needs to be generic.
    pub links: Arc<dyn LinkLookup + Send + Sync>,
    /// The models a Job may name, and the one it gets when it names none.
    ///
    /// **Resolved by the composition root, like every other input here.**
    /// Nothing below reads configuration, which is what lets a test plant a
    /// roster and assert on what a proposal with no model was given. Where the
    /// default is blank, a proposal that names no model is refused at creation
    /// rather than at spawn.
    pub models: ipc::ModelChoices,
    pub events: api::Broadcaster,
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
    pub fn assembled(fittings: Fittings<H, V, W>) -> Fleet<H, V, W> {
        let run = fittings.mint.ulid();
        Fleet {
            store: Mutex::new(fittings.store),
            harness: Arc::new(fittings.harness),
            vcs: Arc::new(fittings.vcs),
            work: fittings.work,
            clock: fittings.clock,
            mint: fittings.mint,
            workflows: fittings.workflows,
            manifest: fittings.manifest,
            host: fittings.host,
            port_range: fittings.port_range,
            run_log_retention: fittings.run_log_retention,
            budget: fittings.budget,
            norms: fittings.norms,
            liveness: fittings.liveness,
            dry_runs: fittings.dry_runs,
            judge: fittings.judge,
            judge_budget: fittings.judge_budget,
            proposer_budget: fittings.proposer_budget,
            command_budget: fittings.command_budget,
            aloft: Aloft::default(),
            underway: Underway::default(),
            proposals: Proposals::new(),
            judge_model: fittings.judge_model,
            proposer_model: fittings.proposer_model,
            links: fittings.links,
            models: fittings.models,
            events: fittings.events,
            turns: api::Turns::new(),
            inbox: EvidenceInbox::new(),
            delivered: Mutex::new(BTreeMap::new()),
            slots: Mutex::new(Slots::bounded_by(fittings.concurrency)),
            names: crate::naming::Names::new(),
            machine: fittings.machine,
            headroom: fittings.headroom,
            polling: fittings.polling,
            noticing: fittings.noticing,
            reclaiming: fittings.reclaiming,
            reading: std::sync::Mutex::new(None),
            swept: Mutex::new(None),
            sweeping: Mutex::new(Sweep::default()),
            proving: Arc::new(Mutex::new(crate::proving::Proving::default())),
            pressing: crate::showing_again::Pressing::default(),
            rehearsals: crate::rehearsing::Rehearsals::default(),
            servers: crate::servers::Servers::default(),
            allowance: fittings.allowance,
            polled: Mutex::new(None),
            drones: std::sync::Mutex::new(Drones::default()),
            peers: fittings.peers,
            merge_end: Mutex::new(()),
            run,
        }
    }
}
