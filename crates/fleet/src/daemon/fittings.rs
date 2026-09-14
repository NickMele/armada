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

use adapter_traits::{
    AgentHarness, CiConfiguration, Delivery, LinkLookup, Model, ModelClient, Vcs, WorkProduct,
};
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
use crate::limits::Limits;
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
    /// The headless agent CLI's program, resolved once by the composition
    /// root's `AGENT_BINARY` override or its default. **Held here for Helm's
    /// host and not for a Drone's** — a Drone's own is `harness`'s, a typed
    /// [`AgentHarness`]; this is the plain program name
    /// `crate::helm::ProcessHost` runs, so a machine that names an override
    /// names it for a Drone, the Judge and Helm alike rather than leaving
    /// Helm on whatever `PATH` happens to hold. `#943`.
    pub agent_binary: String,
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

/// [`Host`] without the repository: what is true of the machine whichever
/// repository a Job is in. **What Fleet holds**, so a repository's root is
/// only ever read off the repository — `crate::repositories`.
#[derive(Clone, Debug)]
pub(crate) struct Local {
    pub(crate) path: String,
    pub(crate) home: String,
    pub(crate) user: String,
    pub(crate) mcp_config: String,
    pub(crate) port: u16,
    pub(crate) attachments_dir: String,
}

