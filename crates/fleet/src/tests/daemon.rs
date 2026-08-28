//! A Job, driven end to end, against fakes.
//!
//! # What is real here and what is not
//!
//! **Real: the store, the machines, the gate, the process.** Every transition
//! goes through `Job::transition`, every one is written to a real SQLite file,
//! and the Drone is a real detached child — `/bin/cat`, which holds its input
//! open and is therefore a Drone that can be spoken to. The store is reopened
//! at the end of the first case and the Job is folded back out of its events,
//! which is the only assertion that proves the loop wrote a history rather than
//! a state.
//!
//! **Not real: the agent, the repository, the diff.** Starting an agent costs
//! money, needs a network and needs a credential, and a suite with any of those
//! in it is a suite people stop running. Whether the *real* argument list
//! confines a Drone is asserted in `adapters`, with no process at all.
//!
//! # The one thing a test has to do that Fleet does not
//!
//! `FakeVcs` creates nothing on disk, deliberately — and a detached child needs
//! a working directory that is really there. So each case below makes the
//! derived directory itself, from the same `WorktreeSpec` Fleet will derive,
//! between proposing and approving. Deriving it a second way would be the
//! second-vocabulary defect in a test.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Model, WorktreeSpec};
use config::{EvidenceType, Manifest};
use core_model::{JobStatus, StepState, Timestamp, Ulid};
use store::Store;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Gate, Sketch};
use verification::{Claimed, NotClaimed, ShownBy};

use crate::clock::Clock;
use crate::converging::StepNorms;
use crate::daemon::{Fittings, Fleet, Host};
use crate::evidence::Call;
use crate::gate::{CheckBudget, Ruling};
use crate::judging::JudgeBudget;
use crate::mint::Mint;
use crate::tests::tmp::TempDir;

/// A clock that answers a different second each time it is asked.
///
/// Different, so the pair of timestamps on a step row says how long the step
/// took; fixed in shape, so a test can write the answer down.
pub struct Ticking(AtomicU64);

impl Ticking {
    pub fn from_nine() -> Ticking {
        Ticking(AtomicU64::new(0))
    }
}

impl Clock for Ticking {
    fn now(&self) -> Timestamp {
        let tick = self.0.fetch_add(1, Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-08-26T09:{:02}:{:02}.000Z",
            tick / 60,
            tick % 60
        ))
    }
}

/// Ids a test can write down. Twenty-six characters, and every one of them
/// legal in a directory name and in a branch name, because `WorktreeSpec`
/// refuses anything else.
pub struct Counted(AtomicU64);

impl Counted {
    pub fn from_one() -> Counted {
        Counted(AtomicU64::new(1))
    }

    /// A real mint is a ULID and cannot repeat; this one restarts at one on
    /// every assembly, so two Fleets over one store would hand out one id twice.
    pub fn from_next(next: u64) -> Counted {
        Counted(AtomicU64::new(next))
    }
}

impl Mint for Counted {
    fn ulid(&self) -> Ulid {
        Ulid::carried(format!(
            "01TEST{:020}",
            self.0.fetch_add(1, Ordering::SeqCst)
        ))
    }
}

/// Two steps: one gated on a non-empty diff, one gated on nothing.
///
/// The second is the shape the gate's own comment calls common rather than
/// edge — a step with no checks advances on evidence alone, and two of the four
/// sample workflows lean on it.
fn two_steps() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "summarise",
            label: "Summarise",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

/// Two steps, each gated on a non-empty diff. What the boundary needs: a
/// second step whose `diff_nonempty` can fail, which `two_steps` cannot express
/// because its second step is gated on nothing.
fn two_steps_both_gated_on_a_diff() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "verify",
            label: "Verify",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

