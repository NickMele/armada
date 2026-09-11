//! The daemon core: the [`Fleet`] a process is. It is assembled from
//! [`mod@fittings`], answers through [`mod@answering`], and hands what it holds
//! to the crate around it through [`mod@seams`]. A slot's contents are
//! [`working`](mod@crate::working), an invariant kept away from the logic that
//! moves it, and [`dispatch`](mod@crate::dispatch) is what moves it.
//!
//! **The queue is not a list.** [`Fleet`] holds a [`Slots`] roster, one slot per
//! Job being worked and bounded by [`Concurrency`](crate::slots::Concurrency),
//! and a Job approved while the bound is spent stays at `queued` — a status the
//! registry already has and the store already persists. So there is no queue
//! object here, and no ordering in memory a restart could lose or disagree with
//! the log: `#50` added the second slot rather than a scheduler over it. **The
//! bound is on Drones, never on approvals.**
//!
//! **A refused transition is not survivable.** Every move goes through
//! `Job::transition` or `Job::transition_step`, and no arm below logs an
//! `Adrift::IllegalMove` or `Adrift::IllegalStepMove` and continues: a refusal
//! means Fleet asked for what the edge table says cannot happen, a bug in Fleet.
//!
//! **Three locks — the roster, one Job's slot, the store — taken in that order
//! and in no other**, with [`crate::slots`] holding the argument. The gate holds
//! a slot across a Check, but the checked Job's own, so a quarter-hour run holds
//! up that Job's Drone alone. [`Fleet::merge_end`] is a fourth and sits outside
//! the order, for the reason its field gives.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{LinkLookup, Model, ModelClient};
use config::{Manifest, ResolvedWorkflow};
use core_model::{JobId, Ulid, WorkflowId};
use store::Store;
use tokio::sync::Mutex;

use crate::admitting::Polled;
use crate::allowance::Allowance;
use crate::clock::Clock;
use crate::converging::StepNorms;
use crate::delivery::Delivered;
use crate::dry_run::DryRuns;
use crate::evidence::EvidenceInbox;
use crate::gate::CheckBudget;
use crate::headroom::{Headroom, Machine, Polling};
use crate::holding::Reclaiming;
use crate::judging::{Aloft, JudgeBudget};
use crate::mint::Mint;
use crate::naming::Names;
use crate::noticing::{Noticing, Sweep};
use crate::peer::{Drones, PeerOf};
use crate::proposals::Proposals;
use crate::silence::Liveness;
use crate::slots::Slots;

mod answering;
mod fittings;
mod rereading;
mod seams;

pub use fittings::{Fittings, Host};

