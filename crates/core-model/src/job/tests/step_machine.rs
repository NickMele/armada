//! The inner machine: what it admits, what it refuses, and what it cannot be
//! asked.
//!
//! **Every test reaches its state by transitioning**, the outer status through
//! [`reach`] and the inner state through [`Job::transition_step`]. Nothing here
//! constructs a step already moved, because [`JobStep`] offers no way to and a
//! test that found one would be asserting against a value it made up.
//!
//! # What is asserted about the edge table, and what is not
//!
//! There is no registry file to compare [`STEP_EDGES`] against — that is the
//! gap this milestone step declined to fill. So these tests do not check the
//! table against a source of truth; they check what the machine *does* with it,
//! and they check the property the table exists to hold: a state no
//! [`StepTarget`] names cannot be arrived at.

use super::*;

/// The first step of the fixture WorkflowDef, and the second.
fn first() -> StepId {
    StepId::new("repro")
}

fn second() -> StepId {
    StepId::new("fix")
}

fn when() -> Timestamp {
    at("2026-08-26T10:00:00.000Z")
}

/// A Job standing at `running` with its steps untouched. The only way in.
fn running() -> Job {
    reach(JobStatus::Running)
}

/// Move a step, failing loudly. Fleet is the actor: it is the only thing that
/// drives a transition.
fn step(job: &Job, step_id: &StepId, to: StepTarget) -> Job {
    job.transition_step(step_id, to, Actor::Fleet, when())
        .unwrap_or_else(|e| panic!("moving {} to {:?}: {e}", step_id.as_str(), to.state()))
        .job
}

// ------------------------------------------------------------ what is legal

#[test]
fn every_step_row_starts_not_started_with_no_verdict_and_no_cursor() {
    let job = created();
    for row in job.steps() {
        assert_eq!(row.state(), StepState::NotStarted);
        assert_eq!(row.last_verdict(), None);
    }
    assert_eq!(
        job.current_step_id(),
        None,
        "creation names no current step"
    );
}

#[test]
fn every_edge_in_the_table_is_admitted() {
    for edge in STEP_EDGES {
        // Walk to `from` first: `not_started` is where a row starts, and
        // `running` is one move past it. There is no other way to stand in
        // either.
        let job = match edge.from {
            StepState::NotStarted => running(),
            StepState::Running => step(&running(), &first(), StepTarget::Running),
            // Two moves past the start. A stopped step is a `from` because a
            // redirect and a restart both resume one, which is the whole
            // reason `stopped -> running` is in the table.
            StepState::Stopped => step(
                &step(&running(), &first(), StepTarget::Running),
                &first(),
                StepTarget::Stopped(gate_failure().expect("gate_failure is step-level")),
            ),
            // Also two moves past the start, and the only way to stand here:
            // a hand-back is `running -> retrying`, which is what makes
            // `retrying -> running` the other half of the pair.
            StepState::Retrying => step(
                &step(&running(), &first(), StepTarget::Running),
                &first(),
                StepTarget::Retrying(gate_failure().expect("gate_failure is step-level")),
            ),
            other => panic!("no way to reach {} by transitioning", other.as_wire()),
        };
        // The reason follows the *edge* and not the destination alone, which
        // is the rule `arriving_at` holds once `advanced` has two ways in: a
        // stop carries one and must, a hand-back carries the failure it
        // answers, an override carries the trigger it overrules, and an
        // ordinary advance carries none.
        let qualified = edge.to == StepState::Stopped
            || edge.to == StepState::Retrying
            || (edge.to == StepState::Advanced && edge.from == StepState::Stopped);
        let target = StepTarget::arriving_at(edge.to, qualified.then(gate_failure).flatten())
            .expect("a table edge names a target");
        let moved = step(&job, &first(), target);
        assert_eq!(
            moved.step(&first()).expect("the row is there").state(),
            edge.to
        );
    }
}

