//! What the machine admits, and what it refuses.
//!
//! Read against `domain/job-transitions.toml` and `domain/job-statuses.toml`:
//! the counts, the terminality and the trigger-bound edges are the registry's
//! claims, asserted here rather than trusted.

use super::*;

// ------------------------------------------------------------- the edge table

#[test]
fn the_edge_table_is_the_registrys_thirty_four_edges() {
    assert_eq!(EDGES.len(), 34);
    for (i, edge) in EDGES.iter().enumerate() {
        for other in &EDGES[i + 1..] {
            assert!(
                !(edge.from == other.from && edge.to == other.to),
                "{} -> {} is in the table twice",
                edge.from.as_wire(),
                edge.to.as_wire()
            );
        }
        assert_ne!(edge.from, edge.to, "the registry names no self-edge");
    }
}

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
        let job = reach(edge.from);
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
