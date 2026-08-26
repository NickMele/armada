//! What a Job is made of, and what a transition records.
//!
//! Creation writes the `job_steps` rows and nothing else writes them; a
//! transition writes the status and nothing else; the event carries the reason,
//! the actor and the time it was given.

use super::*;

// ------------------------------------------------------------------ creation

#[test]
fn a_top_level_job_is_created_at_the_approval_gate() {
    let job = created();
    assert_eq!(job.status(), JobStatus::AwaitingApproval);
    assert_eq!(job.origin(), Origin::Manual);
    assert!(job.dispatched_by().is_none());
    assert!(
        job.current_step_id().is_none(),
        "the approval gate has no current step"
    );
    assert!(job.assigned_drone().is_none());
}

#[test]
fn a_sub_dispatched_job_is_created_queued_and_says_who_dispatched_it() {
    let by = DispatchOrigin {
        job_id: JobId::carried(id("01J0000000000000000000PAR0")),
        step_id: StepId::new("plan"),
    };
    let job = Job::create_sub_dispatched(draft(), by.clone(), at("2026-08-26T09:00:00.000Z"));
    assert_eq!(job.status(), JobStatus::Queued);
    assert_eq!(
        job.origin(),
        Origin::SubDispatched,
        "origin is written from dispatched_by"
    );
    assert_eq!(job.dispatched_by(), Some(&by));
}

#[test]
fn every_step_row_is_written_at_creation_not_started() {
    let job = created();
    assert_eq!(job.steps().len(), 2);
    for row in job.steps() {
        assert_eq!(row.state(), StepState::NotStarted);
        assert!(row.last_verdict().is_none());
        assert_eq!(row.entered_at(), row.updated_at());
        assert_eq!(row.job_id(), job.id());
    }
    assert_eq!(job.step(&StepId::new("fix")).map(|r| r.ordinal()), Some(1));
}

// ----------------------------------------------------------------- the event

#[test]
fn the_event_carries_the_reason_the_actor_and_the_time() {
    let job = reach(JobStatus::Running);
    let moved = job
        .transition(
            Target::Piloted(PilotReason::Assist),
            Actor::Human,
            at("2026-08-26T11:22:33.000Z"),
        )
        .expect("running -> piloted");
    let event = &moved.event;
    assert_eq!(event.job_id(), job.id());
    assert_eq!(event.from(), JobStatus::Running);
    assert_eq!(event.to(), JobStatus::Piloted);
    assert_eq!(
        event.reason(),
        &TransitionReason::Pilot(PilotReason::Assist)
    );
    assert_eq!(event.actor(), Actor::Human);
    assert_eq!(event.at().as_str(), "2026-08-26T11:22:33.000Z");
}

#[test]
fn a_transition_into_a_status_that_stores_no_reason_carries_none() {
    let job = reach(JobStatus::Queued);
    let moved = job
        .transition(
            Target::Running,
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect("queued -> running");
    assert_eq!(moved.event.reason(), &TransitionReason::Unqualified);
    assert!(moved.event.reason().as_wire().is_none());
}

#[test]
fn a_queued_reason_is_marked_derived_rather_than_stored() {
    let moved = created()
        .transition(Target::Queued, Actor::Human, at("2026-08-26T10:00:00.000Z"))
        .expect("awaiting_approval -> queued");
    assert_eq!(moved.event.reason(), &TransitionReason::DerivedAtRead);
    assert!(!moved.event.fields().contains_key("transition_reason"));
}

#[test]
fn the_event_renders_as_structured_fields_never_a_sentence() {
    let job = reach(JobStatus::Running);
    let owed = CriteriaOwed::owing(CriterionId::new("c1"), vec![CriterionId::new("c2")]);
    let moved = job
        .transition(
            Target::AwaitingAttestation(owed),
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect("running -> awaiting_attestation");
    let fields = moved.event.fields();
    assert_eq!(
        fields.get("job_status_to"),
        Some(&FieldValue::Str("awaiting_attestation".into()))
    );
    assert_eq!(
        fields.get("criteria_owed"),
        Some(&FieldValue::List(vec![
            FieldValue::Str("c1".into()),
            FieldValue::Str("c2".into()),
        ]))
    );
}

// ------------------------------------------------------------ wire agreement

#[test]
fn every_wire_value_round_trips() {
    for status in JobStatus::ALL {
        assert_eq!(JobStatus::from_wire(status.as_wire()), Some(*status));
    }
    for state in StepState::ALL {
        assert_eq!(StepState::from_wire(state.as_wire()), Some(*state));
    }
    for trigger in EscalationTrigger::ALL {
        assert_eq!(
            EscalationTrigger::from_wire(trigger.as_wire()),
            Some(*trigger)
        );
    }
    for origin in Origin::ALL {
        assert_eq!(Origin::from_wire(origin.as_wire()), Some(*origin));
    }
    for urgency in Urgency::ALL {
        assert_eq!(Urgency::from_wire(urgency.as_wire()), Some(*urgency));
    }
    assert_eq!(JobStatus::from_wire("in_progress"), None);
}

#[test]
fn the_registrys_counts_are_what_the_enums_hold() {
    assert_eq!(JobStatus::ALL.len(), 12);
    assert_eq!(StepState::ALL.len(), 6);
    assert_eq!(EscalationTrigger::ALL.len(), 13);
    assert_eq!(Origin::ALL.len(), 5);
    assert_eq!(PilotReason::ALL.len(), 3);
}

#[test]
fn silent_is_carried_as_a_sub_kind_rather_than_resolved() {
    assert_eq!(EscalationTrigger::Silent.kind(), TriggerKind::SubKind);
    assert_eq!(
        EscalationTrigger::Silent.sub_kind_of(),
        Some(EscalationTrigger::Stalled)
    );
    assert_eq!(EscalationTrigger::Stalled.kind(), TriggerKind::Trigger);
    assert!(EscalationTrigger::Stalled.sub_kind_of().is_none());
}

#[test]
fn five_triggers_carry_no_level_and_that_is_the_registrys_gap() {
    let undecided: Vec<&str> = EscalationTrigger::ALL
        .iter()
        .filter(|t| t.level().is_none())
        .map(|t| t.as_wire())
        .collect();
    assert_eq!(
        undecided,
        vec![
            "evidence_suspect",
            "fan_out",
            "silent",
            "stalled",
            "thrashing"
        ]
    );
    assert_eq!(
        EscalationTrigger::Interrupted.level(),
        Some(TriggerLevel::Job)
    );
}

#[test]
fn advanced_is_a_step_state_no_status_names() {
    assert!(StepState::Advanced.seen_under().is_empty());
    assert_eq!(StepState::Stopped.seen_under(), &[JobStatus::Escalated]);
}

#[test]
fn an_attestation_debt_is_never_empty() {
    let owed = CriteriaOwed::one(CriterionId::new("c1"));
    assert_eq!(owed.len(), 1);
    assert!(!owed.is_empty());
}
