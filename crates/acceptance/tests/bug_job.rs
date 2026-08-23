//! One hermetic test that drives a Bug Job from `not_started` to
//! `completed_success` through every step of the Bug workflow — and the
//! invariants that make that run mean something.
//!
//! **This file does not compile.** It is written before the code it tests and
//! names the API that must exist. See `src/lib.rs` for why that is the intended
//! state for the whole of M0.
//!
//! Hermetic means: no process spawned, no repository touched, no network
//! opened. Every adapter is a fake from `testkit`.

use adapter_traits::{AgentHarness, ModelClient, Secrets, Vcs};
use core_model::{
    AcceptanceCriterion, Actor, Counter, EscalationTrigger, JobState, StepId, Verdict,
};
use fleet::Fleet;
use testkit::{bug_workflow, FakeHarness, FakeModelClient, FakeSecrets, FakeVcs, Script};

/// Every fake, wired. Nothing in this function may touch the real world.
fn fleet() -> Fleet<FakeHarness, FakeVcs, FakeSecrets, FakeModelClient> {
    Fleet::with_adapters(
        FakeHarness::new(),
        FakeVcs::new(),
        FakeSecrets::new(),
        FakeModelClient::new(),
    )
}

fn criteria() -> Vec<AcceptanceCriterion> {
    vec![
        AcceptanceCriterion::new("the reported symptom no longer occurs"),
        AcceptanceCriterion::new("a test covers the reported symptom"),
    ]
}

// ---------------------------------------------------------------------------
// The run itself
// ---------------------------------------------------------------------------

/// A Bug Job, start to finish, through all seven steps of the sample.
///
/// The assertions between the ticks are the point. A test that only checked the
/// final state would pass on a machine that jumped straight to it.
#[test]
fn a_bug_job_runs_from_not_started_to_completed_success() {
    let mut fleet = fleet();
    let mut job = fleet.create(bug_workflow(), criteria());

    // Dispatch is a human decision, always, one Job at a time.
    assert_eq!(job.state(), JobState::NotStarted);
    assert_eq!(job.step(), StepId::new("repro"));
    fleet.tick(&mut job);
    assert_eq!(
        job.state(),
        JobState::NotStarted,
        "a Job with no approval does not start, however many times it is ticked"
    );

    fleet.approve_dispatch(&mut job, Actor::Human);
    assert_eq!(job.state(), JobState::Running);

    // repro — a failing test, and the check expects it to fail.
    fleet.script(Script::step("repro").test_exits(1).evidence_submitted());
    fleet.tick(&mut job);
    assert_eq!(job.step(), StepId::new("root_cause"));

    // root_cause — a note, judged as a deliverable.
    fleet.script(Script::step("root_cause").artifact("root_cause_note").evidence_submitted());
    fleet.tick(&mut job);
    assert_eq!(job.step(), StepId::new("fix"));

    // fix — a non-empty diff, and a Judge panel of three.
    fleet.script(Script::step("fix").diff_nonempty().evidence_submitted());
    fleet.tick(&mut job);
    assert_eq!(job.step(), StepId::new("regression_verify"));

    // regression_verify — the suite goes green.
    fleet.script(Script::step("regression_verify").test_exits(0).evidence_submitted());
    fleet.tick(&mut job);
    assert_eq!(job.step(), StepId::new("review"));

    // review — advisory only. The Judge summarises and does not gate.
    fleet.script(Script::step("review").evidence_submitted());
    fleet.tick(&mut job);
    assert_eq!(
        job.step(),
        StepId::new("review"),
        "review is gated by a Manifest rule, not by the advisory Judge pass"
    );
    fleet.resolve_gate(&mut job, Verdict::Approve, Actor::Human);
    assert_eq!(job.step(), StepId::new("merge"));

    fleet.tick(&mut job);
    assert_eq!(job.step(), StepId::new("close"));

    fleet.tick(&mut job);
    assert_eq!(job.state(), JobState::CompletedSuccess);
}

