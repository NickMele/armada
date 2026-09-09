//! What Fleet hands the modules around it.
//!
//! **Accessors rather than `pub(crate)` fields**, which is the whole subject of
//! the file: a field would be assignable from the other module, and Fleet's own
//! configuration is fixed at assembly. Most of what is here reads something
//! resolved once; `judging`, `proposing` and `making` assemble a value out of
//! held state and still write none of it.
//!
//! **The async ones take one lock, for one map operation, and let it go.** What
//! a caller does with a slot afterwards is where the order matters, and
//! [`crate::slots`] states that order rather than this file restating it.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, LinkLookup, SpawnConfigRefused, Vcs, WorkProduct};
use config::{Manifest, ResolvedWorkflow};
use core_model::{Job, JobId, Timestamp, Ulid, WorkflowId};
use store::Store;
use tokio::sync::Mutex;

use super::{Fleet, Host};
use crate::admitting::Polled;
use crate::allowance::Allowance;
use crate::asked::Asked;
use crate::clock::Clock;
use crate::converging::StepNorms;
use crate::delivery::Delivered;
use crate::drone::{environment, HostPaths};
use crate::dry_run::DryRuns;
use crate::evidence::EvidenceInbox;
use crate::gate::CheckBudget;
use crate::headroom::{Headroom, Machine, Polling};
use crate::holding::Reclaiming;
use crate::judging::{Aloft, Judging, Marking};
use crate::mint::Mint;
use crate::noticing::{Noticing, Sweep};
use crate::proposal::Proposing;
use crate::proposals::Watching;
use crate::silence::Liveness;
use crate::slots::{Slot, Slots};

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
    pub(crate) fn store(&self) -> &Mutex<Store> {
        &self.store
    }
    pub(crate) fn harness(&self) -> &Arc<H> {
        &self.harness
    }
    pub(crate) fn vcs(&self) -> &V {
        &self.vcs
    }
    pub(crate) fn work(&self) -> &W {
        &self.work
    }
    /// One workflow by id, or `None` where this Fleet holds no such definition.
    pub(crate) fn workflow_named(&self, id: &WorkflowId) -> Option<&ResolvedWorkflow> {
        self.workflows.get(id)
    }
    /// Every workflow this Fleet holds, for `serving`'s `list_workflows`.
    pub(crate) fn workflows(&self) -> &BTreeMap<WorkflowId, ResolvedWorkflow> {
        &self.workflows
    }
    pub(crate) fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub(crate) fn host(&self) -> &Host {
        &self.host
    }
    pub(crate) fn budget(&self) -> CheckBudget {
        self.budget
    }
    /// **The machine tier alone**, and the answer for no particular Job. What
    /// a Job may actually spend is [`Fleet::allowance_for`], which resolves the
    /// repository's cap and the Job's own over this one.
    ///
    /// **Not [`Fleet::budget`]**, which is how long one Check may take — the
    /// two words collided before either shipped and the names are kept apart on
    /// purpose.
    pub(crate) fn allowance(&self) -> Allowance {
        self.allowance
    }
    /// What this Job may spend: the constant above, then `armada.yml`'s
    /// `drone.cost_cap_micros_per_job`, then the Job's own column.
    /// `Allowance::at` is where the order is written.
    pub(crate) fn allowance_for(&self, job: &Job) -> Allowance {
        self.allowance.at(self.manifest(), job)
    }
    pub(crate) fn norms(&self) -> StepNorms {
        self.norms
    }
    pub(crate) fn liveness(&self) -> Liveness {
        self.liveness
    }
    pub(crate) fn dry_runs(&self) -> DryRuns {
        self.dry_runs
    }

    /// What the gate needs in order to ask the Judge.
    ///
    /// The environment is the Drone's own list, built the same way and by the
    /// same function — a Judge call needs the credential floor for the reason a
    /// Drone does, and a second list here would be a second answer.
    ///
    /// **The Job is a parameter for two reasons, and neither reaches the
    /// Judge**: a call that is out has to be nameable while it is out, and a
    /// wait that cannot say whose it is is a fact no surface can place; and
    /// what the call was asked is filed under the Job it was asked about. A
    /// Judge call is still assembled from the step and the workflow, and there
    /// is no arrangement of this argument that puts a Job id into a brief.
    pub(crate) fn judging(&self, job: &JobId) -> Result<Judging, SpawnConfigRefused> {
        Ok(Judging {
            client: Arc::clone(&self.judge),
            budget: self.judge_budget,
            default_model: self.judge_model.clone(),
            environment: environment(HostPaths {
                path: &self.host.path,
                user: &self.host.user,
                home: &self.host.home,
            })?,
            marking: Marking::on(
                job.into(),
                self.aloft.clone(),
                self.events.clone(),
                Arc::clone(&self.clock),
                self.judge_budget,
            ),
            asked: Asked::under(self.host.repo_root.clone(), job.clone()),
        })
    }
    /// The Judge call that is out, for `serving` to put on `get_job`.
    ///
    /// **Read, never written, from here.** The only writer is the guard in
    /// `crate::judging`, which puts the mark up before a call and takes it down
    /// however the call ends.
    pub(crate) fn aloft(&self) -> &Aloft {
        &self.aloft
    }
    /// What the dispatch path needs in order to ask the proposer.
    ///
    /// **The Judge's client, and the proposer's own budget.** The call is the
    /// same call — one turn, no toolset, no directory — so the client is
    /// shared. The wait is not: a Judge's budget is the only thing that can end
    /// a call nobody is watching, and a proposal has a person who can end it,
    /// so a budget tight enough to be that backstop would take the decision
    /// away from them. It read `judge_budget` until the proposal wait was made
    /// watchable and stoppable.
    pub(crate) fn proposing(&self) -> Result<Proposing, SpawnConfigRefused> {
        Ok(Proposing {
            client: Arc::clone(&self.judge),
            budget: self.proposer_budget,
            model: self.proposer_model.clone(),
            environment: environment(HostPaths {
                path: &self.host.path,
                user: &self.host.user,
                home: &self.host.home,
            })?,
        })
    }

    /// What the dispatch path needs in order to be watched while it asks.
    ///
    /// **Beside `proposing` rather than inside it**, because the two answer
    /// different questions and one of them is optional. `Proposing` is how to
    /// make the call — the client, the model, the confinement — and is the same
    /// whether anybody is looking. This is who to tell and what may stop it,
    /// and a Fleet driven by a test with no stream still makes the call.
    pub(crate) fn making(&self) -> Watching {
        Watching {
            proposals: self.proposals.clone(),
            events: self.events.clone(),
            clock: Arc::clone(&self.clock),
            mint: Arc::clone(&self.mint),
            budget: self.proposer_budget,
            // The spelling the call will actually be made with, so a surface
            // naming the model names the one that is out rather than a default
            // read from somewhere else.
            model: self.proposer_model.as_str().to_string(),
        }
    }

    /// Stop a proposal that is out. **The whole of what anybody may do to
    /// one** — see `crate::proposals`.
    pub(crate) fn stop_proposal(&self, proposal: &ipc::ProposalId) -> bool {
        self.proposals.stop(proposal)
    }
    /// The models a Job may name. Read at creation and served by
    /// `list_models`; nothing else consults it.
    pub(crate) fn models(&self) -> &ipc::ModelChoices {
        &self.models
    }
    /// What a request's own link resolves to. Read by `crate::proposal`
    /// before a request becomes a Job's `facts`.
    pub(crate) fn links(&self) -> &Arc<dyn LinkLookup + Send + Sync> {
        &self.links
    }
    pub(crate) fn mint(&self) -> &Arc<dyn Mint> {
        &self.mint
    }
    /// **The one clock reading in the crate.** Everything below Fleet takes its
    /// instant as an argument, and this is where the argument comes from.
    pub(crate) fn now(&self) -> Timestamp {
        self.clock.now()
    }
    /// The clock itself, for the one thing that outlives a call and still has
    /// to stamp what it writes: a transcript's writer.
    pub(crate) fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
    pub(crate) fn run(&self) -> &Ulid {
        &self.run
    }
    pub(crate) fn publish(&self, event: ipc::Event) {
        self.events.publish(event);
    }

    /// The per-Job transcript channels. Dispatch opens one, `serving`
    /// subscribes to it.
    pub(crate) fn turns(&self) -> &api::Turns {
        &self.turns
    }
    /// The roster, for `dispatch` and for a turn.
    pub(crate) fn slots(&self) -> &Mutex<Slots> {
        &self.slots
    }

    /// The three dials the headroom half of admission reads. All of them are
    /// `crate::admitting`'s, which is the only caller.
    pub(crate) fn machine(&self) -> &Arc<dyn Machine> {
        &self.machine
    }
    pub(crate) fn headroom(&self) -> &Headroom {
        &self.headroom
    }
    pub(crate) fn polling(&self) -> Polling {
        self.polling
    }
    pub(crate) fn proving(&self) -> &Arc<Mutex<crate::proving::Proving>> {
        &self.proving
    }
    pub(crate) fn noticing(&self) -> Noticing {
        self.noticing
    }
    pub(crate) fn reclaiming(&self) -> Reclaiming {
        self.reclaiming
    }
    pub(crate) fn swept(&self) -> &Mutex<Option<core_model::Timestamp>> {
        &self.swept
    }
    pub(crate) fn sweeping(&self) -> &Mutex<Sweep> {
        &self.sweeping
    }
    pub(crate) fn polled(&self) -> &Mutex<Option<Polled>> {
        &self.polled
    }

    /// One Job's working slot, for the three Drone tools and for every act a
    /// person takes on a named Job. `None` is a Job with no Drone.
    ///
    /// Everything a Drone tool binds to is inside the slot, and taking that
    /// lock once is what makes which-Job, which-step and which-type one
    /// decision rather than three reads a turn can interleave with. **Which
    /// slot is `crate::peer`'s answer** — the Drone does not name it.
    pub(crate) async fn slot_of(&self, job: &JobId) -> Option<Slot> {
        self.slots.lock().await.slot_of(job)
    }

    /// One Job's slot, made if it has none.
    ///
    /// **For the acts a person takes on a Job Fleet is not holding**, which
    /// need somewhere to stand a Drone down or to land work through and have no
    /// entry in the roster to do it in. None of them starts a Drone: since #50
    /// only admission does, so an empty slot opened here holds nothing and
    /// `Slots::sweep` forgets it when the caller lets it go.
    pub(crate) async fn slot_for(&self, job: &JobId) -> Slot {
        self.slots.lock().await.opened_for(job)
    }

    /// The one working slot, for a fixture bounded at one.
    ///
    /// **A test's convenience and nothing else**, which is why it is `cfg(test)`
    /// rather than a method a surface could reach: with the bound above one
    /// there is no such thing as *the* slot, and asking for it is the question
    /// `#50` exists to stop anybody asking. A Fleet working nothing answers with
    /// an empty slot, so a caller reads "nothing is working" where it used to.
    #[cfg(test)]
    pub(crate) async fn the_only_slot(&self) -> Slot {
        let mut slots = self.slots.lock().await;
        match slots.working_on().first() {
            Some(job) => slots.opened_for(job),
            None => Arc::new(Mutex::new(None)),
        }
    }

    /// The rebase-and-push tail's lock. See the field.
    pub(crate) fn merge_end(&self) -> &Mutex<()> {
        &self.merge_end
    }
    pub(crate) fn inbox(&self) -> &EvidenceInbox {
        &self.inbox
    }
    /// Leave what a Job's branch came to for the turn that reports it.
    pub(crate) async fn left_delivered(&self, job: &JobId, delivered: Delivered) {
        self.delivered.lock().await.insert(job.clone(), delivered);
    }

    /// Take what this Job's branch came to, where anything is waiting.
    /// **Drained, not read**, so a second turn cannot report a push once.
    pub(crate) async fn take_delivered(&self, job: &JobId) -> Option<Delivered> {
        self.delivered.lock().await.remove(job)
    }
}
