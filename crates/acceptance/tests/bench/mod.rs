//! The bench the claim is asserted against: two injected seams, a workflow
//! fixture and a Job that can only be moved by transitioning.
//!
//! Separate from `bug_job.rs` because it is a different subject. That file is
//! what the milestone claims; this is the apparatus, and none of it asserts
//! anything.
//!
//! # Nothing here reaches the world
//!
//! No process, no repository, no network, no file. The clock and the mint are
//! planted, the version control creates nothing on disk and the work product is
//! whatever the test said it is.
//!
//! # Why the clock and the mint are written out again here
//!
//! `fleet`'s own tests carry a `Ticking` and a `Counted` that do the same job.
//! They cannot be shared: [`Clock`] and [`Mint`] are declared in `fleet`, and
//! `testkit` — the crate that would hold a shared fixture — sits *below* `fleet`
//! and cannot implement a trait it cannot see. Every test that wants a fixed
//! clock therefore writes its own. Named here as a gap rather than left to be
//! discovered as a coincidence.

/// Focus's own apparatus, which reaches this module's private seams rather
/// than making a second copy of them. See its own header.
pub mod focus;

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use std::sync::Arc;

use adapter_traits::{Environment, Footprint, Model, Vcs, Worktree, WorktreeSpec};
use config::{EvidenceType, ResolvedWorkflow};
use core_model::{
    AcceptanceCriterion, Actor, CriterionId, CriterionSource, DeclaredPaths, Facts,
    IllegalStepTransition, IllegalTransition, Job, JobEvent, JobId, JobStatus, JobStep, ManifestId,
    ModelName, NewJob, StepEvent, StepEvidence, StepId, StepSeed, StepState, StepTarget, Target,
    Timestamp, Title, TopLevelOrigin, TransitionReason, Ulid, Urgency,
};
use fleet::dispatch::stopping;
use fleet::{
    apply, rule_on, AtStep, CheckBudget, Clock, JudgeBudget, Judging, Keeping, Marking, Mint,
    Ruling,
};
use testkit::{resolved, FakeJudge, FakeVcs, FakeWorkProduct, Gaming, Gate, Sketch};
use verification::{Claimed, NotClaimed, Request, ShownBy, Submission};

/// Absolute, because `WorktreeSpec` refuses a relative root — a derived path
/// that moves with the caller is the stored-path failure in another shape.
pub const REPO_ROOT: &str = "/repos/armada";

// ---------------------------------------------------------------------------
// The two seams the whole system takes as arguments
// ---------------------------------------------------------------------------

/// A clock that answers a different second each time it is asked.
///
/// Time is injected and never read. Nothing below `fleet` may call
/// `SystemTime::now`, and this is what pays for that rule here.
pub struct Ticking(AtomicU64);

impl Clock for Ticking {
    fn now(&self) -> Timestamp {
        let tick = self.0.fetch_add(1, Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-08-26T11:{:02}:{:02}.000Z",
            tick / 60,
            tick % 60
        ))
    }
}

/// Ids a test can write down, minted through the same trait Fleet mints
/// through. Twenty-six characters, every one legal in a directory and in a
/// branch name, because `WorktreeSpec` refuses anything else.
pub struct Counted(AtomicU64);

impl Mint for Counted {
    fn ulid(&self) -> Ulid {
        Ulid::carried(format!(
            "01BUG{:021}",
            self.0.fetch_add(1, Ordering::SeqCst)
        ))
    }
}

// ---------------------------------------------------------------------------
// The fixture
// ---------------------------------------------------------------------------