// ---------------------------------------------------------------------------
// Fleet is the only writer
// ---------------------------------------------------------------------------

/// A Drone saying it is finished moves nothing.
///
/// This is v1's production failure in one assertion: a Drone claimed completion
/// and was believed. The self-report is admitted as a signal and nothing else.
#[test]
fn a_drones_self_report_moves_nothing() {
    let mut fleet = fleet();
    let mut job = fleet.create(bug_workflow(), criteria());
    fleet.approve_dispatch(&mut job, Actor::Human);

    let before = job.step();
    fleet.script(Script::step("repro").drone_claims_success().no_evidence());
    fleet.tick(&mut job);

    assert_eq!(job.step(), before, "a claim is not evidence");
    assert_eq!(job.state(), JobState::Running);
}

/// Every state change arrives through `Job::transition`.
///
/// A test that wrote a status field directly would assert nothing about the
/// machine it is meant to prove, so the machine records its own transitions and
/// this reads them back.
#[test]
fn every_state_change_went_through_the_transition_machine() {
    let mut fleet = fleet();
    let mut job = fleet.create(bug_workflow(), criteria());
    fleet.approve_dispatch(&mut job, Actor::Human);
    fleet.script(Script::step("repro").test_exits(1).evidence_submitted());
    fleet.tick(&mut job);

    let states: Vec<JobState> = job.transitions().map(|t| t.to).collect();
    assert_eq!(states, vec![JobState::Running]);
    assert!(
        job.transitions().all(|t| t.recorded_by_transition_machine()),
        "a state reached any other way is a state nothing verified"
    );
}

// ---------------------------------------------------------------------------
// The Judge is veto-only, in both directions
// ---------------------------------------------------------------------------

/// A refusal on a step whose mechanical checks passed leaves it unadvanced.
#[test]
fn a_judge_refusal_holds_a_step_whose_checks_passed() {
    let mut fleet = fleet();
    let mut job = fleet.create(bug_workflow(), criteria());
    fleet.approve_dispatch(&mut job, Actor::Human);

    fleet.script(
        Script::step("repro")
            .test_exits(1)
            .evidence_submitted()
            .judge_refuses("the failing test does not represent the reported bug"),
    );
    fleet.tick(&mut job);

    assert_eq!(job.step(), StepId::new("repro"));
}

/// A Judge pass on a step whose mechanical checks failed does not advance it.
///
/// The Judge is a veto, not a grant. It cannot vouch for something an exit code
/// already contradicted — that is the direction people forget, because a
/// passing Judge feels like permission.
#[test]
fn a_judge_pass_cannot_rescue_a_failed_check() {
    let mut fleet = fleet();
    let mut job = fleet.create(bug_workflow(), criteria());
    fleet.approve_dispatch(&mut job, Actor::Human);

    fleet.script(
        Script::step("repro")
            .test_exits(0) // the repro step demands a *failing* test
            .evidence_submitted()
            .judge_passes(),
    );
    fleet.tick(&mut job);

    assert_eq!(job.step(), StepId::new("repro"));
}

// ---------------------------------------------------------------------------
// repro is a hard prerequisite
// ---------------------------------------------------------------------------

/// `repro` cannot be advanced past without its artifact, **even on retry**.
///
/// A bug with no reproduction is a bug nobody can prove was fixed, and the whole
/// workflow is built on that artifact existing. The retry path is where a
/// prerequisite quietly stops being one.
#[test]
fn repro_holds_even_on_retry() {
    let mut fleet = fleet();
    let mut job = fleet.create(bug_workflow(), criteria());
    fleet.approve_dispatch(&mut job, Actor::Human);

    for _ in 0..3 {
        fleet.script(Script::step("repro").no_evidence());
        fleet.tick(&mut job);
        assert_eq!(job.step(), StepId::new("repro"));
    }

    assert_eq!(job.state(), JobState::Escalated(EscalationTrigger::Stalled));
    assert!(
        !job.step_completed(StepId::new("repro")),
        "a hard prerequisite that escalation waves through is not one"
    );
}