/// The same two steps, with a person on the gate of the one named, and
/// optionally one question to the Judge on that same step.
///
/// **Written as a file rather than through `testkit::Sketch`**, which fixes a
/// step's gate from whether it declares a criterion and so cannot express a
/// human gate at all. It goes through both parsers for the reason every other
/// fixture does: a workflow a real `armada.yml` could not produce would prove
/// nothing about the gate — and a human gate carrying a Judge is exactly the
/// pairing the parser's agreement rule is asked about.
pub fn two_steps_gated_on_a_person(
    gate_on: &str,
    question: Option<&str>,
) -> config::ResolvedWorkflow {
    let gate = |step: &str| match step == gate_on {
        true => "human_always",
        false => "auto",
    };
    let judge = match question {
        None => String::new(),
        Some(question) => format!(
            "    judge_checks:\n      - model: haiku\n        criteria:\n          - \
             criterion_id: c1\n            question: {question}\n"
        ),
    };
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        &format!(
            "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\n\
             steps:\n  - id: implement\n    label: \"Implement\"\n    evidence_type: diff\n    \
             mechanical_checks:\n      - type: diff_nonempty\n{judge}    advance_gate: {}\n  - \
             id: summarise\n    label: \"Summarise\"\n    evidence_type: facts_note\n    \
             advance_gate: {}\n",
            gate("implement"),
            gate("summarise"),
        ),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest())
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

/// One workflow, keyed by its own id — what `fittings` holds when a case
/// wants nothing more than the fixture.
pub fn one(
    workflow: config::ResolvedWorkflow,
) -> BTreeMap<core_model::WorkflowId, config::ResolvedWorkflow> {
    let mut held = BTreeMap::new();
    held.insert(workflow.id().clone(), workflow);
    held
}

/// A one-step workflow with its own `workflow_id`, gated on nothing but the
/// evidence arriving — for a case that needs two workflows Fleet can tell
/// apart by id, and cannot tell apart by step id, since each step is named
/// for the workflow that declares it.
pub fn workflow_named(id: &str) -> config::ResolvedWorkflow {
    workflow_named_and(id, false)
}

/// The same, gated on a non-empty diff — for a case that needs a Check that
/// can fail.
pub fn workflow_named_gated_on_diff(id: &str) -> config::ResolvedWorkflow {
    workflow_named_and(id, true)
}

fn workflow_named_and(id: &str, gated: bool) -> config::ResolvedWorkflow {
    let step_id = format!("only_in_{id}");
    let mechanical = if gated {
        "    mechanical_checks:\n      - type: diff_nonempty\n"
    } else {
        ""
    };
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        &format!(
            "version: 1\nworkflow_id: {id}\nname: {id}\nstructure: linear\nsteps:\n  - id: \
             {step_id}\n    label: \"{step_id}\"\n    evidence_type: diff\n{mechanical}    \
             advance_gate: auto\n"
        ),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    config::ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

fn manifest() -> Manifest {
    Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("a manifest that parses")
}

/// Norms no fixture trips. A step's turn count, its wall clock and the grace
/// after a forced report are all put out of reach, so every test but
/// `converging`'s own behaves exactly as it did before the chain existed.
pub const UNTRIPPABLE: StepNorms = StepNorms::of(
    u32::MAX,
    Duration::from_secs(86_400),
    Duration::from_secs(86_400),
);

/// Everything a Fleet is assembled from, over one temporary directory.
pub fn fittings(
    home: &TempDir,
    work: FakeWorkProduct,
) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    fitted_with(home, work, FakeHarness::that_listens())
}

pub fn fitted_with(
    home: &TempDir,
    work: FakeWorkProduct,
    harness: FakeHarness,
) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    let root = home.path().to_string_lossy().to_string();
    Fittings {
        store: Store::open(&home.path().join("armada.db")).expect("a store"),
        harness,
        vcs: FakeVcs::new(),
        work,
        clock: Arc::new(Ticking::from_nine()),
        mint: Arc::new(Counted::from_one()),
        workflows: one(two_steps()),
        manifest: manifest(),
        host: Host {
            user: String::from("someone"),
            repo_root: root.clone(),
            path: "/usr/bin:/bin".to_string(),
            home: root,
            mcp_config: "/etc/armada/mcp.json".to_string(),
            attachments_dir: home
                .path()
                .join("attachments")
                .to_string_lossy()
                .to_string(),
        },
        budget: CheckBudget::of(Duration::from_secs(5)),
        norms: UNTRIPPABLE,
        // A Judge that fails every call, because no step in these fixtures
        // declares a criterion. One that answered would let a cold-by-default
        // regression pass unseen.
        judge: Arc::new(FakeJudge::that_fails("a Judge that should never be asked")),
        judge_budget: JudgeBudget::of(Duration::from_secs(5)),
        judge_model: Model::named("the-cheap-model").expect("a model name"),
        proposer_model: Model::named("the-cheap-model").expect("a model name"),
        // Planted, not read. The composition root resolves these from the
        // environment and the adapter; a test that read the same sources would
        // be asserting against a machine rather than against Fleet.
        models: ipc::ModelChoices {
            models: vec!["a-model".to_string(), "another-model".to_string()],
            default: "a-model".to_string(),
        },
        events: api::Broadcaster::new(),
    }
}