#[test]
fn the_cursor_follows_the_step_that_starts_running() {
    let job = step(&running(), &first(), StepTarget::Running);
    assert_eq!(job.current_step_id(), Some(&first()));
    assert_eq!(
        job.current_step().map(JobStep::state),
        Some(StepState::Running)
    );

    let job = step(&job, &first(), StepTarget::Advanced);
    let job = step(&job, &second(), StepTarget::Running);
    assert_eq!(job.current_step_id(), Some(&second()));
}

#[test]
fn advancing_the_last_step_leaves_the_cursor_where_it_was() {
    let job = step(&running(), &first(), StepTarget::Running);
    let job = step(&job, &first(), StepTarget::Advanced);
    assert_eq!(
        job.current_step_id(),
        Some(&first()),
        "the registry says frozen, never cleared, still rendered"
    );
}

#[test]
fn advancing_a_step_records_that_it_passed() {
    let job = step(&running(), &first(), StepTarget::Running);
    let started = job.step(&first()).expect("the row is there").clone();
    assert_eq!(started.last_verdict(), None, "starting rules on nothing");

    let job = step(&job, &first(), StepTarget::Advanced);
    let advanced = job.step(&first()).expect("the row is there");
    assert_eq!(
        advanced.last_verdict(),
        Some(StepVerdict::Passed),
        "`advanced` means the step passed its advance gate"
    );
    assert_eq!(
        advanced.entered_at(),
        started.entered_at(),
        "advancing does not re-enter the step"
    );
    assert_eq!(advanced.updated_at(), &when());
}

#[test]
fn stopping_a_step_records_that_it_failed_and_why() {
    let job = step(&running(), &first(), StepTarget::Running);
    let started = job.step(&first()).expect("the row is there").clone();

    let why = gate_failure().expect("gate_failure is step-level");
    let job = step(&job, &first(), StepTarget::Stopped(why));
    let stopped = job.step(&first()).expect("the row is there");
    assert_eq!(stopped.state(), StepState::Stopped);
    assert_eq!(
        stopped.last_verdict(),
        Some(StepVerdict::Failed(why)),
        "`stopped` says the retries are spent; the verdict says what spent them"
    );
    assert_eq!(
        stopped.entered_at(),
        started.entered_at(),
        "stopping does not re-enter the step"
    );
    assert_eq!(
        job.current_step_id(),
        Some(&first()),
        "an escalated Job points at the step it stopped on"
    );
}

/// The trigger travels on the event as well as on the row, because the log is
/// the authority and the column is a cache of the fold over it.
#[test]
fn the_move_that_stops_a_step_records_why_and_no_other_move_does() {
    let job = step(&running(), &first(), StepTarget::Running);
    let why = gate_failure().expect("gate_failure is step-level");
    let stopped = job
        .transition_step(&first(), StepTarget::Stopped(why), Actor::Fleet, when())
        .expect("running -> stopped is an edge");
    assert_eq!(stopped.event.why(), Some(why));
    assert_eq!(stopped.event.to(), StepState::Stopped);

    let advanced = job
        .transition_step(&first(), StepTarget::Advanced, Actor::Fleet, when())
        .expect("running -> advanced is an edge");
    assert_eq!(advanced.event.why(), None, "an advance qualifies nothing");
}

