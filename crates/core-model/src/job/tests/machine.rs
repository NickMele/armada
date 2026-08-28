//! What the machine admits, and what it refuses.
//!
//! Read against `domain/job-transitions.toml` and `domain/job-statuses.toml`:
//! the terminality and the trigger-bound edges are the registry's claims,
//! asserted here rather than trusted.
//!
//! # What [`EDGES`] itself holds is not asserted here, and no longer can be
//!
//! It was, by `EDGES.len() == 34` — a number this file could compare against
//! nothing, since a `no_std` crate cannot read the TOML it was transcribed
//! from. Thirty-four entries with one `from` wrong is still thirty-four, and
//! the assertion passed on exactly that. The gate's `the transition registry
//! and the edge table name the same edges` now matches the two sets both ways,
//! pair by pair, and reports the duplicate and the self-edge that assertion
//! also carried. Everything below tests what the machine *does* with the table,
//! which is the half a comparison rule cannot reach.

use super::*;

#[test]
fn declared_terminality_agrees_with_the_wired_edges() {
    for status in JobStatus::ALL {
        let leaves = status.transitions_out().count();
        assert_eq!(
            status.is_terminal(),
            leaves == 0,
            "{} declares terminal={} and has {leaves} outbound edges",
            status.as_wire(),
            status.is_terminal()
        );
    }
}

#[test]
fn every_status_is_reachable_from_the_entry_status() {
    let mut seen = vec![JobStatus::AwaitingApproval];
    let mut frontier = vec![JobStatus::AwaitingApproval];
    while let Some(status) = frontier.pop() {
        for next in status.transitions_out() {
            if !seen.contains(&next) {
                seen.push(next);
                frontier.push(next);
            }
        }
    }
    assert_eq!(seen.len(), JobStatus::ALL.len(), "unreachable: {seen:?}");
}

#[test]
fn killed_is_reachable_from_every_non_terminal_status() {
    for status in JobStatus::ALL.iter().filter(|s| !s.is_terminal()) {
        let job = reach(*status);
        let moved = job
            .transition(Target::Killed, Actor::Human, at("2026-08-26T10:00:00.000Z"))
            .expect("killed is reachable from every non-terminal status");
        assert_eq!(moved.job.status(), JobStatus::Killed);
    }
}

// ------------------------------------------------------------- what is legal

#[test]
fn every_edge_in_the_table_is_admitted() {
    for edge in EDGES {
        // A guarded edge makes two claims and this is the first: admitted with
        // its condition met. That it refuses without it is asserted below.
        let job = match edge.guard {
            Some(_) => reach_with_every_step_advanced(edge.from),
            None => reach(edge.from),
        };
        let target = target_for(edge.to, edge.escalation_trigger);
        let moved = job
            .transition(target, Actor::Fleet, at("2026-08-26T10:00:00.000Z"))
            .unwrap_or_else(|e| panic!("{} -> {}: {e}", edge.from.as_wire(), edge.to.as_wire()));
        assert_eq!(moved.job.status(), edge.to);
        assert_eq!(moved.event.from(), edge.from);
        assert_eq!(moved.event.to(), edge.to);
    }
}

#[test]
fn every_pair_the_table_does_not_name_is_refused() {
    for from in JobStatus::ALL {
        for to in JobStatus::ALL {
            if EDGES.iter().any(|e| e.from == *from && e.to == *to) {
                continue;
            }
            let job = reach(*from);
            let error = job
                .transition(
                    target_for(*to, None),
                    Actor::Fleet,
                    at("2026-08-26T10:00:00.000Z"),
                )
                .expect_err("an edge the registry does not name was admitted");
            let expected = if from.is_terminal() {
                IllegalTransition::FromTerminal {
                    from: *from,
                    to: *to,
                }
            } else {
                IllegalTransition::NoSuchEdge {
                    from: *from,
                    to: *to,
                }
            };
            assert_eq!(error, expected);
        }
    }
}

#[test]
fn no_status_transitions_to_itself() {
    for status in JobStatus::ALL {
        let job = reach(*status);
        assert!(job
            .transition(
                target_for(*status, None),
                Actor::Fleet,
                at("2026-08-26T10:00:00.000Z")
            )
            .is_err());
    }
}