// ---------------------------------------------------------------------------
// A retry and a loop return are different things
// ---------------------------------------------------------------------------

/// They move different counters, never the same one.
///
/// Both are backward jumps and they look identical in a state diagram. A retry
/// is a failure being re-attempted; a loop return is feedback being addressed,
/// and nothing failed. Sharing a counter makes one of the two caps unreachable.
#[test]
fn a_retry_and_a_loop_return_move_different_counters() {
    let mut fleet = fleet();
    let mut job = fleet.create(bug_workflow(), criteria());
    fleet.approve_dispatch(&mut job, Actor::Human);
    drive_to_step(&mut fleet, &mut job, StepId::new("regression_verify"));

    let retries = job.counter(Counter::Retry);
    let iterations = job.counter(Counter::Iteration);

    // regression_verify fails: `on_fail` sends it back to `fix`. That is a retry.
    fleet.script(Script::step("regression_verify").test_exits(1).evidence_submitted());
    fleet.tick(&mut job);
    assert_eq!(job.step(), StepId::new("fix"));
    assert_eq!(job.counter(Counter::Retry), retries + 1);
    assert_eq!(job.counter(Counter::Iteration), iterations);

    // review returns `request_changes`: verdict_routing sends it back to `fix`.
    // Nothing failed, so the retry counter must not move.
    drive_to_step(&mut fleet, &mut job, StepId::new("review"));
    let retries = job.counter(Counter::Retry);
    let iterations = job.counter(Counter::Iteration);
    fleet.resolve_gate(&mut job, Verdict::RequestChanges, Actor::Human);

    assert_eq!(job.step(), StepId::new("fix"));
    assert_eq!(job.counter(Counter::Retry), retries);
    assert_eq!(job.counter(Counter::Iteration), iterations + 1);
}

// ---------------------------------------------------------------------------
// The yardstick cannot move under the work
// ---------------------------------------------------------------------------

/// `acceptance_criteria[]` is frozen at creation.
///
/// Editing, reordering or removing one fails. A Job that can rewrite what it is
/// being judged against has no gate at all, and this is the cheapest possible
/// version of evidence gaming.
#[test]
fn acceptance_criteria_are_frozen_at_creation() {
    let mut fleet = fleet();
    let mut job = fleet.create(bug_workflow(), criteria());
    let frozen = job.acceptance_criteria().to_vec();

    assert!(job.try_edit_criterion(0, "something weaker").is_err());
    assert!(job.try_remove_criterion(1).is_err());
    assert!(job.try_reorder_criteria(&[1, 0]).is_err());

    assert_eq!(job.acceptance_criteria(), frozen.as_slice());
}

/// The workflow is frozen the same way and for the same reason: a Manifest may
/// shadow a built-in workflow, so `workflow_id` alone does not identify what ran.
#[test]
fn the_workflow_is_frozen_into_the_job() {
    let mut fleet = fleet();
    let job = fleet.create(bug_workflow(), criteria());
    let frozen = job.workflow().clone();

    fleet.replace_workflow_definition(bug_workflow_with_a_weaker_gate());

    assert_eq!(job.workflow(), &frozen);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Drive a Job forward until it rests on `target`, scripting each step to pass.
/// Panics rather than looping forever if the Job stops moving.
fn drive_to_step(
    fleet: &mut Fleet<FakeHarness, FakeVcs, FakeSecrets, FakeModelClient>,
    job: &mut core_model::Job,
    target: StepId,
) {
    for _ in 0..16 {
        if job.step() == target {
            return;
        }
        fleet.script(Script::step(job.step().as_str()).passes());
        let before = job.step();
        fleet.tick(job);
        if job.step() == before {
            panic!("Job stopped at {before:?} before reaching {target:?}");
        }
    }
    panic!("Job never reached {target:?}");
}

fn bug_workflow_with_a_weaker_gate() -> core_model::WorkflowDef {
    bug_workflow().without_judge_on(StepId::new("fix"))
}
