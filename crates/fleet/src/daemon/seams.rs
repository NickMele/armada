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

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, CiConfiguration, Delivery, LinkLookup, SpawnConfigRefused, Vcs, WorkProduct,
};
use core_model::{Job, JobId, ManifestId, Timestamp, Ulid};
use store::Store;
use tokio::sync::Mutex;

use super::{Fleet, Local};
use crate::admitting::Polled;
use crate::allowance::Allowance;
use crate::asked::Asked;
use crate::clock::Clock;
use crate::converging::StepNorms;
use crate::delivery::Delivered;
use crate::drone::{environment, HostPaths};
use crate::dry_run::DryRuns;
use crate::evidence::EvidenceInbox;
use crate::explaining::Explaining;
use crate::gate::CheckBudget;
use crate::headroom::{Headroom, Machine, Polling};
use crate::holding::Reclaiming;
use crate::judging::{Aloft, Judging, Marking};
use crate::mint::Mint;
use crate::noticing::{Noticing, Sweep};
use crate::policy::Policies;
use crate::proposal::Proposing;
use crate::proposals::Watching;
use crate::silence::Liveness;
use crate::slots::{Slot, Slots};
use crate::underway::{Announcing, Underway};

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
    /// What each Job is called on disk. See [`mod@crate::naming`].
    pub(crate) fn names(&self) -> &Arc<crate::naming::Names> {
        &self.names
    }
    pub(crate) fn store(&self) -> &Mutex<Store> {
        &self.store
    }
    pub(crate) fn harness(&self) -> &Arc<H> {
        &self.harness
    }
    pub(crate) fn vcs(&self) -> &Arc<V> {
        &self.vcs
    }
    pub(crate) fn work(&self) -> &W {
        &self.work
    }
    /// Every repository this Fleet serves — `crate::repositories`.
    pub(crate) fn repositories(&self) -> &Arc<crate::repositories::Repositories> {
        &self.repositories
    }
    pub(crate) fn locating(&self) -> &Arc<dyn crate::repositories::Locating> {
        &self.locating
    }
    pub(crate) fn port_range(&self) -> crate::ports::PortRange {
        self.port_range
    }
    pub(crate) fn run_log_retention(&self) -> std::time::Duration {
        self.run_log_retention
    }
    /// How far this machine lets Helm act rather than only read. **The one
    /// place `settings.helm-action-authority-tier-1-redirect-enabled-vs-read-
    /// only` is resolved to a value** — `crate::helm::admitting` is the one
    /// place it is asked, and the brief and the door both follow that answer.
    /// `#943`.
    pub(crate) fn helm_authority(&self) -> crate::helm::Authority {
        self.helm_authority
    }
    /// How long a closed Helm session's stored session id is kept. See
    /// `crate::helm::serving`, which sweeps by it on every reply. `#943`.
    pub(crate) fn helm_session_retention(&self) -> std::time::Duration {
        self.helm_session_retention
    }
    /// What this repository has said about `auto_merge` and `review_gate`,
    /// folded across the Manifests gating one Job.
    ///
    /// **Read fresh on every call and never held.** Both settings are `Live`,
    /// so the answer is true at the instant it is taken and no longer — which
    /// is why every caller asks again rather than passing one down.
    ///
    /// **One Manifest per Job today, and the fold is still called.** A Convoy
    /// is gated by several, and this is the one function that grows when
    /// `Job::gate_manifests` can be resolved to files. `crate::policy` carries
    /// the argument, and `docs/concepts/convoy.md` the rule.
    pub(crate) fn gating_policies(&self, served: &crate::repositories::Served) -> Policies {
        let manifest = served.manifest();
        Policies::gating([(manifest.auto_merge(), manifest.review_gate())])
    }
    pub(crate) fn host(&self) -> &Local {
        &self.host
    }
    pub(crate) fn budget(&self) -> CheckBudget {
        self.budget
    }
    /// How long a plain command may take. See
    /// [`crate::commanding::CommandBudget`].
    pub(crate) fn command_budget(&self) -> crate::commanding::CommandBudget {
        self.command_budget
    }
    /// How long a permission question is held inside the Drone's call. See
    /// [`crate::permitting::PermissionHold`].
    pub(crate) fn permission_hold(&self) -> crate::permitting::PermissionHold {
        self.permission_hold
    }
    /// How long a permission ask may go unanswered before Fleet ends the
    /// Drone and escalates the Job. See
    /// [`crate::permitting::UnansweredAskLimit`].
    pub(crate) fn unanswered_ask_limit(&self) -> crate::permitting::UnansweredAskLimit {
        self.unanswered_ask_limit
    }
    /// What this Job may spend: the composition root's constant, then
    /// `armada.yml`'s `drone.cost_cap_micros_per_job`, then the Job's own
    /// column. `Allowance::at` is where the order is written.
    ///
    /// **There is no accessor for the machine tier on its own, and that is the
    /// point.** One shipped beside this for a day and had no callers by the end
    /// of it: every reader wants what a *Job* may spend, and the one that took
    /// the constant instead was drawing the wrong cap on the detail of a Job
    /// carrying an override. A tier is not a question anything asks.
    ///
    /// **Not [`Fleet::budget`]**, which is how long one Check may take — the
    /// two words collided before either shipped and the names are kept apart on
    /// purpose.
    pub(crate) fn allowance_for(&self, job: &Job) -> Allowance {
        // A Job no served repository owns is held to the machine's tier alone.
        match self.served_by(job) {
            Ok(served) => self.allowance.at(served.manifest(), job),
            Err(_) => self.allowance,
        }
    }
    /// What a Job may spend on this machine before any Manifest says otherwise.
    pub(crate) fn machine_allowance(&self) -> Allowance {
        self.allowance
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
    pub(crate) fn fixes(&self) -> crate::fixing::Fixes {
        self.fixes
    }
    pub(crate) fn fixing_on_main(&self) -> &Mutex<std::collections::BTreeSet<String>> {
        &self.fixing_on_main
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
    pub(crate) fn judging(
        &self,
        job: &Job,
        served: &crate::repositories::Served,
    ) -> Result<Judging, SpawnConfigRefused> {
        Ok(Judging {
            client: Arc::clone(&self.judge),
            budget: self.judge_budget,
            default_model: self.judge_model.clone(),
            second_opinion_model: self.second_opinion_model.clone(),
            environment: environment(
                HostPaths {
                    path: &self.host.path,
                    user: &self.host.user,
                    home: &self.host.home,
                },
                // No worktree, no Manifest ports resolved against one -- a
                // Judge and a proposer call carry neither.
                &[],
            )?,
            marking: Marking::on(
                job.id().into(),
                self.aloft.clone(),
                self.events.clone(),
                Arc::clone(&self.clock),
                self.judge_budget,
            ),
            asked: Asked::under(served.records_root().to_string(), job.handle()),
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
    /// Each Job's gate that is running Checks, for `serving` to put on
    /// `get_job` and to resolve a live log's name against. **Read, never
    /// written, from here** — [`announcing`](Self::announcing) hands out the
    /// only writer.
    pub(crate) fn underway(&self) -> &Underway {
        &self.underway
    }
    /// Where one gate says what each of its Checks is doing.
    ///
    /// **The step and the run are bound here**, because they name the entry a
    /// surface reads and the file each live log is written to — and a writer
    /// that was told them per call could be told two different ones.
    pub(crate) fn announcing(
        &self,
        served: &crate::repositories::Served,
        job: &Job,
        step: &core_model::StepId,
        attempt: core_model::Attempt,
    ) -> Announcing {
        Announcing::on(
            job.id().into(),
            step.clone(),
            attempt,
            self.underway.clone(),
            self.events.clone(),
            Arc::clone(&self.clock),
            served.records_root(),
            &job.handle(),
        )
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
            environment: environment(
                HostPaths {
                    path: &self.host.path,
                    user: &self.host.user,
                    home: &self.host.home,
                },
                // No worktree, no Manifest ports resolved against one -- a
                // Judge and a proposer call carry neither.
                &[],
            )?,
        })
    }

    /// What a reading of one blocked command needs in order to ask.
    ///
    /// **The Judge's client, budget and dial, rather than a fourth set.** The
    /// call is the same call — one turn, no toolset, no directory — and the
    /// model is the same cheap one, which `crates/ipc/operations.toml` says
    /// outright: the dial `judge_model` already derives, and not a fourth
    /// spelling of a vendor's name. What it does not take is a Job, because a
    /// reading is marked nowhere and filed nowhere.
    pub(crate) fn explaining(&self) -> Result<Explaining, SpawnConfigRefused> {
        Ok(Explaining {
            client: Arc::clone(&self.judge),
            budget: self.judge_budget,
            model: self.judge_model.clone(),
            environment: environment(
                HostPaths {
                    path: &self.host.path,
                    user: &self.host.user,
                    home: &self.host.home,
                },
                // No worktree, for `judging`'s reason one call over.
                &[],
            )?,
        })
    }

    /// What the dispatch path needs in order to be watched while it asks.
    ///
    /// **Beside `proposing` rather than inside it**, because the two answer
    /// different questions and one of them is optional. `Proposing` is how to
    /// make the call — the client, the model, the confinement — and is the same
    /// whether anybody is looking. This is who to tell and what may stop it,
    /// and a Fleet driven by a test with no stream still makes the call.
    ///
    /// `by` is who `ProposalMoved` is published against — the caller, never
    /// Fleet: the call is Fleet's to make, not Fleet's to have asked for.
    /// `#943`.
    pub(crate) fn making(&self, by: core_model::Actor) -> Watching {
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
            actor: by,
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
    /// What reads a repository's CI configuration, for Scan.
    pub(crate) fn ci_configuration(&self) -> &Arc<dyn CiConfiguration + Send + Sync> {
        &self.ci_configuration
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
    pub(crate) fn helm(&self) -> &crate::helm::Conversations {
        &self.helm
    }
    /// The roster, for `dispatch` and for a turn.
    pub(crate) fn slots(&self) -> &Mutex<Slots> {
        &self.slots
    }

    /// The machine reader and the headroom in force: what admission asks before a
    /// Drone starts, and what [`Fleet::room`] asks before a Check does.
    pub(crate) fn machine(&self) -> &Arc<dyn Machine> {
        &self.machine
    }
    /// How many Checks run at once on this machine, in force. #284.
    pub(crate) fn checks_at_once(&self) -> crate::places::ChecksAtOnce {
        self.places.at_once()
    }
    /// Put a saved Checks-at-once in force. `crate::limits`' alone.
    pub(crate) fn rechecked(&self, at_once: crate::places::ChecksAtOnce) {
        self.places.limit(at_once);
    }

    /// Put a width in force after a saved Jobs bound moved — the two are one
    /// number, so `crate::limits` sets this wherever it rebounds the roster.
    /// #1444.
    pub(crate) fn rewidened(&self, bound: crate::slots::Concurrency) {
        self.places
            .widen(checks_runner::CheckWidth::read(bound.jobs()));
    }
    /// A room in the machine's one line of places, for whoever is `asking`,
    /// with the headroom as it stands now. #284, #1063.
    pub(crate) fn room(&self, asking: crate::places::Asking) -> crate::places::Room {
        crate::places::Room::sharing(
            &self.places,
            asking,
            Arc::clone(&self.machine),
            self.headroom(),
            self.places.width(),
        )
    }
    /// [`Fleet::room`], knowing how long this Job's repository's Checks took
    /// before. A store that will not read starts them in Manifest order. #1062.
    pub(crate) async fn checks_room_for(
        &self,
        job: &Job,
        asking: crate::places::Asking,
    ) -> crate::places::Room {
        let past = self
            .store()
            .lock()
            .await
            .check_timings(job.owner_manifest_id())
            .unwrap_or_default();
        self.room(asking).knowing(crate::ordering::Past::of(past))
    }
    /// Keep how long each Check of one of this Job's runs took. A row that will
    /// not write fails nothing: the run's own answer is already out.
    pub(crate) async fn kept_timings(&self, job: &Job, timed: Vec<(String, std::time::Duration)>) {
        self.kept_repository_timings(job.owner_manifest_id(), timed)
            .await;
    }
    /// [`Fleet::kept_timings`], against a repository rather than a Job. A
    /// proof has no Job to key by — it runs against the commit that merged,
    /// so it records against the repository the commit belongs to. #1072.
    pub(crate) async fn kept_repository_timings(
        &self,
        repository: &ManifestId,
        timed: Vec<(String, std::time::Duration)>,
    ) {
        if timed.is_empty() {
            return;
        }
        let at = self.now();
        let mut store = self.store().lock().await;
        for (check, took) in timed {
            let _ = store.record_check_took(repository, &check, took, &at);
        }
    }
    /// A fix draft's one-test run, kept apart from a whole Check's. #1072.
    pub(crate) async fn kept_one_test_timing(
        &self,
        repository: &ManifestId,
        check: &str,
        took: std::time::Duration,
    ) {
        let at = self.now();
        let mut store = self.store().lock().await;
        let _ = store.record_one_test_took(repository, check, took, &at);
    }
    /// Where a Drone's own run says what each of its Checks is doing: shown
    /// beside the gate's and never as it, and heard as each result lands.
    /// `whole` is whether it ran every Check whole, which is when it is timed.
    pub(crate) fn announcing_dry_run(
        &self,
        job: &Job,
        step: &core_model::StepId,
        attempt: core_model::Attempt,
        hearing: tokio::sync::mpsc::UnboundedSender<crate::underway::Landed>,
        whole: bool,
    ) -> Announcing {
        Announcing::dry_run(
            job.id().into(),
            step.clone(),
            attempt,
            self.underway.clone(),
            self.events.clone(),
            Arc::clone(&self.clock),
            hearing,
            whole,
        )
    }
    /// The headroom in force, **by value**: a save replaces it, so a borrow
    /// would be a lock held across whatever the caller did next.
    pub(crate) fn headroom(&self) -> Headroom {
        *self
            .headroom
            .lock()
            .expect("the headroom lock is not poisoned")
    }
    /// Put a saved headroom in force. `crate::limits`' alone.
    pub(crate) fn rehoused(&self, headroom: Headroom) {
        *self
            .headroom
            .lock()
            .expect("the headroom lock is not poisoned") = headroom;
    }
    /// The limits the composition root handed in, before anything was saved.
    pub(crate) fn shipped(&self) -> crate::limits::Limits {
        self.shipped
    }
    pub(crate) fn polling(&self) -> Polling {
        self.polling
    }
    pub(crate) fn proving(&self) -> &Arc<Mutex<crate::proving::Proving>> {
        &self.proving
    }
    pub(crate) fn seeds(&self) -> &Arc<std::sync::Mutex<crate::seeding::Seeds>> {
        &self.seeds
    }
    pub(crate) fn base_preparing(&self) -> &Arc<tokio::sync::Mutex<()>> {
        &self.base_preparing
    }
    pub(crate) fn copy_on_write(&self) -> &Arc<dyn crate::seeding::CopyOnWrite> {
        &self.copy_on_write
    }
    pub(crate) fn pressing(&self) -> &crate::showing_again::Pressing {
        &self.pressing
    }
    pub(crate) fn rechecking(&self) -> &crate::rechecking::Rechecking {
        &self.rechecking
    }
    pub(crate) fn rehearsals(&self) -> &crate::rehearsing::Rehearsals {
        &self.rehearsals
    }
    pub(crate) fn servers(&self) -> &crate::servers::Servers {
        &self.servers
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
    pub(crate) fn peering(&self) -> &Mutex<crate::peers::Peering> {
        &self.peering
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