/// One repository a Fleet is assembled already serving.
#[derive(Clone, Debug)]
pub struct StartingIn {
    /// Absolute. Every worktree of its Jobs is added here.
    pub root: String,
    /// Where its Job records live, never under `root`. `crate::records::root`
    /// resolves it.
    pub records_root: String,
    /// Every workflow a Job there may run, keyed by `workflow_id`, each
    /// resolved against `manifest`.
    pub workflows: BTreeMap<WorkflowId, ResolvedWorkflow>,
    /// The Kit and carried definitions left out of `workflows`.
    pub left_out: Vec<ipc::LeftOutWorkflow>,
    pub manifest: Manifest,
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
    /// The repository Fleet is assembled serving, if any. **None at the
    /// composition root**: a Fleet starts with nothing and a person adds the
    /// first. A test starts in one, so its cases need no add.
    pub starting_in: Option<StartingIn>,
    pub host: Host,
    /// Reading a folder a person adds. See [`crate::repositories::Locating`].
    pub locating: Arc<dyn crate::repositories::Locating>,
    /// The range a Job's port span is claimed from. `settings.port-range-base`,
    /// `settings.port-range-ceiling` (`crate::ports::detect_ceiling` supplies
    /// the composition root's default) and `settings.port-block-granule`.
    pub port_range: crate::ports::PortRange,
    /// `settings.ad-hoc-run-log-retention`. How long a run fired by hand from
    /// the Manifest surface keeps its log — see [`mod@crate::rehearsing`].
    pub run_log_retention: std::time::Duration,
    /// `settings.helm-action-authority-tier-1-redirect-enabled-vs-read-only`,
    /// resolved here like every other Machine setting. `#943`.
    pub helm_authority: crate::helm::Authority,
    /// `settings.helm-session-retention-expiry`, resolved here for
    /// `run_log_retention`'s reason. `#943`.
    pub helm_session_retention: std::time::Duration,
    /// How a caller is placed: which process holds the connection a tool call
    /// arrived on. **A seam so a test can plant one** — the shipped answer is
    /// [`peer::Kernel`](crate::peer::Kernel), and a fixture has no sockets to
    /// ask about.
    pub peers: Arc<dyn PeerOf>,
    /// How many Jobs Fleet may work at once **where nobody has saved another**.
    /// The `settings.concurrency-cap` row, enforced — see [`Concurrency`], which
    /// has no default for the reason none of the four dials above it does, and
    /// `crate::limits` for how a saved value replaces it.
    pub concurrency: Concurrency,
    /// What the machine has left, asked rather than assumed. **A seam so a
    /// test can plant one**, exactly as `peers` is — the shipped answer is
    /// [`TheMachine`](crate::headroom::TheMachine) and a fixture has no machine
    /// it can hold still.
    pub machine: Arc<dyn Machine>,
    /// Cloning a seed into a worktree. **A seam for `machine`'s reason** — the
    /// shipped answer is [`TheVolume`](crate::seeding::TheVolume). #1064.
    pub copy_on_write: Arc<dyn crate::seeding::CopyOnWrite>,
    /// How much memory and disk must be free before another Drone starts,
    /// **where nobody has saved another**. The
    /// `settings.cpu-mem-headroom-threshold-for-spawning` and
    /// `settings.disk-headroom-floor-for-spawning` rows, enforced — see
    /// [`Headroom`], which has no default for [`Concurrency`]'s reason.
    pub headroom: Headroom,
    /// How many Checks may run at once on this machine, **where nobody has
    /// saved another**. The `settings.checks-at-once` row, enforced — see
    /// [`ChecksAtOnce`](crate::ChecksAtOnce), which has no default for
    /// [`Concurrency`]'s reason. #284, #1063.
    pub checks_at_once: crate::ChecksAtOnce,
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
    /// How many fixes one step may ask for. See [`Fixes`](crate::fixing::Fixes). #999.
    pub fixes: crate::fixing::Fixes,
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
    /// How long one permission question is held open inside the Drone's call
    /// before the Drone is told to wait for the answer as a turn. See
    /// [`crate::permitting::PermissionHold`], which has no default for
    /// [`JudgeBudget`]'s reason — and which the composition root writes as the
    /// harness-derived [`crate::permitting::HOLD`].
    pub permission_hold: crate::permitting::PermissionHold,
    /// How long a permission ask may go unanswered before Fleet ends the
    /// Drone and escalates the Job, reclaiming its concurrency slot. See
    /// [`crate::permitting::UnansweredAskLimit`], which has no default for
    /// [`JudgeBudget`]'s reason. `#801`.
    pub unanswered_ask_limit: crate::permitting::UnansweredAskLimit,
    /// What a step naming no model of its own is judged by. **Resolved by the
    /// composition root**, like every other input here — which model is cheap
    /// is a vendor's fact, and nothing below Fleet may spell one.
    pub judge_model: Model,
    /// What a judged gaming flag is read a second time on. **Resolved by the
    /// composition root**, for `judge_model`'s reason.
    pub second_opinion_model: Model,
    /// What a dispatch request is read by. **Its own dial and not the Judge's**
    /// — this call fires on every dispatch rather than on every criterion, so
    /// the two are raised for different reasons and at different prices.
    pub proposer_model: Model,
    /// What a request's own link resolves to, before it becomes a Job's
    /// `facts`. **A pointer rather than a type parameter**, for `judge`'s
    /// reason: rendering cannot fail, so nothing about it needs to be generic.
    pub links: Arc<dyn LinkLookup + Send + Sync>,
    /// What reads a repository's CI configuration for Scan. **A seam so Fleet
    /// names no provider**, and so a test can plant what a reader answers.
    pub ci_configuration: Arc<dyn CiConfiguration + Send + Sync>,
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
        let helm = crate::helm::Conversations::hosted_by(Arc::new(
            crate::helm::ProcessHost::on_this_machine(&fittings.host),
        ));
        let shipped = Limits {
            concurrency: fittings.concurrency,
            headroom: fittings.headroom,
            checks_at_once: fittings.checks_at_once,
        };
        // **A row that will not read is the shipped limits**, not a Fleet that
        // will not start: `Store::open` already refused a damaged file, and the
        // next save writes the whole row again.
        let in_force = shipped.overlaid_by(&fittings.store.saved_limits().unwrap_or_default());
        Fleet {
            store: Mutex::new(fittings.store),
            harness: Arc::new(fittings.harness),
            vcs: Arc::new(fittings.vcs),
            work: fittings.work,
            clock: fittings.clock,
            mint: fittings.mint,
            repositories: Arc::new(match fittings.starting_in {
                Some(first) => crate::repositories::Repositories::starting_in(
                    first.root,
                    first.records_root,
                    crate::repositories::SetUp::of(first.manifest, first.workflows)
                        .leaving_out(first.left_out),
                ),
                None => crate::repositories::Repositories::none(),
            }),
            locating: fittings.locating,
            host: Local {
                path: fittings.host.path,
                home: fittings.host.home,
                user: fittings.host.user,
                mcp_config: fittings.host.mcp_config,
                port: fittings.host.port,
                attachments_dir: fittings.host.attachments_dir,
            },
            port_range: fittings.port_range,
            run_log_retention: fittings.run_log_retention,
            helm_authority: fittings.helm_authority,
            helm_session_retention: fittings.helm_session_retention,
            budget: fittings.budget,
            norms: fittings.norms,
            liveness: fittings.liveness,
            dry_runs: fittings.dry_runs,
            fixes: fittings.fixes,
            judge: fittings.judge,
            judge_budget: fittings.judge_budget,
            proposer_budget: fittings.proposer_budget,
            command_budget: fittings.command_budget,
            permission_hold: fittings.permission_hold,
            unanswered_ask_limit: fittings.unanswered_ask_limit,
            aloft: Aloft::default(),
            underway: Underway::default(),
            proposals: Proposals::new(),
            judge_model: fittings.judge_model,
            second_opinion_model: fittings.second_opinion_model,
            proposer_model: fittings.proposer_model,
            links: fittings.links,
            ci_configuration: fittings.ci_configuration,
            models: fittings.models,
            events: fittings.events,
            turns: api::Turns::new(),
            helm,
            inbox: EvidenceInbox::new(),
            delivered: Mutex::new(BTreeMap::new()),
            slots: Mutex::new(Slots::bounded_by(in_force.concurrency)),
            names: Arc::new(crate::naming::Names::new()),
            machine: fittings.machine,
            copy_on_write: fittings.copy_on_write,
            seeds: Arc::new(std::sync::Mutex::new(crate::seeding::Seeds::default())),
            base_preparing: Arc::new(tokio::sync::Mutex::new(())),
            headroom: std::sync::Mutex::new(in_force.headroom),
            places: crate::places::Places::of(in_force.checks_at_once),
            shipped,
            polling: fittings.polling,
            noticing: fittings.noticing,
            reclaiming: fittings.reclaiming,
            swept: Mutex::new(None),
            sweeping: Mutex::new(Sweep::default()),
            peering: Mutex::new(crate::peers::Peering::default()),
            proving: Arc::new(Mutex::new(crate::proving::Proving::default())),
            fixing_on_main: Mutex::new(std::collections::BTreeSet::new()),
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

    /// The same Fleet with Helm's messages carried by `host` — a stand-in in a
    /// test, or a host somewhere other than a process Fleet starts.
    pub fn hosting_helm_on(mut self, host: Arc<dyn crate::helm::Hosting>) -> Fleet<H, V, W> {
        self.helm = crate::helm::Conversations::hosted_by(host);
        self
    }
}
