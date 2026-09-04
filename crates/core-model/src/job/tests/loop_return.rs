//! The loop return: the one edge that takes a step backwards, and the two
//! refusals that stop it meaning anything else.
//!
//! **These are `step_machine`'s tests, in a file of their own.** That one is
//! already past the 500-line mark where a Rust file usually stops doing one
//! thing, and the return is a distinct subject: `advanced -> running` is the
//! only edge in the table whose whole purpose is to undo a pass, and every
//! assertion here is about telling it apart from the three acts it resembles —
//! a dispatch, a resume and a retry.
//!
//! Like every test beside it, nothing here constructs a step already moved.

use alloc::collections::BTreeMap;

use super::*;

fn first() -> StepId {
    StepId::new("repro")
}

fn second() -> StepId {
    StepId::new("fix")
}

fn when() -> Timestamp {
    at("2026-08-26T10:00:00.000Z")
}

fn later() -> Timestamp {
    at("2026-08-26T11:30:00.000Z")
}

fn gate_failure() -> StepLevelTrigger {
    StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("gate_failure is step-level")
}

fn step(job: &Job, step_id: &StepId, to: StepTarget) -> Job {
    step_at(job, step_id, to, when())
}

fn step_at(job: &Job, step_id: &StepId, to: StepTarget, at: Timestamp) -> Job {
    let state = to.state();
    job.transition_step(step_id, to, Actor::Fleet, at)
        .unwrap_or_else(|e| panic!("moving {} to {state:?}: {e}", step_id.as_str()))
        .job
}

/// The return Design Plan makes: `fix` is the step that emitted the verdict, so
/// it is the step the pass is counted against.
fn returned() -> StepTarget {
    StepTarget::Returned(second())
}

/// The shape Design Plan draws: `repro` worked and passed, `fix` worked, and a
/// verdict on `fix` about to send the Job back to `repro`.
fn a_pass_over_both_steps() -> Job {
    let job = reach(JobStatus::Running);
    let job = step(&job, &first(), StepTarget::Running);
    let job = step(&job, &first(), StepTarget::Advanced);
    step(&job, &second(), StepTarget::Running)
}

// ------------------------------------------------------------- what is legal

#[test]
fn a_loop_returns_to_a_step_that_already_advanced() {
    let job = a_pass_over_both_steps();
    let job = step(&job, &first(), returned());
    assert_eq!(
        job.step(&first()).expect("the row is there").state(),
        StepState::Running,
        "a returned step is being worked again"
    );
}

/// The half of the issue that names a decision: the cursor moves to the step
/// the loop routes *to*, and it is the only move in the machine that takes it
/// to a lower ordinal.
#[test]
fn the_cursor_moves_back_to_the_step_the_loop_returns_to() {
    let job = a_pass_over_both_steps();
    assert_eq!(job.current_step_id(), Some(&second()));

    let job = step(&job, &first(), returned());
    assert_eq!(
        job.current_step_id(),
        Some(&first()),
        "the next Drone goes where the work goes"
    );
    assert_eq!(
        job.step(&second()).expect("the row is there").state(),
        StepState::Running,
        "this call moves one row, and which state the emitting step is left in \
         is nobody's decision yet"
    );
}

/// `entered_at` is what the pair `(entered_at, updated_at)` measures a pass
/// against, and a return starts a pass. A hand-back does not, which is the
/// arm this one has to be told apart from.
#[test]
fn a_return_re_enters_the_step_and_a_hand_back_does_not() {
    let job = a_pass_over_both_steps();
    let opened = job
        .step(&first())
        .expect("the row is there")
        .entered_at()
        .clone();

    let returned = step_at(&job, &first(), returned(), later());
    let row = returned.step(&first()).expect("the row is there");
    assert_eq!(row.entered_at(), &later(), "a new pass, so a new clock");
    assert_ne!(row.entered_at(), &opened);

    let handed_back = step_at(
        &job,
        &second(),
        StepTarget::Retrying(gate_failure()),
        later(),
    );
    let row = handed_back.step(&second()).expect("the row is there");
    assert_eq!(
        row.entered_at(),
        &when(),
        "a retry is the same run continuing, and the clock stays where it was"
    );
}

/// Activity and verdict are separate fields, so a return says the step is being
/// worked without saying anything new about how it was last ruled on.
#[test]
fn a_return_leaves_the_last_ruling_standing() {
    let job = a_pass_over_both_steps();
    let job = step(&job, &first(), returned());
    assert_eq!(
        job.step(&first()).expect("the row is there").last_verdict(),
        Some(StepVerdict::Passed),
        "the gate did clear the draft; what a later step disagreed with is the plan"
    );
}