/// A Fleet whose Drone holds its input open, so the gate has something to speak
/// to when a step advances.
pub fn a_fleet(
    home: &TempDir,
    work: FakeWorkProduct,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    Fleet::assembled(fittings(home, work))
}

/// A Fleet whose version control is scripted. What `landing` needs: the commit
/// a finished Job gets is the fake's to record, refuse, or answer as nothing,
/// and what `delivery` needs: where the branch stands and what a push answers.
pub fn a_fleet_committing_through(
    home: &TempDir,
    work: FakeWorkProduct,
    vcs: FakeVcs,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.vcs = vcs;
    Fleet::assembled(fittings)
}

/// A Fleet whose workflow puts a person on one step's gate, committing through
/// this version control — the second half is what the last-step case needs,
/// since approving there lands the work.
pub fn a_fleet_gated_on_a_person(
    home: &TempDir,
    work: FakeWorkProduct,
    gate_on: &str,
    vcs: FakeVcs,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = one(two_steps_gated_on_a_person(gate_on, None));
    fittings.vcs = vcs;
    Fleet::assembled(fittings)
}

/// A Fleet over an `armada.yml` that names the branch its work merges into.
pub fn a_fleet_whose_manifest_declares_a_base(
    home: &TempDir,
    work: FakeWorkProduct,
    vcs: FakeVcs,
    base: &str,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.vcs = vcs;
    fittings.manifest = Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\nbase: {base}\n"),
    )
    .expect("a manifest that parses");
    Fleet::assembled(fittings)
}

/// A Fleet over a store another Fleet wrote to, holding a workflow of its own.
///
/// The pair of arguments is what an edited `.armada/workflows/` looks like from
/// inside the process: Fleet reads the file once at assembly, so a different
/// definition and a restart are the same event.
pub fn a_fleet_holding(
    home: &TempDir,
    work: FakeWorkProduct,
    workflow: config::ResolvedWorkflow,
    next: u64,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = one(workflow);
    fittings.mint = Arc::new(Counted::from_next(next));
    Fleet::assembled(fittings)
}

/// A Fleet holding every one of these workflows, keyed by each one's own id —
/// what an `.armada/workflows/` with more than one definition looks like from
/// inside the process.
pub fn a_fleet_holding_all(
    home: &TempDir,
    work: FakeWorkProduct,
    workflows: Vec<config::ResolvedWorkflow>,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = workflows
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    Fleet::assembled(fittings)
}

/// [`a_proposal`], naming a workflow other than the fixture's own.
pub fn a_proposal_for(title: &str, workflow_id: &str) -> ipc::ProposeJob {
    let mut proposal = a_proposal(title);
    proposal.workflow_id = ipc::WorkflowId::carried(workflow_id);
    proposal
}

/// A Fleet whose steps are judged, and by whom.
///
/// The workflow is an argument because the Judge is cold by default: the
/// fixture workflow declares no criterion, so a Fleet that only swapped the
/// client would never make a call.
pub fn a_fleet_judged_by(
    home: &TempDir,
    work: FakeWorkProduct,
    workflow: config::ResolvedWorkflow,
    judge: FakeJudge,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = one(workflow);
    fittings.judge = Arc::new(judge);
    Fleet::assembled(fittings)
}

/// A Fleet whose dispatch requests are read by this client, holding these
/// workflows.
///
/// The workflows are an argument because they are half of what the proposer is
/// told: a catalogue it cannot choose from proves nothing about it choosing.
pub fn a_fleet_proposing_through(
    home: &TempDir,
    work: FakeWorkProduct,
    workflows: Vec<config::ResolvedWorkflow>,
    proposer: FakeJudge,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.workflows = workflows
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = Arc::new(proposer);
    Fleet::assembled(fittings)
}

