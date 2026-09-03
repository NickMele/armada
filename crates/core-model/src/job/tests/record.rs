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

// -------------------------------------------------------------------- title

/// The empty string is a title to the type system and not one to a reader, so
/// it is refused where the text is typed rather than checked on every write.
#[test]
fn a_blank_title_is_not_a_title() {
    for blank in ["", " ", "\t\n  "] {
        assert_eq!(
            Title::new(blank),
            Err(BlankTitle),
            "{blank:?} names no Job a person could pick out of a list"
        );
    }
}

/// Trimmed, so two names that read identically are the same value and a list
/// cell never begins with whitespace nobody typed on purpose.
#[test]
fn a_title_is_stored_trimmed() {
    let padded = Title::new("  fix the parser  ").expect("a title");
    assert_eq!(padded.as_str(), "fix the parser");
    assert_eq!(padded, Title::new("fix the parser").expect("a title"));
}

/// Creation carries it and nothing moves it. There is no setter — a title is
/// correctable in principle, and the write that corrects one does not exist.
#[test]
fn the_title_survives_creation_and_every_transition() {
    let job = reach(JobStatus::CompletedSuccess);
    assert_eq!(job.title().as_str(), "fix the parser's off-by-one");
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
    for source in CriterionSource::ALL {
        assert_eq!(CriterionSource::from_wire(source.as_wire()), Some(*source));
    }
    for direction in DependencyDirection::ALL {
        assert_eq!(
            DependencyDirection::from_wire(direction.as_wire()),
            Some(*direction)
        );
    }
    for reason in NotRunReason::ALL {
        assert_eq!(NotRunReason::from_wire(reason.as_wire()), Some(*reason));
    }
    for reason in PilotReason::ALL {
        assert_eq!(PilotReason::from_wire(reason.as_wire()), Some(*reason));
    }
    for actor in Actor::ALL {
        assert_eq!(Actor::from_wire(actor.as_wire()), Some(*actor));
    }
    assert_eq!(JobStatus::from_wire("in_progress"), None);
}

/// [`GateOutcome`] has a payload, so it has no `ALL` to scan. The set it round
/// trips over is built here instead — every variant, and every reason the one
/// that carries a reason can carry.
#[test]
fn every_gate_outcome_round_trips_including_each_reason() {
    let mut outcomes = vec![GateOutcome::RanAndPassed, GateOutcome::RanAndFailed];
    outcomes.extend(
        NotRunReason::ALL
            .iter()
            .copied()
            .map(GateOutcome::DidNotRun),
    );
    for outcome in outcomes {
        let (stored, reason) = outcome.as_wire();
        assert_eq!(GateOutcome::from_wire(stored, reason), Some(outcome));
    }
    // The reason column is `Some` exactly when there is a reason. Neither
    // mismatch reads back as an outcome.
    assert_eq!(
        GateOutcome::from_wire("ran_and_passed", Some("frozen")),
        None
    );
    assert_eq!(GateOutcome::from_wire("did_not_run", None), None);
    assert_eq!(GateOutcome::from_wire("did_not_run", Some("bored")), None);
}

/// The narrowing back from [`Origin`], which the rebuild needs to pick a
/// constructor. Round trips through `From<TopLevelOrigin>` for the four, and
/// declines for the one written by the other constructor.
#[test]
fn every_top_level_origin_narrows_back_to_itself() {
    for origin in Origin::ALL {
        match origin.top_level() {
            Some(top_level) => assert_eq!(Origin::from(top_level), *origin),
            None => assert_eq!(*origin, Origin::SubDispatched),
        }
    }
    assert!(Origin::SubDispatched.top_level().is_none());
}