/// The edge is walkable beneath `awaiting_review`, and that is the whole point:
/// a human gate holds the Job there, and `request_changes` is the verdict that
/// routes backwards.
#[test]
fn a_loop_returns_beneath_the_status_a_human_gate_holds_the_job_at() {
    let job = a_pass_over_both_steps();
    let job = drive(&job, &[Target::AwaitingReview]);
    assert_eq!(job.status(), JobStatus::AwaitingReview);

    let returned = job
        .transition_step(&first(), returned(), Actor::Human, when())
        .expect("awaiting_review is in ADVANCING_STATUSES");
    assert_eq!(
        returned
            .job
            .step(&first())
            .expect("the row is there")
            .state(),
        StepState::Running
    );
}

/// A loop is not one return: the second is walked from the state the first
/// left, which is what makes the count off the log a count rather than a flag.
#[test]
fn a_step_returns_to_more_than_once() {
    let job = a_pass_over_both_steps();
    let job = step(&job, &first(), returned());
    let job = step(&job, &first(), StepTarget::Advanced);
    let job = step(&job, &first(), returned());
    assert_eq!(
        job.step(&first()).expect("the row is there").state(),
        StepState::Running
    );
}

// --------------------------------------------------------- what is refused

/// The narrowing that is the reason this edge is safe to add. Without it,
/// `advanced -> running` would admit the redispatch `fleet::resume` refuses.
#[test]
fn an_advanced_step_cannot_be_dispatched_into_as_if_it_had_never_run() {
    let job = a_pass_over_both_steps();
    assert_eq!(
        job.transition_step(&first(), StepTarget::Running, Actor::Fleet, when()),
        Err(IllegalStepTransition::StepAlreadyAdvanced { step_id: first() }),
        "re-running an advanced step is a redispatch, and it is not a loop return"
    );
}

/// The other half. A return onto a step that has not advanced is a dispatch, a
/// restart or a no-op wearing a loop's name, and each of those has its own
/// target already.
#[test]
fn a_return_onto_a_step_that_has_not_advanced_is_refused_from_every_state() {
    let fresh = reach(JobStatus::Running);
    assert_eq!(
        fresh.transition_step(&first(), returned(), Actor::Fleet, when()),
        Err(IllegalStepTransition::NotAnAdvancedStep {
            step_id: first(),
            from: StepState::NotStarted,
        })
    );

    // The one that used to be refused as a missing edge and is not any more:
    // the self-edge exists now, so a return onto the step being worked reaches
    // the narrowing and is refused as the act it actually is. `Revisited` is
    // what walks that edge, and a return is not it.
    let running = step(&fresh, &first(), StepTarget::Running);
    assert_eq!(
        running.transition_step(&first(), returned(), Actor::Fleet, when()),
        Err(IllegalStepTransition::NotAnAdvancedStep {
            step_id: first(),
            from: StepState::Running,
        })
    );

    let retrying = step(&running, &first(), StepTarget::Retrying(gate_failure()));
    assert_eq!(
        retrying.transition_step(&first(), returned(), Actor::Fleet, when()),
        Err(IllegalStepTransition::NotAnAdvancedStep {
            step_id: first(),
            from: StepState::Retrying,
        })
    );

    let stopped = step(&running, &first(), StepTarget::Stopped(gate_failure()));
    assert_eq!(
        stopped.transition_step(&first(), returned(), Actor::Fleet, when()),
        Err(IllegalStepTransition::NotAnAdvancedStep {
            step_id: first(),
            from: StepState::Stopped,
        }),
        "a stopped step is restarted, which is `stopped -> running` and is a person's act"
    );
}

/// The outer machine still gates the inner one. A loop return is a machine act
/// and neither of the two exceptions is a machine act.
#[test]
fn a_loop_does_not_return_beneath_a_status_the_steps_are_frozen_under() {
    let job = a_pass_over_both_steps();
    let job = drive(&job, &[Target::Escalated(EscalationTrigger::Stalled)]);
    assert_eq!(
        job.transition_step(&first(), returned(), Actor::Human, when()),
        Err(IllegalStepTransition::StepsAreFrozen {
            step_id: first(),
            status: JobStatus::Escalated,
        }),
        "an override and a kill are the two a person may make, and this is neither"
    );
}

// ------------------------------------------------- what a stored row says

/// The fold's question. Both arrivals at `running` carry no trigger, so the
/// state left behind is the only thing that tells them apart — which is why
/// `arriving_at` takes it.
#[test]
fn a_stored_return_is_told_from_a_stored_dispatch_by_the_state_it_left() {
    assert_eq!(
        StepTarget::arriving_at(
            StepState::Advanced,
            StepState::Running,
            None,
            Some(second())
        ),
        Some(returned())
    );
    for from in [
        StepState::NotStarted,
        StepState::Retrying,
        StepState::Stopped,
    ] {
        assert_eq!(
            StepTarget::arriving_at(from, StepState::Running, None, None),
            Some(StepTarget::Running),
            "only `advanced` is a loop return's origin"
        );
    }
    // The loop's other half, told apart the same way: two edges arrive at
    // `running` carrying nothing, and the state left behind is all there is.
    assert_eq!(
        StepTarget::arriving_at(StepState::Running, StepState::Running, None, None),
        Some(StepTarget::Revisited),
        "a step that was already running was revisited, not dispatched into"
    );
}