/// The daemon core: **the only writer of Job state.**
pub struct Fleet<H, V, W> {
    store: Mutex<Store>,
    harness: Arc<H>,
    vcs: V,
    work: W,
    clock: Arc<dyn Clock>,
    mint: Arc<dyn Mint>,
    workflows: BTreeMap<WorkflowId, ResolvedWorkflow>,
    manifest: Manifest,
    host: Host,
    budget: CheckBudget,
    norms: StepNorms,
    liveness: Liveness,
    dry_runs: DryRuns,
    judge: Arc<dyn ModelClient + Send + Sync>,
    judge_budget: JudgeBudget,
    proposer_budget: JudgeBudget,
    /// The Judge call that is out right now, or none. **The one piece of Fleet
    /// state that is only ever true for as long as it takes** — it is never
    /// written down, because a record of it would outlive the fact.
    aloft: Aloft,
    /// Every proposal in flight, for the reason `aloft` exists one Job along:
    /// a call somebody could be waiting on has to be nameable while it is out.
    /// **Minted here, not a fitting** — nothing outside this crate holds one,
    /// and it is empty after a restart because a proposal is not a record.
    proposals: Proposals,
    judge_model: Model,
    proposer_model: Model,
    links: Arc<dyn LinkLookup + Send + Sync>,
    models: ipc::ModelChoices,
    events: api::Broadcaster,
    /// Every Job somebody could be watching. **Minted here, not a fitting** —
    /// nothing outside this crate holds one, because a viewer reaches it
    /// through `api::Queries::observe_job` rather than through the composition
    /// root.
    turns: api::Turns,
    inbox: EvidenceInbox,
    /// What each finished Job's delivery came to, waiting for the turn that
    /// reports it. **Drained, not read** — a second turn must not report a
    /// push that happened before it.
    ///
    /// **Keyed by Job**, unlike the single value it was: two Jobs can reach
    /// their branch in one turn, and one slot for both would have the second
    /// one's push overwrite the first's before anybody was told about it.
    delivered: Mutex<BTreeMap<JobId, Delivered>>,
    /// Which Jobs are being worked and how many may be. See [`crate::slots`]
    /// for the two locks and the order they are taken in.
    slots: Mutex<Slots>,
    /// What each Job is called on disk. **Minted here, not a fitting** — it is
    /// filled from the boot read and from every insert, so nothing outside this
    /// crate could hand one over already true. See [`mod@crate::naming`].
    names: Names,
    machine: Arc<dyn Machine>,
    headroom: Headroom,
    polling: Polling,
    noticing: Noticing,
    reclaiming: Reclaiming,
    /// What this Fleet's last read of `armada.yml` came to. **Never written
    /// down**, for `swept`'s reason: a reading that outlived the process would
    /// describe a file this Fleet never read. `None` is a Fleet still running
    /// on the Manifest it booted with, which is not a re-read at all. See
    /// [`mod@rereading`], and `drones` for why the lock is `std`'s.
    reading: std::sync::Mutex<Option<ipc::ManifestReading>>,
    /// When the reclaim sweep last ran. **Never written down**, for
    /// `sweeping`'s reason: what it decides is re-derived from git and the
    /// board every time, so a stamp that outlived the process would only make
    /// the first sweep after a restart come later than it should.
    swept: Mutex<Option<core_model::Timestamp>>,
    /// Where the pull-request rotation stands, and when it last ran. **Never
    /// written down**, for `polled`'s reason: a cursor that outlived the
    /// process would name a position in a list that has since changed, and the
    /// answers it produces are on the record already.
    sweeping: Mutex<Sweep>,
    /// Which commit is being proved and what came back. Never written down, for
    /// `sweeping`'s reason; an `Arc` because the run is spawned — `crate::proving`.
    proving: Arc<Mutex<crate::proving::Proving>>,
    /// Which Jobs have a person's press out. Never written down, for
    /// `proving`'s reason; shared because the press's own task gives it back.
    pressing: crate::showing_again::Pressing,
    /// What one Job may spend. **Held rather than read** — like every other
    /// dial here, the composition root resolves it and nothing below Fleet
    /// reads configuration.
    allowance: Allowance,
    /// The last machine reading, and when it was taken. **Never written down**
    /// — headroom frees on its own, so a reading that outlived the process
    /// would be a reason that was already wrong when it was read back.
    ///
    /// Taken after the roster and never before it, which is the order
    /// `crate::slots` states. Nothing is held while this one is.
    polled: Mutex<Option<Polled>>,
    /// Which process is working which Job. See [`Drones`], which argues why
    /// this is not the same fact as the pid inside the slot.
    ///
    /// A `std::sync::Mutex` rather than tokio's: it is never held across an
    /// `.await`, and what it guards is a map of a handful of integers.
    drones: std::sync::Mutex<Drones>,
    peers: Arc<dyn PeerOf>,
    /// The rebase-and-push tail, held by one Job at a time.
    ///
    /// **Every worktree is cut from one `.git`**, and whether two of them can
    /// rebase and push into it concurrently is not established — `#50` accepted
    /// the serialisation rather than discovering git's ref lockfiles by way of
    /// a Job dying at its push, unattended. So dispatch and the work run
    /// N-wide and this one part does not.
    ///
    /// **It is at the tail and not at admission.** A lock taken when a Job
    /// starts would be the single working slot again under another name; this
    /// one is taken when a Job's branch is touched and released when it has
    /// been.
    merge_end: Mutex<()>,
    /// **This process's** run id, minted once at assembly.
    ///
    /// It names the emitter rather than a record, which is the one id a
    /// process mints for itself — and it is why a Fleet restart is visible as
    /// this value changing rather than as nothing at all.
    run: Ulid,
}