/// A Fleet over a store another Fleet wrote to. See [`Counted::from_next`].
pub fn a_fleet_minting_from(
    home: &TempDir,
    work: FakeWorkProduct,
    next: u64,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, work);
    fittings.mint = Arc::new(Counted::from_next(next));
    Fleet::assembled(fittings)
}

/// A Fleet whose Drone reads its first turn, prints it back and exits — which
/// is a Drone that finished having submitted nothing.
fn a_fleet_whose_drone_leaves(
    home: &TempDir,
    work: FakeWorkProduct,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    Fleet::assembled(fitted_with(
        home,
        work,
        FakeHarness::that_echoes_its_first_turn(),
    ))
}

pub fn a_proposal(title: &str) -> ipc::ProposeJob {
    ipc::ProposeJob {
        title: title.to_string(),
        // The values the fixture Fleet actually holds. Naming anything else is
        // refused at creation now, which is the point.
        workflow_id: ipc::WorkflowId::carried("fixture-workflow"),
        owner_manifest_id: ipc::ManifestId::carried("01FIXTUREMANIFEST"),
        origin: ipc::TopLevelOrigin::from_wire("manual").expect("an origin"),
        urgency: ipc::Urgency::from_wire("normal").expect("an urgency"),
        atomic: false,
        model: Some("a-model".to_string()),
        acceptance_criteria: vec![ipc::ProposedCriterion {
            text: "the symptom is gone".to_string(),
            source: ipc::CriterionSource::from_wire("check").expect("a source"),
        }],
        subject: None,
        dependencies: Vec::new(),
        facts: "the reader is off by one".to_string(),
        write_targets: None,
        attachments: Vec::new(),
    }
}

/// Make the directory `FakeVcs` says it made. See this module's comment.
pub fn worktree_directory(home: &TempDir, job: &core_model::JobId) {
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), job.as_str()).expect("a legal spec");
    std::fs::create_dir_all(spec.worktree_path()).expect("a directory for the Drone to run in");
}

pub fn diff_evidence() -> Call<'static> {
    Call {
        evidence_type: EvidenceType::Diff,
        claimed: Claimed("The reader stops one line later."),
        shown_by: ShownBy("src/log.rs, six lines"),
        not_claimed: NotClaimed("The writer has the same bug and is untouched."),
    }
}

pub fn note_evidence() -> Call<'static> {
    Call {
        evidence_type: EvidenceType::FactsNote,
        claimed: Claimed("The cause was an inclusive bound."),
        shown_by: ShownBy("`.armada/root-cause.md`, written this step"),
        not_claimed: NotClaimed(""),
    }
}

/// The whole of it: created, approved, worktree, Drone, evidence, checks,
/// advance, advance, complete — and then read back out of a reopened store.
#[tokio::test]
async fn a_job_is_driven_from_created_to_completed_and_survives_a_reopen() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .unwrap();
    assert_eq!(job.status(), JobStatus::AwaitingApproval);
    assert_eq!(job.steps().len(), 2, "the frozen workflow's steps");
    worktree_directory(&home, job.id());

    let approved = fleet.approve(job.id()).await.unwrap();
    assert_eq!(approved.status(), JobStatus::Running);
    assert_eq!(fleet.working_on().await.as_ref(), Some(job.id()));
    assert_eq!(
        approved.current_step_id().map(|id| id.as_str()),
        Some("implement")
    );

    // The receipt is not a verdict: nothing has been decided at this point,
    // which is the whole reason the inbox exists.
    fleet.submit_evidence(diff_evidence()).await.unwrap();
    assert_eq!(fleet.evidence_waiting(), 1);
    assert_eq!(
        fleet
            .load(job.id())
            .await
            .unwrap()
            .step(&core_model::StepId::new("implement"))
            .unwrap()
            .state(),
        StepState::Running,
        "a submission alone advances nothing"
    );

    let turned = fleet.turn().await.unwrap();
    assert!(
        matches!(turned.ruled, Some(Ruling::Advanced { .. })),
        "the diff was non-empty and the step declared no other check"
    );
    let midway = fleet.load(job.id()).await.unwrap();
    assert_eq!(
        midway.status(),
        JobStatus::Running,
        "a step is the inner machine"
    );
    assert_eq!(
        midway.current_step_id().map(|id| id.as_str()),
        Some("summarise")
    );

    fleet.submit_evidence(note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(matches!(turned.ruled, Some(Ruling::Finished { .. })));
    assert_eq!(fleet.working_on().await, None, "the slot came free");

    drop(fleet);
    let mut reopened = Store::open(&home.path().join("armada.db")).expect("the same store");
    let loaded = reopened.load_all_jobs().expect("every Job folds");
    let same = loaded
        .jobs
        .iter()
        .find(|held| held.id() == job.id())
        .unwrap();
    assert_eq!(same.status(), JobStatus::CompletedSuccess);
    assert!(
        same.steps()
            .iter()
            .all(|step| step.state() == StepState::Advanced),
        "both steps passed their advance gate"
    );
    assert_eq!(
        same.current_step_id().map(|id| id.as_str()),
        Some("summarise"),
        "the cursor is never cleared — a finished Job still points at its last step"
    );
    assert!(
        loaded.repaired.is_empty(),
        "no cached status disagreed with the log"
    );
}