// ------------------------------------------------------- the loop's other half

/// **The second pass reaches the step that asked for it.** A return leaves the
/// emitting step `running`, so the forward walk that follows arrives at a step
/// that never left — and until `#263` there was no edge for that arrival at
/// all, so the loop stopped one move short of closing.
#[test]
fn the_loop_comes_round_to_the_step_that_sent_the_work_back() {
    let job = a_pass_over_both_steps();
    let job = step(&job, &first(), returned());
    assert_eq!(
        job.step(&second()).expect("the row is there").state(),
        StepState::Running,
        "the emitting step is where the return left it"
    );

    // The redone step passes again, and the Job walks forward into the gate.
    let job = step(&job, &first(), StepTarget::Advanced);
    let job = step_at(&job, &second(), StepTarget::Revisited, later());
    let gate = job.step(&second()).expect("the row is there");
    assert_eq!(gate.state(), StepState::Running);
    assert_eq!(
        gate.entered_at(),
        &later(),
        "a new pass, so a new clock — which is what `store::attempt` counts"
    );
    assert_eq!(
        job.current_step_id(),
        Some(&second()),
        "and the cursor followed the work forward"
    );
}

/// The narrowing that makes the self-edge safe to declare. Without it the edge
/// would legalise entering a step a Drone is already working, which is a
/// redispatch with no move in it.
#[test]
fn a_step_being_worked_cannot_be_dispatched_into_as_if_it_were_idle() {
    let job = a_pass_over_both_steps();
    assert_eq!(
        job.transition_step(&second(), StepTarget::Running, Actor::Fleet, when()),
        Err(IllegalStepTransition::StepIsAlreadyRunning { step_id: second() })
    );
}

/// The other half of that narrowing. A revisit onto a step that is not being
/// worked is a dispatch, a return, a retry or a restart wearing a loop's name.
#[test]
fn a_revisit_onto_a_step_that_is_not_being_worked_is_refused_from_every_state() {
    let fresh = reach(JobStatus::Running);
    assert_eq!(
        fresh.transition_step(&first(), StepTarget::Revisited, Actor::Fleet, when()),
        Err(IllegalStepTransition::NotARunningStep {
            step_id: first(),
            from: StepState::NotStarted,
        })
    );

    let running = step(&fresh, &first(), StepTarget::Running);
    let advanced = step(&running, &first(), StepTarget::Advanced);
    assert_eq!(
        advanced.transition_step(&first(), StepTarget::Revisited, Actor::Fleet, when()),
        Err(IllegalStepTransition::NotARunningStep {
            step_id: first(),
            from: StepState::Advanced,
        }),
        "an advanced step is returned to, and a return names who sent the work back"
    );

    let stopped = step(&running, &first(), StepTarget::Stopped(gate_failure()));
    assert_eq!(
        stopped.transition_step(&first(), StepTarget::Revisited, Actor::Fleet, when()),
        Err(IllegalStepTransition::NotARunningStep {
            step_id: first(),
            from: StepState::Stopped,
        })
    );
}

/// A revisit charges no pass. The loop was counted where it was declared — on
/// the return — and counting it again on the arrival would read one loop as
/// two, which is the whole reason this target carries no step id.
#[test]
fn a_revisit_carries_nothing_and_charges_no_pass() {
    assert_eq!(StepTarget::Revisited.why(), None);
    assert_eq!(StepTarget::Revisited.returned_by(), None);
    assert!(StepTarget::Revisited.begins_a_run());

    let job = a_pass_over_both_steps();
    let job = step(&job, &first(), returned());
    let job = step(&job, &first(), StepTarget::Advanced);
    let moved = job
        .transition_step(&second(), StepTarget::Revisited, Actor::Fleet, later())
        .expect("running -> running is an edge");
    assert_eq!(moved.event.returned_by(), None);
    assert_eq!(moved.event.from(), StepState::Running);
    assert_eq!(moved.event.to(), StepState::Running);
}

/// The three things that turn on "is this a run of the step" ask one question,
/// so they cannot drift apart.
#[test]
fn a_return_begins_a_run_and_the_moves_that_do_not_say_so() {
    assert!(returned().begins_a_run());
    assert!(StepTarget::Running.begins_a_run());
    assert!(!StepTarget::Advanced.begins_a_run());
    assert!(!StepTarget::Stopped(gate_failure()).begins_a_run());
    assert!(!StepTarget::Retrying(gate_failure()).begins_a_run());
    assert!(!StepTarget::Overridden(gate_failure()).begins_a_run());
}