#[test]
fn an_edge_that_belongs_to_a_trigger_refuses_any_other() {
    let job = reach(JobStatus::AwaitingReview);
    let error = job
        .transition(
            Target::Escalated(EscalationTrigger::Stalled),
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect_err("the liveness clock is suspended at a human gate, so stalled cannot fire");
    assert_eq!(
        error,
        IllegalTransition::WrongTrigger {
            from: JobStatus::AwaitingReview,
            to: JobStatus::Escalated,
            expected: EscalationTrigger::Interrupted,
            given: EscalationTrigger::Stalled,
        }
    );
    assert!(job
        .transition(
            Target::Escalated(EscalationTrigger::Interrupted),
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z")
        )
        .is_ok());
}

#[test]
fn the_default_escalation_edge_accepts_every_trigger() {
    for trigger in EscalationTrigger::ALL {
        let job = reach(JobStatus::Running);
        let moved = job
            .transition(
                Target::Escalated(*trigger),
                Actor::Fleet,
                at("2026-08-26T10:00:00.000Z"),
            )
            .unwrap_or_else(|e| panic!("running -> escalated({}): {e}", trigger.as_wire()));
        assert_eq!(
            moved.event.reason(),
            &TransitionReason::Escalation(*trigger)
        );
    }
}

#[test]
fn every_declared_trigger_edge_is_in_the_table() {
    for trigger in EscalationTrigger::ALL {
        let Some((from, to)) = trigger.declared_edge() else {
            continue;
        };
        let edge = EDGES
            .iter()
            .find(|e| e.from == from && e.to == to)
            .expect("a trigger names an edge the table does not have");
        assert_eq!(edge.escalation_trigger, Some(*trigger));
    }
}

// ------------------------------------------------------------- what a guard does

/// The condition `completed_success` has claimed since the registry was
/// written, and could not hold until an edge could carry one. Issue #189.
#[test]
fn completed_success_is_refused_while_a_step_has_not_advanced() {
    for from in [
        JobStatus::Running,
        JobStatus::AwaitingReview,
        JobStatus::AwaitingAttestation,
        JobStatus::Piloted,
    ] {
        let job = reach(from);
        let error = job
            .transition(
                Target::CompletedSuccess,
                Actor::Fleet,
                at("2026-08-26T10:00:00.000Z"),
            )
            .expect_err("a Job whose steps have not advanced cannot complete");
        assert_eq!(
            error,
            IllegalTransition::GuardRefused {
                from,
                to: JobStatus::CompletedSuccess,
                guard: Guard::EveryStepAdvanced,
                step_id: StepId::new("repro"),
                holding: StepState::NotStarted,
            }
        );
    }
}

/// **A refused guard is not a refused edge**, and a caller that could not tell
/// them apart would report a Job that is not ready as a move nothing sanctions.
#[test]
fn a_refused_guard_names_the_guard_and_is_not_a_missing_edge() {
    let job = reach(JobStatus::Running);
    let error = job
        .transition(
            Target::CompletedSuccess,
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect_err("the steps have not advanced");
    assert!(
        !matches!(error, IllegalTransition::NoSuchEdge { .. }),
        "the edge exists; it is the condition that failed"
    );
    let said = format!("{error}");
    assert!(said.contains("every_step_advanced"), "{said}");
    assert!(said.contains("repro"), "{said}");
}

/// The half a guard would be useless without: it admits the move once the
/// condition holds, and the condition is met by walking the inner machine.
#[test]
fn completed_success_is_admitted_once_every_step_has_advanced() {
    let job = reach_with_every_step_advanced(JobStatus::Running);
    let moved = job
        .transition(
            Target::CompletedSuccess,
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect("every step advanced");
    assert_eq!(moved.job.status(), JobStatus::CompletedSuccess);
    assert!(moved
        .job
        .steps()
        .iter()
        .all(|row| row.state() == StepState::Advanced));
}

/// **One advanced step is not every step.** A guard reading only the step the
/// cursor names would pass this, which is the reading the registry's own
/// "last step advanced" invites and the one a predicate must not take.
#[test]
fn advancing_the_last_step_alone_does_not_satisfy_the_guard() {
    let job = reach(JobStatus::Running);
    let mut job = job;
    for target in [StepTarget::Running, StepTarget::Advanced] {
        job = job
            .transition_step(
                &StepId::new("fix"),
                target,
                Actor::Fleet,
                at("2026-08-26T09:30:00.000Z"),
            )
            .expect("the last step advances")
            .job;
    }
    let error = job
        .transition(
            Target::CompletedSuccess,
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect_err("the first step never ran");
    assert_eq!(
        error,
        IllegalTransition::GuardRefused {
            from: JobStatus::Running,
            to: JobStatus::CompletedSuccess,
            guard: Guard::EveryStepAdvanced,
            step_id: StepId::new("repro"),
            holding: StepState::NotStarted,
        }
    );
}

/// **The guard reads the latest attempt, because the row is the latest
/// attempt.** #63 made a step workable twice; a step handed back is `retrying`
/// on its row whatever it held on the run before, so the earlier pass cannot
/// satisfy the guard on its behalf.
#[test]
fn a_step_handed_back_for_another_attempt_does_not_satisfy_the_guard() {
    let why = StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("a step-level trigger");
    let mut job = reach(JobStatus::Running);
    for (step, targets) in [
        ("repro", vec![StepTarget::Running, StepTarget::Advanced]),
        ("fix", vec![StepTarget::Running, StepTarget::Retrying(why)]),
    ] {
        for target in targets {
            job = job
                .transition_step(
                    &StepId::new(step),
                    target,
                    Actor::Fleet,
                    at("2026-08-26T09:40:00.000Z"),
                )
                .unwrap_or_else(|e| panic!("moving {step}: {e}"))
                .job;
        }
    }
    let error = job
        .transition(
            Target::CompletedSuccess,
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect_err("a step going round again has not advanced");
    assert_eq!(
        error,
        IllegalTransition::GuardRefused {
            from: JobStatus::Running,
            to: JobStatus::CompletedSuccess,
            guard: Guard::EveryStepAdvanced,
            step_id: StepId::new("fix"),
            holding: StepState::Retrying,
        }
    );
}

/// A guard refuses without moving anything, exactly as a refused edge does.
#[test]
fn a_refused_guard_leaves_the_job_exactly_as_it_was() {
    let job = reach(JobStatus::Running);
    let before = job.clone();
    let _ = job.transition(
        Target::CompletedSuccess,
        Actor::Fleet,
        at("2026-08-26T10:00:00.000Z"),
    );
    assert_eq!(job, before);
}

/// **Only the edges the registry guards are guarded.** A condition that leaked
/// onto its neighbours would stop a Job being killed or escalated mid-step,
/// which is the opposite of what the machine has to allow — an escalation is
/// how a running step gets a person, so guarding that edge would trap the Job
/// this one exists to rescue.
#[test]
fn only_the_two_endings_out_of_running_carry_a_condition() {
    for edge in EDGES.iter().filter(|e| e.from == JobStatus::Running) {
        let expected = match edge.to {
            JobStatus::CompletedSuccess => Some(Guard::EveryStepAdvanced),
            JobStatus::CompletedFailed => Some(Guard::NoStepRunning),
            _ => None,
        };
        assert_eq!(
            edge.guard,
            expected,
            "{} -> {} carries the wrong condition",
            edge.from.as_wire(),
            edge.to.as_wire()
        );
    }
}

/// **The guard is on the one inbound edge, and the other two are unguarded on
/// purpose.** `escalated -> completed_failed` is a person accepting the failure
/// of a Job escalated on `stalled`, which holds a `running` step legitimately —
/// so `completed_failed`'s `step_states` row does not narrow behind this guard
/// and is not waiting to. Guarding all three, the way `every_step_advanced`
/// guards all four of its own, would refuse that person's decision.
#[test]
fn only_the_edge_from_running_ends_a_job_on_a_condition() {
    for edge in EDGES.iter().filter(|e| e.to == JobStatus::CompletedFailed) {
        assert_eq!(
            edge.guard.is_some(),
            edge.from == JobStatus::Running,
            "{} -> {} carries the wrong condition",
            edge.from.as_wire(),
            edge.to.as_wire()
        );
    }
}

/// The write #179 observed, refused. A Job whose step is still being worked
/// cannot be ended as failed from `running`, and the refusal names the step
/// rather than only the Job — "cannot complete" is useless, "step `fix` is
/// running" is the finding.
#[test]
fn completed_failed_is_refused_from_running_while_a_step_is_being_worked() {
    let job = reach(JobStatus::Running);
    let job = job
        .transition_step(
            &StepId::new("fix"),
            StepTarget::Running,
            Actor::Fleet,
            at("2026-08-26T09:30:00.000Z"),
        )
        .expect("a step is entered")
        .job;
    let error = job
        .transition(
            Target::CompletedFailed,
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect_err("a step still being worked cannot be left running beneath a terminal Job");
    assert_eq!(
        error,
        IllegalTransition::GuardRefused {
            from: JobStatus::Running,
            to: JobStatus::CompletedFailed,
            guard: Guard::NoStepRunning,
            step_id: StepId::new("fix"),
            holding: StepState::Running,
        }
    );
    let said = format!("{error}");
    assert!(said.contains("fix"), "{said}");
    assert!(said.contains("no_step_running"), "{said}");
}

/// The half the guard would be useless without: the step is stopped with the
/// trigger that stopped it, and the Job then ends. **This is the order
/// `fleet::dispatch` walks**, and the states the machine leaves behind are the
/// ones `completed_failed` declares.
#[test]
fn completed_failed_is_admitted_once_the_worked_step_is_stopped() {
    let why = StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("a step-level trigger");
    let mut job = reach(JobStatus::Running);
    for target in [StepTarget::Running, StepTarget::Stopped(why)] {
        job = job
            .transition_step(
                &StepId::new("fix"),
                target,
                Actor::Fleet,
                at("2026-08-26T09:30:00.000Z"),
            )
            .expect("a step is entered and stopped")
            .job;
    }
    let moved = job
        .transition(
            Target::CompletedFailed,
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect("no step is running");
    assert_eq!(moved.job.status(), JobStatus::CompletedFailed);
    assert!(moved
        .job
        .steps()
        .iter()
        .all(|row| row.state() != StepState::Running));
}

/// **A step that was never entered does not hold the Job open.** The guard says
/// no step is running, not that every step ran: a Job that fails on its first
/// step leaves the rest `not_started`, and refusing that would make a failure
/// unrecordable.
#[test]
fn a_step_that_never_started_does_not_refuse_the_ending() {
    let job = reach(JobStatus::Running);
    let moved = job
        .transition(
            Target::CompletedFailed,
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect("a step that was never entered is not being worked");
    assert_eq!(moved.job.status(), JobStatus::CompletedFailed);
}

// ------------------------------------------------------- what does not move

#[test]
fn a_refused_transition_leaves_the_job_exactly_as_it_was() {
    let job = reach(JobStatus::Running);
    let before = job.clone();
    let _ = job.transition(
        Target::Superseded,
        Actor::Fleet,
        at("2026-08-26T10:00:00.000Z"),
    );
    assert_eq!(job, before);
}

#[test]
fn a_transition_moves_the_status_and_nothing_else() {
    let job = reach(JobStatus::Running);
    let moved = job
        .transition(
            Target::AwaitingReview,
            Actor::Fleet,
            at("2026-08-26T10:00:00.000Z"),
        )
        .expect("running -> awaiting_review");
    assert_eq!(
        moved.job.steps(),
        job.steps(),
        "the inner machine did not move"
    );
    assert_eq!(moved.job.acceptance_criteria(), job.acceptance_criteria());
    assert_eq!(moved.job.current_step_id(), job.current_step_id());
    assert_eq!(moved.job.id(), job.id());
}

#[test]
fn a_terminal_job_goes_nowhere_at_all() {
    for terminal in JobStatus::ALL.iter().filter(|s| s.is_terminal()) {
        let job = reach(*terminal);
        for to in JobStatus::ALL {
            assert_eq!(
                job.transition(
                    target_for(*to, None),
                    Actor::Human,
                    at("2026-08-26T10:00:00.000Z")
                ),
                Err(IllegalTransition::FromTerminal {
                    from: *terminal,
                    to: *to
                })
            );
        }
    }
}