/// A failed Check ends the Job, and the worktree stays exactly where it is.
#[tokio::test]
async fn a_failed_check_ends_the_job_and_keeps_the_worktree() {
    let home = TempDir::new();
    // Nothing changed, so `diff_nonempty` fails. A reading that failed and a
    // diff that was empty are different things, and this is the second.
    let fleet = a_fleet(&home, FakeWorkProduct::untouched());

    let job = fleet.propose(a_proposal("change nothing")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet.submit_evidence(diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    let Some(Ruling::Failed { failures, .. }) = turned.ruled else {
        panic!("an empty diff is a failed check");
    };
    assert_eq!(failures.len(), 1);

    let ended = fleet.load(job.id()).await.unwrap();
    assert_eq!(ended.status(), JobStatus::CompletedFailed);
    assert_eq!(fleet.working_on().await, None);

    let spec = WorktreeSpec::for_job(&home.path().to_string_lossy(), job.id().as_str()).unwrap();
    assert!(
        std::path::Path::new(&spec.worktree_path()).exists(),
        "the worktree is kept — nothing in this workspace can remove one"
    );
}

/// A Job approved while another is being worked sits at `queued`, and starts
/// when the slot comes free.
#[tokio::test]
async fn a_second_approved_job_waits_while_one_is_worked() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let first = fleet.propose(a_proposal("the first")).await.unwrap();
    worktree_directory(&home, first.id());
    fleet.approve(first.id()).await.unwrap();

    let second = fleet.propose(a_proposal("the second")).await.unwrap();
    worktree_directory(&home, second.id());
    let waiting = fleet.approve(second.id()).await.unwrap();

    assert_eq!(
        waiting.status(),
        JobStatus::Queued,
        "approved, and not started"
    );
    assert_eq!(fleet.working_on().await.as_ref(), Some(first.id()));

    // Finish the first. Its slot is what the second was waiting for, and
    // nothing else about it changed.
    fleet.submit_evidence(diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    fleet.submit_evidence(note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();

    assert_eq!(
        turned.admitted.as_ref(),
        Some(second.id()),
        "the queue is the store's `queued` status, and it emptied by one"
    );
    assert_eq!(fleet.working_on().await.as_ref(), Some(second.id()));
    assert_eq!(
        fleet.load(first.id()).await.unwrap().status(),
        JobStatus::CompletedSuccess
    );
}

/// A Job the store says was `running` whose Drone this Fleet does not have is
/// `interrupted`. **Never resumed silently.**
#[tokio::test]
async fn a_running_job_with_no_drone_is_interrupted_at_startup() {
    let home = TempDir::new();
    let job_id = {
        let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
        let job = fleet
            .propose(a_proposal("interrupted mid-flight"))
            .await
            .unwrap();
        worktree_directory(&home, job.id());
        fleet.approve(job.id()).await.unwrap();
        job.id().clone()
    };

    // A second Fleet over the same store. It holds no Drone, because a Drone is
    // held in memory by the Fleet that spawned it.
    let restarted = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let reconciled = restarted.reconcile().await.unwrap();

    assert_eq!(reconciled.interrupted, vec![job_id.clone()]);
    assert!(reconciled.unreadable.is_empty());
    assert_eq!(
        restarted.load(&job_id).await.unwrap().status(),
        JobStatus::Escalated
    );
    assert_eq!(
        restarted.last_reason(&job_id).await.unwrap(),
        Some(core_model::TransitionReason::Escalation(
            core_model::EscalationTrigger::Interrupted
        )),
        "the trigger the registry gives for a Job marked running with no process"
    );
    assert_eq!(
        restarted.working_on().await,
        None,
        "an interrupted Job is not picked back up"
    );
}

/// The wrong kind of evidence spends no Check and moves nothing.
#[tokio::test]
async fn a_submission_of_the_wrong_kind_moves_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet.propose(a_proposal("the wrong kind")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    // The first step asks for a diff.
    fleet.submit_evidence(note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();

    assert!(matches!(turned.ruled, Some(Ruling::NotWhatTheStepAsked(_))));
    let unmoved = fleet.load(job.id()).await.unwrap();
    assert_eq!(unmoved.status(), JobStatus::Running);
    assert_eq!(
        unmoved.current_step_id().map(|id| id.as_str()),
        Some("implement"),
        "nothing ran and nothing moved"
    );
    assert_eq!(fleet.working_on().await.as_ref(), Some(job.id()));
}

/// Killing a Job ends it, wherever it stood, and frees the slot.
#[tokio::test]
async fn killing_a_job_ends_it_and_frees_the_slot() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet.propose(a_proposal("kill me")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    let killed = Fleet::kill_job(&fleet, job.id()).await.unwrap();
    assert_eq!(killed.status(), JobStatus::Killed);
    assert!(killed.status().is_terminal());
    assert_eq!(fleet.working_on().await, None);
}

/// **No aftermath leaves a Job running.** A Drone that finished having
/// submitted nothing pauses the Job for a person, and the slot comes free.
///
/// The wait below is for the operating system rather than for Fleet: the child
/// has to actually exit and close its pipe before there is anything to reap,
/// and no amount of asking earlier changes that.
#[tokio::test]
async fn a_drone_that_leaves_without_submitting_does_not_leave_the_job_running() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_drone_leaves(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("say nothing and go"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    let mut after = None;
    for _ in 0..200 {
        let turned = fleet.turn().await.unwrap();
        if turned.after.is_some() {
            after = turned.after;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let Some(crate::Aftermath::JobMoves(target)) = after else {
        panic!("a Drone that is gone having left nothing moves the Job");
    };
    assert_eq!(target.status(), JobStatus::Escalated);
    let paused = fleet.load(job.id()).await.unwrap();
    assert_eq!(paused.status(), JobStatus::Escalated);
    assert!(
        !paused.status().is_terminal(),
        "escalated holds the worktree until a person answers — it does not end the Job"
    );
    assert_eq!(fleet.working_on().await, None, "the slot came free");
}

/// **The boundary is read, and a step is gated on what it did rather than on
/// what the branch holds.**
///
/// A Job's first step writes a file and advances. Its second writes nothing,
/// submits well-formed Evidence, and must fail — before this, it was credited
/// with the first step's file and advanced `passed`, which made every step
/// after the first that wrote anything pass `diff_nonempty` for free.
///
/// The only thing standing between the two submissions is `Fleet::turn`, so
/// what is under test is the boundary reading the worktree — not the gate,
/// which `tests::gate` asks directly.
#[tokio::test]
async fn a_second_step_that_writes_nothing_is_not_credited_with_the_first_step_s_file() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::untouched());
    fittings.workflows = one(two_steps_both_gated_on_a_diff());
    let fleet = Fleet::assembled(fittings);

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    // The first step's Drone puts something on disk, then submits.
    fleet
        .work()
        .wrote(&[("src/log.rs", adapter_traits::Change::Modified)]);
    fleet.submit_evidence(diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(
        matches!(turned.ruled, Some(Ruling::Advanced { .. })),
        "the first step wrote a file: {:?}",
        turned.ruled
    );
    assert_eq!(
        fleet
            .load(job.id())
            .await
            .unwrap()
            .current_step_id()
            .map(|id| id.as_str()),
        Some("verify")
    );

    // The second step's Drone writes nothing at all and submits anyway.
    fleet.submit_evidence(diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    let Some(Ruling::Failed { failures, .. }) = &turned.ruled else {
        panic!(
            "the second step advanced having written nothing: {:?}",
            turned.ruled
        );
    };
    assert_eq!(failures, &[verification::CheckFailed::DiffEmpty]);
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::CompletedFailed
    );
}