/// The Bug workflow, as far as M1's vocabulary expresses it.
///
/// `crates/core-model/domain/workflow-samples/bug.json` is the real one and it
/// has seven steps. **It does not load**, and `config`'s own sample tests
/// assert so: it declares `structure: linear` and carries `verdict_routing`,
/// which the structure rule rejects. Five of its seven steps need machinery M1
/// did not build — `test_run` and `pr_merged` are sanctioned check types with
/// no implementation, `review` is a Judge and there is no Judge, and `merge` is
/// a push nothing in this workspace can perform. `artifact_exists` was a third
/// such type and is not any more: it is carried, as a path in the worktree
/// rather than the registry name the sample writes.
/// `regression_verify` is expressible and is left out here for a different
/// reason: its check is a command, and running one would spawn.
///
/// What remains is two steps, under the names and in the order `bug.json` gives
/// them, gated as far as `bug.json` gates them.
pub fn bug_workflow_as_far_as_m1_expresses_it() -> ResolvedWorkflow {
    resolved(&[
        Sketch {
            id: "root_cause",
            label: "Root cause",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "fix",
            label: "Fix",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

/// The same workflow with the fix step gated on the Judge as well as on a
/// diff, so a run can show the semantic tier stopping work the mechanical tier
/// cleared.
pub fn bug_workflow_with_the_fix_judged() -> ResolvedWorkflow {
    resolved(&[
        Sketch {
            id: "root_cause",
            label: "Root cause",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "fix",
            label: "Fix",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[("c1", "Does the fix address the cause the note names?")],
            scope: None,
            gaming: None,
        },
    ])
}

/// The same workflow again, with the fix step watching for evidence that games
/// its Check rather than satisfying it. `check_config_edited` is the pattern
/// the diff answers, so this run reaches no model at all.
pub fn bug_workflow_watching_for_gaming() -> ResolvedWorkflow {
    resolved(&[
        Sketch {
            id: "root_cause",
            label: "Root cause",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "fix",
            label: "Fix",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
            gaming: Some(Gaming {
                baseline: Some("root_cause.evidence"),
                flag_if: &["check_config_edited", "test_deleted"],
            }),
        },
    ])
}

/// A diff that honours the frozen `run:` string exactly and narrows what it
/// runs. **The mechanical tier passes this by design.**
pub const A_NARROWED_GATE: &str = "diff --git a/jest.config.js b/jest.config.js\n\
     --- a/jest.config.js\n\
     +++ b/jest.config.js\n\
     -  testPathIgnorePatterns: [\"/node_modules/\"],\n\
     +  testPathIgnorePatterns: [\"/node_modules/\", \"/tests/edge-cases/\"],\n";

/// The yardstick, frozen at creation and never editable afterwards.
pub fn criteria() -> Vec<AcceptanceCriterion> {
    vec![
        AcceptanceCriterion {
            criterion_id: CriterionId::new("c1"),
            text: String::from("the reported symptom no longer occurs"),
            source: CriterionSource::Check,
        },
        AcceptanceCriterion {
            criterion_id: CriterionId::new("c2"),
            text: String::from("a test covers the reported symptom"),
            source: CriterionSource::Check,
        },
    ]
}

pub fn a_root_cause_note() -> Submission {
    Submission::submitted(
        EvidenceType::FactsNote,
        Claimed("The reader's bound is inclusive where the caller expects exclusive."),
        ShownBy("`read_to` stops at `end` rather than before it — read.rs:41"),
        NotClaimed("The writer has the same bound and is untouched."),
    )
    .expect("a well-formed root-cause submission")
}

pub fn a_fix_diff() -> Submission {
    Submission::submitted(
        EvidenceType::Diff,
        Claimed("The reader stops one line earlier."),
        ShownBy("crates/store/src/read.rs, six lines"),
        NotClaimed("The writer is unchanged."),
    )
    .expect("a well-formed diff submission")
}

// ---------------------------------------------------------------------------
// The bench
// ---------------------------------------------------------------------------

/// A Job that exists, and the worktree made for it.
pub struct Run {
    pub job: Job,
    pub worktree: Worktree,
    /// What the Drone declared this step's work would be in. `None` on a step
    /// that asks for no scope, which is every step of the bug workflow.
    pub declared: Option<DeclaredPaths>,
}

/// Everything the run needs, with no process, no repository and no network
/// under any of it.
pub struct Bench {
    clock: Ticking,
    mint: Counted,
    workflow: ResolvedWorkflow,
    pub vcs: FakeVcs,
    pub work: FakeWorkProduct,
    budget: CheckBudget,
    judging: Judging,
    /// Every event the two machines handed back, in order.
    ///
    /// **This is the whole record.** A `Job` carries no history — the log is
    /// the store's — so appending here as each transition returns is what lets
    /// a test assert that a Job arrived somewhere rather than was put there.
    pub moves: RefCell<Vec<JobEvent>>,
    step_moves: RefCell<Vec<StepEvent>>,
    /// What each step has submitted, which is what `store` holds for a real
    /// Job. A later step's gaming check reads it as its baseline, so a bench
    /// that dropped it would run every gaming check baseline-less and prove
    /// less than it looks like it does.
    recorded: RefCell<Vec<(StepId, StepEvidence)>>,
}

impl Bench {
    /// A bench whose Judge fails every call it is given.
    ///
    /// **The default**, because no step of the workflow below declares a
    /// criterion: a gate that asked anyway would answer `CouldNotDecide` and
    /// the run would stop, rather than advancing and saying nothing.
    pub fn with(work: FakeWorkProduct) -> Bench {
        Bench::judged_by(
            work,
            bug_workflow_as_far_as_m1_expresses_it(),
            FakeJudge::that_fails("a Judge that should never be asked"),
        )
    }

    pub fn judged_by(work: FakeWorkProduct, workflow: ResolvedWorkflow, judge: FakeJudge) -> Bench {
        Bench {
            clock: Ticking(AtomicU64::new(0)),
            mint: Counted(AtomicU64::new(1)),
            workflow,
            vcs: FakeVcs::new(),
            work,
            budget: CheckBudget::of(Duration::from_secs(5)),
            judging: Judging {
                client: Arc::new(judge),
                budget: JudgeBudget::of(Duration::from_secs(20)),
                default_model: Model::named("the-cheap-model").expect("a model name"),
                environment: Environment::nothing(),
                // The bench asserts on rulings, not on what a call said about
                // itself while it was out — and it reaches no `api` and no
                // `ipc`, which is the hermetic rule this file's header states.
                marking: Marking::detached(),
            },
            moves: RefCell::new(Vec::new()),
            step_moves: RefCell::new(Vec::new()),
            recorded: RefCell::new(Vec::new()),
        }
    }

    /// A Job at the approval gate, and the worktree its branch names.
    ///
    /// The id is minted through [`Mint`]; the workflow and Manifest ids are
    /// carried, because those name things that already exist rather than
    /// records being created. The steps are the workflow's, written at
    /// creation, which is what freezing means here.
    pub fn created(&self, title: &str) -> Run {
        let id = JobId::carried(self.mint.ulid());
        let spec = WorktreeSpec::for_job(REPO_ROOT, id.as_str()).expect("a legal spec");
        let worktree = self.vcs.create_worktree(&spec).expect("a worktree");
        let new = NewJob {
            id,
            title: Title::new(title).expect("a title somebody could pick out of a list"),
            workflow: self.workflow.frozen().clone(),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01FIXTUREMANIFEST")),
            urgency: Urgency::Normal,
            atomic: false,
            model: ModelName::new("a-model").expect("a model name"),
            acceptance_criteria: criteria(),
            steps: self
                .workflow
                .frozen()
                .steps()
                .iter()
                .enumerate()
                .map(|(ordinal, step)| StepSeed {
                    step_id: step.id().clone(),
                    ordinal: ordinal as u32,
                })
                .collect(),
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            facts: Facts::new("the store's cursor reads one row past the end"),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        };
        let job = Job::create_top_level(new, TopLevelOrigin::Manual, self.clock.now());
        Run {
            job,
            worktree,
            declared: None,
        }
    }

    /// Move the Job, and record the event the machine returned.
    pub fn moved(&self, run: &mut Run, to: Target, by: Actor) {
        let moved = run
            .job
            .transition(to, by, self.clock.now())
            .expect("a legal move");
        self.moves.borrow_mut().push(moved.event);
        run.job = moved.job;
    }

    pub fn refuses(&self, run: &Run, to: Target, by: Actor) -> IllegalTransition {
        run.job
            .transition(to, by, self.clock.now())
            .expect_err("the machine admitted a move it should have refused")
    }

    /// Move one step of the frozen workflow. **Always Fleet**: the inner
    /// machine has no human actor at M1.
    ///
    pub fn step_moved(&self, run: &mut Run, step: &StepId, to: StepTarget) {
        let moved = run
            .job
            .transition_step(step, to, Actor::Fleet, self.clock.now())
            .expect("a legal step move");
        self.step_moves.borrow_mut().push(moved.event);
        run.job = moved.job;
    }

    pub fn refuses_step(&self, run: &Run, step: &StepId, to: StepTarget) -> IllegalStepTransition {
        run.job
            .transition_step(step, to, Actor::Fleet, self.clock.now())
            .expect_err("the machine admitted a step move it should have refused")
    }

    /// Approval, then dispatch: **two edges and two different actors.**
    ///
    /// `awaiting_approval -> queued` is the primary autonomy control and is
    /// recorded as a person's; `queued -> running` is Fleet's. Entering the
    /// first step is what sets the Job's cursor.
    pub fn approved_and_dispatched(&self, run: &mut Run) {
        self.moved(run, Target::Queued, Actor::Human);
        self.moved(run, Target::Running, Actor::Fleet);
        let first = self.workflow.steps()[0].id().clone();
        self.step_moved(run, &first, StepTarget::Running);
    }

    /// Run the step's checks over one submission and decide. **Fleet's own
    /// gate**, called directly, because a hermetic test cannot reach the loop
    /// that ordinarily calls it.
    pub async fn gate(&self, run: &Run, step: &StepId, submitted: &Submission) -> Ruling {
        let at = AtStep::named(self.workflow.frozen(), step, &run.worktree)
            .expect("a step of the workflow");
        let recorded = self.recorded.borrow().clone();
        // The worktree as the step found it. A Run's worktree starts empty and
        // the bench drives one step at a time, so an empty footprint is what
        // `fleet::dispatch` would have read at the step's start — see
        // `Working::entered_with`, which is the thing being stood in for.
        let entered_with = Footprint::nothing();
        let ruling = rule_on(
            at,
            // The Job's own request, which is what the Judge is measured
            // against — not a fixture, because the bench holds the real record.
            Request::of(&run.job),
            submitted,
            run.declared.as_ref(),
            Some(&entered_with),
            &recorded,
            &self.work,
            self.budget,
            &self.judging,
            // Where a kept deliverable would go. **Nothing is written**: no
            // step here declares an `artifact_exists`, so the gate never holds
            // a deliverable's bytes and `REPO_ROOT` is never touched.
            &Keeping::of(REPO_ROOT, run.job.id()),
        )
        .await;
        // What `fleet::dispatch` does with the store, done with a list: the
        // evidence the gate ruled on is written down before anything moves.
        let mut held = self.recorded.borrow_mut();
        held.retain(|(id, _)| id != step);
        held.push((step.clone(), submitted.recorded()));
        ruling
    }

    /// The moves a ruling implies, in the one order the two machines admit.
    ///
    /// **This is the one thing the test restates rather than calls.**
    /// `fleet::dispatch` does it inside a method that also speaks to a live
    /// session and reaps a child, so there is no way to reach it without a
    /// process. The Job move itself is `fleet::apply` and is not restated.
    /// **The step moves first, and the order is not a preference.** The inner
    /// machine is frozen beneath every status but `running` and
    /// `awaiting_review`, so a step stopped after the Job ended would be
    /// refused and its verdict never written — which is #179, observed on a
    /// real Job whose `tests` step read `running` beneath `completed_failed`.
    /// Why it stops is [`fleet::dispatch::stopping`]'s and is not restated
    /// here: a second spelling of the trigger is how the bench and Fleet would
    /// come to disagree about the same failure.
    pub fn settled(&self, run: &mut Run, step: &StepId, ruling: &Ruling) {
        if ruling.advanced() {
            self.step_moved(run, step, StepTarget::Advanced);
        }
        if let Ruling::Advanced { .. } = ruling {
            let next = self.step_after(step).expect("a step follows an advance");
            self.step_moved(run, &next, StepTarget::Running);
        }
        if let Some(why) = stopping(ruling) {
            self.step_moved(run, step, StepTarget::Stopped(why));
        }
        if let Some(moved) = apply(&run.job, ruling, self.clock.now()) {
            let moved = moved.expect("the move a ruling implies is a legal one");
            self.moves.borrow_mut().push(moved.event);
            run.job = moved.job;
        }
    }

    pub fn step_after(&self, step: &StepId) -> Option<StepId> {
        let at = self.workflow.steps().iter().position(|s| s.id() == step)?;
        self.workflow.steps().get(at + 1).map(|s| s.id().clone())
    }

    pub fn step(&self, at: usize) -> StepId {
        self.workflow.steps()[at].id().clone()
    }

    /// Every status the Job passed through, read out of the events rather than
    /// out of the Job.
    pub fn statuses(&self) -> Vec<JobStatus> {
        self.moves.borrow().iter().map(|e| e.to()).collect()
    }

    pub fn actors(&self) -> Vec<Actor> {
        self.moves.borrow().iter().map(|e| e.actor()).collect()
    }

    /// The qualifying reason each move carried, read out of the events.
    /// Unqualified on most; an escalation is where the trigger is.
    pub fn reasons(&self) -> Vec<TransitionReason> {
        self.moves
            .borrow()
            .iter()
            .map(|e| e.reason().clone())
            .collect()
    }
}

pub fn states(job: &Job) -> Vec<(&str, StepState)> {
    job.steps()
        .iter()
        .map(|row: &JobStep| (row.step_id().as_str(), row.state()))
        .collect()
}