/// A trigger is a reason a gate refused something, and no gate refused this.
#[test]
fn a_return_carries_no_trigger_and_the_event_says_so() {
    let job = a_pass_over_both_steps();
    let moved = job
        .transition_step(&first(), returned(), Actor::Fleet, when())
        .expect("advanced -> running is an edge");
    assert_eq!(moved.event.why(), None);
    assert_eq!(moved.event.from(), StepState::Advanced);
    assert_eq!(moved.event.to(), StepState::Running);
    assert_eq!(
        moved.event.returned_by(),
        Some(&second()),
        "no trigger, and the step that sent it back instead"
    );
}

/// The decision this issue records: `iteration_count` is the emitting step's,
/// and the emitting step makes no move of its own — so the row the routed-to
/// step writes is the only place the attribution can live.
#[test]
fn the_return_names_the_step_that_caused_it_and_not_the_step_it_lands_on() {
    let job = a_pass_over_both_steps();
    let moved = job
        .transition_step(&first(), returned(), Actor::Fleet, when())
        .expect("advanced -> running is an edge");
    assert_eq!(moved.event.step_id(), &first(), "the step that moved");
    assert_eq!(
        moved.event.returned_by(),
        Some(&second()),
        "the step whose iteration_count this pass is charged to"
    );
    assert_eq!(
        moved.job.step(&second()).expect("the row is there").state(),
        StepState::Running,
        "and it is still `running`, which is how a step at a human gate reads"
    );
}

/// A count attributed to a step the Job does not have is a count nobody can
/// read back, so the machine refuses it where the moved step is refused.
#[test]
fn a_return_naming_a_step_the_job_does_not_have_is_refused() {
    let job = a_pass_over_both_steps();
    let stranger = StepId::new("present");
    assert_eq!(
        job.transition_step(
            &first(),
            StepTarget::Returned(stranger.clone()),
            Actor::Fleet,
            when()
        ),
        Err(IllegalStepTransition::NoSuchStep { step_id: stranger })
    );
}

/// The mirror of the reason arm: a stored emitter on a move that stores none is
/// a row this build cannot have written, and a return with none is a pass
/// nobody's count was charged for.
#[test]
fn a_stored_emitter_belongs_to_the_return_and_to_no_other_move() {
    assert_eq!(
        StepTarget::arriving_at(StepState::Advanced, StepState::Running, None, None),
        None,
        "a return with no emitter cannot be attributed"
    );
    assert_eq!(
        StepTarget::arriving_at(
            StepState::Retrying,
            StepState::Running,
            None,
            Some(second())
        ),
        None,
        "and a hand-back's re-entry never carries one"
    );
    assert_eq!(
        StepTarget::arriving_at(
            StepState::Running,
            StepState::Advanced,
            None,
            Some(second())
        ),
        None
    );
}

// --------------------------------------------------------------- the cap

/// `iteration_cap` counts passes, not returns: with a cap of five the fifth
/// pass is the last. That is the reading `retry_limit × iteration_cap` and
/// "iteration 3 of 5" both compute against.
#[test]
fn the_cap_bounds_passes_and_the_last_one_may_not_return() {
    let capped = ResolvedStep::frozen(
        first(),
        "Reproduce".into(),
        None,
        Vec::new(),
        AdvanceGate::HumanAlways,
        Vec::new(),
        None,
        0,
        None,
    )
    .looping(BTreeMap::from([(GateVerdict::RequestChanges, second())]), 5);
    assert_eq!(capped.iteration_cap(), 5);
    assert!(capped.may_return(Iteration::FIRST));
    assert!(capped.may_return(Iteration::returns_made(3)));
    assert!(
        !capped.may_return(Iteration::returns_made(4)),
        "the fifth pass is the fifth of five"
    );
    assert!(!capped.may_return(Iteration::returns_made(9)));
}

/// The fail-closed default, and the reason it is a count rather than an
/// `Option`: a step nothing routes back to is on its first pass forever.
#[test]
fn a_step_that_declared_no_cap_permits_no_return() {
    let plain = ResolvedStep::frozen(
        first(),
        "Reproduce".into(),
        None,
        Vec::new(),
        AdvanceGate::Auto,
        Vec::new(),
        None,
        3,
        None,
    );
    assert_eq!(plain.iteration_cap(), 0);
    assert!(!plain.may_return(Iteration::FIRST));
    assert!(
        plain.may_hand_back(Spent::FIRST),
        "and the retry budget is untouched by it: the two caps bound different things"
    );
}