/// A tripwire on the enums' own size, and **not** a cross-check against the
/// registries.
///
/// It was named for one and never performed one: it asserted thirteen against
/// `EscalationTrigger::ALL` while `escalation-triggers.toml` held thirteen keys
/// and the enum was missing one of them, and passed, because nothing here reads
/// the file. `core-model` has zero
/// dependencies and cannot parse TOML, so nothing here ever will — the
/// comparison lives in `cargo xtask verify-foundations`, which reads both sides
/// and fails on either missing the other.
///
/// What this is worth on its own is the tripwire: a variant added or dropped
/// changes a number here, so it cannot be done without touching a line whose
/// only purpose is to be noticed. That is a prompt to go and check the
/// registry, not evidence that anyone did.
#[test]
fn each_enums_size_is_pinned_here_and_compared_to_no_registry() {
    assert_eq!(JobStatus::ALL.len(), 12);
    assert_eq!(StepState::ALL.len(), 6);
    assert_eq!(EscalationTrigger::ALL.len(), 22);
    assert_eq!(Origin::ALL.len(), 5);
    assert_eq!(PilotReason::ALL.len(), 3);
    assert_eq!(CriterionSource::ALL.len(), 3);
    assert_eq!(DependencyDirection::ALL.len(), 2);
    assert_eq!(NotRunReason::ALL.len(), 4);
    assert_eq!(Actor::ALL.len(), 3);
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

/// `last_verdict` admits step-level triggers only, so a trigger with no level
/// is one nothing can check against that rule. There is none.
#[test]
fn every_trigger_says_whether_it_is_about_a_step_or_about_the_job() {
    let step: Vec<&str> = EscalationTrigger::ALL
        .iter()
        .filter(|t| t.level() == TriggerLevel::Step)
        .map(|t| t.as_wire())
        .collect();
    assert_eq!(
        step,
        vec![
            "blocked_by_policy",
            "check_timeout",
            "drone_killed",
            "evidence_suspect",
            "evidence_too_large",
            "gate_failure",
            "gate_undecided",
            "loop_cap",
            "no_report",
            "run_ended",
            "thrashing"
        ]
    );
    assert_eq!(EscalationTrigger::Interrupted.level(), TriggerLevel::Job);
    // A sub-kind reads its parent's rather than carrying a second answer.
    assert_eq!(
        EscalationTrigger::Silent.level(),
        EscalationTrigger::Stalled.level()
    );
}

/// The assertion this replaces was `advanced_is_a_step_state_no_status_names`,
/// and it held `Stopped.seen_under()` to `[escalated]` alone — which is issue
/// #184: a Job escalated on `stalled` holds a step that is still `running`, so
/// `escalated` is not the only status a stopped step is seen beneath and was
/// never the only one. A frozen step crosses an unguarded edge holding what it
/// held.
///
/// **`completed_success` is the one status that is not everywhere**, which is
/// issue #189: every edge arriving there is guarded on `every_step_advanced`,
/// so no state but `advanced` can be carried across one. That is the whole
/// difference a guard makes to this relation, stated once.
#[test]
fn a_step_state_the_machine_reaches_is_seen_beneath_every_status_but_the_guarded_one() {
    assert_eq!(StepState::Advanced.seen_under(), JobStatus::ALL);
    for state in [
        StepState::Stopped,
        StepState::NotStarted,
        StepState::Retrying,
        StepState::Running,
    ] {
        assert!(
            !state.seen_under().contains(&JobStatus::CompletedSuccess),
            "{} is seen beneath a status guarded against it",
            state.as_wire()
        );
        assert_eq!(
            state.seen_under().len(),
            JobStatus::ALL.len() - 1,
            "{} is seen beneath every status but the guarded one",
            state.as_wire()
        );
    }
    // The one nothing reaches yet is where its design puts it. `retrying`
    // was beside it here and does not belong: `Running -> Retrying` is a
    // real edge, so a step between attempts is carried across an unguarded
    // status edge like any other frozen step, and it is in the loop above.
    assert_eq!(
        StepState::AwaitingHuman.seen_under(),
        &[JobStatus::AwaitingReview]
    );
}

#[test]
fn an_attestation_debt_is_never_empty() {
    let owed = CriteriaOwed::one(CriterionId::new("c1"));
    assert_eq!(owed.len(), 1);
    assert!(!owed.is_empty());
}

/// Every `flag_if` value reads back as the one it was written as. A pattern
/// whose wire value did not round-trip would come off a frozen workflow as a
/// pattern nothing looks for, which reads exactly like a step that never asked.
#[test]
fn every_gaming_pattern_survives_the_wire() {
    for pattern in GamingPattern::ALL {
        assert_eq!(
            GamingPattern::from_wire(pattern.as_wire()),
            Some(*pattern),
            "{}",
            pattern.as_wire()
        );
    }
    assert!(GamingPattern::from_wire("looks_dodgy").is_none());
}

/// **The defect the first production flag had.** A test split into a loop and a
/// special case needing different setup asserts less in the loop — truthfully —
/// and asserts the same thing twenty lines below, in the same patch. A question
/// stopping at "does it assert less" is answered `yes` and is still wrong about
/// the change, so it has to reach the rest of the patch.
///
/// The teeth are asserted beside it: an assertion made nowhere else is what
/// this pattern is for, and softening that away would pass this test only by
/// deleting a line of it.
#[test]
fn the_weakened_assertion_question_asks_where_the_assertion_went() {
    let asked = GamingPattern::AssertionWeakened
        .question()
        .expect("a judged pattern has a question");
    assert!(
        asked.contains("made nowhere else in this change"),
        "the question has to reach the rest of the patch: {asked}"
    );
    assert!(
        asked.contains("asserts less"),
        "and still be the question it was: {asked}"
    );
}

/// **The second defect in the same flag.** It cited two assertions the change
/// had copied verbatim out of the test it was splitting — a fair criticism of
/// that test's standard, and no claim at all about the change. So the question
/// separates an assertion this change wrote from one it carried, and keeps both
/// arms it had: a vacuous test written here, and an existing one this change
/// hollowed out.
#[test]
fn the_tautological_test_question_tells_a_written_assertion_from_a_carried_one() {
    let asked = GamingPattern::TautologicalTest
        .question()
        .expect("a judged pattern has a question");
    assert!(
        asked.contains("moved or copied unchanged"),
        "a standard that was already here is not this change's doing: {asked}"
    );
    assert!(
        asked.contains("write a test that would pass whatever the code"),
        "a vacuous test written here is still flagged: {asked}"
    );
    assert!(
        asked.contains("leave an existing one passing"),
        "and so is one this change hollowed out: {asked}"
    );
}

/// A gaming check with no pattern is a check that does not fire, which is the
/// same representation an absent one has — one way to be off, as everywhere
/// else in this file.
#[test]
fn a_gaming_check_naming_no_pattern_does_not_fire_and_costs_nothing() {
    let silent = GamingCheck::declared(EvidenceRef::parse("scope.evidence"), Vec::new());
    assert!(!silent.fires());
    assert_eq!(silent.calls(), 0);

    // Two patterns, one call: the diff answers `test_deleted` for free.
    let watching = GamingCheck::declared(
        None,
        vec![GamingPattern::TestDeleted, GamingPattern::TautologicalTest],
    );
    assert!(watching.fires());
    assert_eq!(watching.calls(), 1);
    assert!(watching.baseline().is_none(), "a first step has none");
}