/// A step that never ran cannot stop: `stopped` means the retries are spent,
/// and a step nothing attempted has spent none.
#[test]
fn a_step_that_never_started_cannot_be_stopped() {
    let why = gate_failure().expect("gate_failure is step-level");
    match running().transition_step(&first(), StepTarget::Stopped(why), Actor::Fleet, when()) {
        Err(IllegalStepTransition::NoSuchEdge { from, to, .. }) => {
            assert_eq!((from, to), (StepState::NotStarted, StepState::Stopped));
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
}

#[test]
fn moving_one_step_leaves_every_other_row_alone() {
    let job = step(&running(), &first(), StepTarget::Running);
    let untouched = job.step(&second()).expect("the row is there");
    assert_eq!(untouched.state(), StepState::NotStarted);
    assert_eq!(untouched.last_verdict(), None);
}

#[test]
fn a_step_moves_beneath_awaiting_review_too() {
    let job = reach(JobStatus::AwaitingReview);
    let moved = step(&job, &first(), StepTarget::Running);
    assert_eq!(
        moved.status(),
        JobStatus::AwaitingReview,
        "the Job stays put"
    );
    assert_eq!(moved.current_step_id(), Some(&first()));
}

// ---------------------------------------------------------- what is refused

#[test]
fn a_step_is_frozen_beneath_every_status_but_the_two() {
    for status in JobStatus::ALL {
        if ADVANCING_STATUSES.contains(status) {
            continue;
        }
        let job = reach(*status);
        match job.transition_step(&first(), StepTarget::Running, Actor::Fleet, when()) {
            Err(IllegalStepTransition::StepsAreFrozen {
                step_id,
                status: at,
            }) => {
                assert_eq!(step_id, first());
                assert_eq!(at, *status);
            }
            other => panic!("{} admitted a step move: {other:?}", status.as_wire()),
        }
    }
}

#[test]
fn every_pair_the_table_does_not_name_is_refused() {
    let reachable = [
        (StepState::NotStarted, running()),
        (
            StepState::Running,
            step(&running(), &first(), StepTarget::Running),
        ),
        (StepState::Advanced, {
            let job = step(&running(), &first(), StepTarget::Running);
            step(&job, &first(), StepTarget::Advanced)
        }),
    ];
    for (from, job) in reachable {
        for to in [StepTarget::Running, StepTarget::Advanced] {
            if STEP_EDGES
                .iter()
                .any(|edge| edge.from == from && edge.to == to.state())
            {
                continue;
            }
            match job.transition_step(&first(), to, Actor::Fleet, when()) {
                Err(IllegalStepTransition::NoSuchEdge {
                    from: refused_from,
                    to: refused_to,
                    ..
                }) => {
                    assert_eq!(refused_from, from);
                    assert_eq!(refused_to, to.state());
                }
                other => panic!(
                    "{} -> {} is not an edge and was admitted: {other:?}",
                    from.as_wire(),
                    to.state().as_wire()
                ),
            }
        }
    }
}

#[test]
fn naming_a_step_the_job_does_not_have_is_refused_rather_than_ignored() {
    let job = running();
    match job.transition_step(
        &StepId::new("a-step-of-some-other-job"),
        StepTarget::Running,
        Actor::Fleet,
        when(),
    ) {
        Err(IllegalStepTransition::NoSuchStep { step_id }) => {
            assert_eq!(step_id.as_str(), "a-step-of-some-other-job");
        }
        other => panic!("expected a refusal naming the step, found {other:?}"),
    }
}

// ------------------------------------------------- what cannot be asked for

#[test]
fn the_two_states_m1_cannot_reach_have_no_target_to_arrive_by() {
    // One state, now that `retrying` has a budget to be inside. `awaiting_human`
    // has its gate and is still unreachable, because a step at that gate stays
    // `running` — `step_machine`'s own comment says what changing that costs.
    assert!(
        StepTarget::arriving_at(StepState::AwaitingHuman, None).is_none(),
        "awaiting_human needs a variant and two edges, and M1 has neither"
    );
    assert!(
        StepTarget::arriving_at(StepState::AwaitingHuman, gate_failure()).is_none(),
        "and a trigger does not buy one"
    );
    assert!(
        StepTarget::arriving_at(StepState::NotStarted, None).is_none(),
        "not_started is written at creation and is not a destination"
    );
}

/// The pair is read as a whole. A stop with no trigger names no target, and a
/// trigger on a destination that stores none names no target either — which is
/// what stops a stored row folding into a step that stopped for no reason.
#[test]
fn a_stored_state_and_its_reason_are_read_as_one_value() {
    assert!(StepTarget::arriving_at(StepState::Stopped, None).is_none());
    assert!(StepTarget::arriving_at(StepState::Running, gate_failure()).is_none());
    assert_eq!(
        StepTarget::arriving_at(StepState::Stopped, gate_failure()),
        gate_failure().map(StepTarget::Stopped)
    );
    // `advanced` is the one state two targets reach, and the trigger is what
    // tells them apart. Without the reason a stored override would fold back as
    // an ordinary pass, and the verdict a person overruled would be gone.
    assert_eq!(
        StepTarget::arriving_at(StepState::Advanced, None),
        Some(StepTarget::Advanced)
    );
    assert_eq!(
        StepTarget::arriving_at(StepState::Advanced, gate_failure()),
        gate_failure().map(StepTarget::Overridden)
    );
}

/// A stopped step advances **only** as an override.
///
/// The edge is there and only one of the two targets that reach `advanced` may
/// walk it. `StepTarget::Advanced` records `passed`, which over a stopped step
/// would erase the verdict the person is disagreeing with — and an override
/// that reads like an ordinary advance is how a Judge becomes decorative
/// without anybody deciding it should be.
#[test]
fn a_stopped_step_advances_only_by_overruling_what_stopped_it() {
    let why = gate_failure().expect("gate_failure is step-level");
    let stopped = step(
        &step(&running(), &first(), StepTarget::Running),
        &first(),
        StepTarget::Stopped(why),
    );

    match stopped.transition_step(&first(), StepTarget::Advanced, Actor::Human, when()) {
        Err(IllegalStepTransition::StepDidNotPass { step_id }) => assert_eq!(step_id, first()),
        other => panic!("a stopped step was advanced as a pass: {other:?}"),
    }

    let overridden = step(&stopped, &first(), StepTarget::Overridden(why));
    let row = overridden.step(&first()).expect("the row is there");
    assert_eq!(row.state(), StepState::Advanced);
    assert_eq!(
        row.last_verdict(),
        Some(StepVerdict::Failed(why)),
        "the gate's ruling stands beside the state a person moved it to"
    );
}

/// `last_verdict` admits step-level triggers only, and the payload cannot be
/// built from a Job-level one at all.
#[test]
fn a_job_level_trigger_cannot_be_made_into_a_step_stop() {
    for trigger in EscalationTrigger::ALL {
        assert_eq!(
            StepLevelTrigger::of(*trigger).is_some(),
            trigger.level() == TriggerLevel::Step,
            "{}",
            trigger.as_wire()
        );
    }
}

fn gate_failure() -> Option<StepLevelTrigger> {
    StepLevelTrigger::of(EscalationTrigger::GateFailure)
}

fn drone_killed() -> StepLevelTrigger {
    StepLevelTrigger::of(EscalationTrigger::DroneKilled).expect("drone_killed is step-level")
}

/// A Job escalated over a step that is still being worked — the `stalled`
/// shape, and the one the freeze had no answer for.
///
/// The escalation is Job-level, so nothing stops the step on the way: this is
/// the record a liveness escalation genuinely leaves, with a Drone alive and
/// idle on a `running` row.
fn escalated_over_a_running_step() -> Job {
    step(&running(), &first(), StepTarget::Running)
        .transition(
            Target::Escalated(EscalationTrigger::Stalled),
            Actor::Fleet,
            when(),
        )
        .expect("running -> escalated")
        .job
}

/// The second exception to the freeze: a person ending the Drone stops the step
/// it was on, beneath `escalated`.
///
/// **The move is a person's and the actor says so.** Without it the row stayed
/// `running` on a Job holding no Drone, which is neither true nor restartable —
/// `#313`.
#[test]
fn a_person_ending_a_drone_stops_the_step_beneath_an_escalated_job() {
    let job = escalated_over_a_running_step();
    let moved = job
        .transition_step(
            &first(),
            StepTarget::Stopped(drone_killed()),
            Actor::Human,
            when(),
        )
        .expect("a person may take the Drone off a step of a Job they are holding");

    let row = moved.job.step(&first()).expect("the row is there");
    assert_eq!(row.state(), StepState::Stopped);
    assert_eq!(
        row.last_verdict(),
        Some(StepVerdict::Failed(drone_killed()))
    );
    assert_eq!(
        moved.job.status(),
        JobStatus::Escalated,
        "the Job stays put"
    );
    assert_eq!(
        moved.job.stopped_on().map(|(step, _)| step.clone()),
        Some(first()),
        "and the step a restart lands on is now findable"
    );
    assert_eq!(moved.event.actor(), Actor::Human);
    assert_eq!(moved.event.under(), JobStatus::Escalated);
}

/// The exception admits that one move and nothing beside it.
///
/// Each case drops one of the three conditions the predicate is written from,
/// and each is refused — which is what keeps it an exception rather than a hole
/// in the freeze.
#[test]
fn the_freeze_still_holds_everywhere_the_kill_does_not_reach() {
    let job = escalated_over_a_running_step();

    // A different trigger. The gate stopping a step beneath a parked Job is
    // exactly what the freeze exists to refuse.
    assert!(matches!(
        job.transition_step(
            &first(),
            StepTarget::Stopped(gate_failure().expect("gate_failure is step-level")),
            Actor::Fleet,
            when(),
        ),
        Err(IllegalStepTransition::StepsAreFrozen { .. })
    ));

    // A different destination. Nothing is being resumed or advanced here.
    assert!(matches!(
        job.transition_step(&first(), StepTarget::Running, Actor::Human, when()),
        Err(IllegalStepTransition::StepsAreFrozen { .. })
    ));

    // A different status. `killed` is terminal and its steps are frozen for
    // good, whoever is asking and whatever they carry.
    assert!(matches!(
        reach(JobStatus::Killed).transition_step(
            &first(),
            StepTarget::Stopped(drone_killed()),
            Actor::Human,
            when(),
        ),
        Err(IllegalStepTransition::StepsAreFrozen { .. })
    ));
}

/// The exception closes behind the move it admitted.
///
/// `from == Running` is one of the three conditions, so once the step is
/// `stopped` the predicate is false again and the freeze answers — which is
/// what makes this an opening for one act rather than a status the inner
/// machine now moves beneath. The refusal is the freeze's and not the edge
/// table's, because the freeze is asked first.
#[test]
fn the_exception_closes_once_the_step_has_stopped() {
    let stopped = escalated_over_a_running_step()
        .transition_step(
            &first(),
            StepTarget::Stopped(drone_killed()),
            Actor::Human,
            when(),
        )
        .expect("the first kill lands")
        .job;

    assert!(matches!(
        stopped.transition_step(
            &first(),
            StepTarget::Stopped(drone_killed()),
            Actor::Human,
            when(),
        ),
        Err(IllegalStepTransition::StepsAreFrozen { .. })
    ));
}

#[test]
fn the_registry_still_declares_all_six_states() {
    assert_eq!(
        StepState::ALL.len(),
        6,
        "unreachable is not undeclared: a stored row may render any of the six"
    );
}

#[test]
fn a_step_move_records_the_status_it_happened_under() {
    let job = running();
    let moved = job
        .transition_step(&first(), StepTarget::Running, Actor::Fleet, when())
        .expect("running admits a step move");
    let event = &moved.event;
    assert_eq!(event.job_id(), job.id());
    assert_eq!(event.step_id(), &first());
    assert_eq!(event.from(), StepState::NotStarted);
    assert_eq!(event.to(), StepState::Running);
    assert_eq!(event.under(), JobStatus::Running);
    assert_eq!(event.actor(), Actor::Fleet);
    assert_eq!(event.at(), &when());

    let fields = event.fields();
    assert_eq!(
        fields.get("step_state_to"),
        Some(&FieldValue::Str("running".into()))
    );
    assert_eq!(
        fields.get("job_status"),
        Some(&FieldValue::Str("running".into()))
    );
}
