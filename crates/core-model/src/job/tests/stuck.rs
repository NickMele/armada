//! What kind of stuck a Job is, and what moves it.
//!
//! **Every Job here reaches its state by transitioning**, which is this
//! module's own rule and matters more than usual: the classification is read
//! off a stopped step's verdict, and a step constructed into `stopped` would
//! let a test assert against a row the machine would never have written.
//!
//! The four facts in [`Standing`] are the argument, so each case below writes
//! all four out. That is the point of the struct — a case that forgot one would
//! not compile, and the case that mattered on 2026-08-28 was the one where
//! `worktree_on_disk` was false and nobody was asking.

use super::*;

/// A Fleet with everything present: a Drone in the slot, a worktree on disk,
/// Checks passed, the workflow still held. The ordinary escalation.
fn all_there() -> Standing {
    Standing {
        drone_holding: true,
        worktree_on_disk: true,
        checks_passed: true,
        workflow_held: true,
    }
}

/// The same, with the Drone gone — which is what makes a restart the act rather
/// than a redirect.
fn drone_gone() -> Standing {
    Standing {
        drone_holding: false,
        ..all_there()
    }
}

/// A Job escalated on a step-level trigger: the step stopped carrying it, then
/// the status moved, which is the only order the machines admit.
fn stopped_on(trigger: EscalationTrigger) -> Job {
    let running = drive(&created(), &[Target::Queued, Target::Running]);
    let step = StepId::new("repro");
    let stopped = running
        .transition_step(
            &step,
            StepTarget::Running,
            Actor::Fleet,
            at("2026-08-26T09:02:00.000Z"),
        )
        .expect("not_started -> running")
        .job
        .transition_step(
            &step,
            StepTarget::Stopped(StepLevelTrigger::of(trigger).expect("a step-level trigger")),
            Actor::Fleet,
            at("2026-08-26T09:03:00.000Z"),
        )
        .expect("running -> stopped")
        .job;
    drive(&stopped, &[Target::Escalated(trigger)])
}

/// A Job escalated on a Job-level trigger: nothing underneath it stopped, which
/// is the whole of what `stalled` is.
fn stalled() -> Job {
    drive(
        &created(),
        &[
            Target::Queued,
            Target::Running,
            Target::Escalated(EscalationTrigger::Stalled),
        ],
    )
}

fn classify(job: &Job, standing: Standing) -> Stuck {
    Stuck::of(job, None, standing).expect("a stopped Job is classified")
}

/// The absence is half the answer: a Job that has not stopped gets no
/// classification at all, so a surface cannot draw acts against one.
#[test]
fn a_job_that_has_not_stopped_is_not_classified() {
    for status in [
        JobStatus::AwaitingApproval,
        JobStatus::Queued,
        JobStatus::Running,
        JobStatus::AwaitingReview,
        JobStatus::Piloted,
        JobStatus::Superseded,
        JobStatus::CompletedSuccess,
    ] {
        assert!(
            Stuck::of(&reach(status), None, all_there()).is_none(),
            "{} is not a Job a person opens asking why it stopped",
            status.as_wire()
        );
        assert!(!Stuck::asked_of(status), "{}", status.as_wire());
    }
}

/// The Job-level shape. `stalled` names no step, so a restart has nowhere to
/// land — and the Drone is alive, which is exactly what a redirect wants.
#[test]
fn a_stalled_job_over_a_live_drone_is_redirected_and_never_restarted() {
    let stuck = classify(&stalled(), all_there());

    assert_eq!(stuck.step(), None, "a Job-level trigger names no step");
    assert_eq!(
        stuck.recourse(),
        [Recourse::Redirect, Recourse::Redispatch],
        "a live Drone and no stopped step leaves these two"
    );
}

/// The trigger a Job-level escalation carries is on the transition and nowhere
/// else, so a classification that did not read it would have nothing to say
/// about the commonest escalation there is.
#[test]
fn a_job_level_trigger_is_read_off_the_transition_reason() {
    let reason = TransitionReason::Escalation(EscalationTrigger::Stalled);
    let stuck = Stuck::of(&stalled(), Some(&reason), all_there()).expect("escalated");

    assert_eq!(stuck.stopped_by(), Some(EscalationTrigger::Stalled));
}

/// The step-level shape, and the ordering: an override takes nothing away, so
/// it comes first.
#[test]
fn a_refused_step_may_be_overruled_redirected_or_replaced() {
    let stuck = classify(&stopped_on(EscalationTrigger::GateFailure), all_there());

    assert_eq!(stuck.stopped_by(), Some(EscalationTrigger::GateFailure));
    assert_eq!(stuck.step(), Some(&StepId::new("repro")));
    assert_eq!(
        stuck.recourse(),
        [
            Recourse::OverrideVerdict,
            Recourse::Redirect,
            Recourse::Redispatch
        ]
    );
}

/// **The case the whole classification is for.** A Drone that died on a refused
/// step is a restart; the same Job with the worktree deleted under it is not,
/// and Bridge could not tell the two apart because it does not read the
/// filesystem.
#[test]
fn a_worktree_that_is_gone_leaves_only_a_redispatch() {
    let refused = stopped_on(EscalationTrigger::GateFailure);

    let survived = classify(&refused, drone_gone());
    assert!(
        survived.admits(Recourse::RestartStep),
        "the worktree is there"
    );
    assert!(survived.admits(Recourse::OverrideVerdict));

    let lost = classify(
        &refused,
        Standing {
            worktree_on_disk: false,
            ..drone_gone()
        },
    );
    assert_eq!(
        lost.recourse(),
        [Recourse::Redispatch],
        "nothing lands on a worktree that is not there"
    );
    assert!(
        !lost.standing().worktree_on_disk,
        "and the fact says why, rather than leaving the absence to be guessed"
    );
}

/// A restart and a redirect are exclusive, and the Drone decides which — never
/// the person. A surface offering both would be offering one that will fail.
#[test]
fn a_restart_and_a_redirect_are_never_offered_together() {
    for trigger in [EscalationTrigger::GateFailure, EscalationTrigger::Thrashing] {
        let job = stopped_on(trigger);
        for standing in [all_there(), drone_gone()] {
            let stuck = classify(&job, standing);
            assert!(
                !(stuck.admits(Recourse::Redirect) && stuck.admits(Recourse::RestartStep)),
                "{}: {:?}",
                trigger.as_wire(),
                stuck.recourse()
            );
        }
    }
}

/// **A new trigger that changed what a person may do would be the defect.**
/// `no_report` was split out of `thrashing` so the two read differently on the
/// Board; nothing about them behaves differently, and the acts are the proof.
///
/// The standing is `drone_gone` because that is the one the chain leaves: the
/// convergence chain is the single place Fleet stops a Drone itself, so by the
/// time anybody classifies this Job the slot is empty.
#[test]
fn going_quiet_offers_exactly_what_churning_offers() {
    let quiet = classify(&stopped_on(EscalationTrigger::NoReport), drone_gone());
    let churning = classify(&stopped_on(EscalationTrigger::Thrashing), drone_gone());

    assert_eq!(quiet.stopped_by(), Some(EscalationTrigger::NoReport));
    assert_eq!(quiet.step(), Some(&StepId::new("repro")));
    assert_eq!(
        quiet.recourse(),
        [Recourse::RestartStep, Recourse::Redispatch]
    );
    assert_eq!(quiet.recourse(), churning.recourse());
    assert!(
        !quiet.admits(Recourse::OverrideVerdict),
        "nothing weighed the work, so there is no verdict to overrule"
    );
}

/// A gate that could not decide is asked again, and is never overruled: there
/// is no ruling to disagree with.
#[test]
fn an_undecided_gate_is_rerun_and_not_overruled() {
    let stuck = classify(&stopped_on(EscalationTrigger::GateUndecided), all_there());

    assert!(stuck.admits(Recourse::RerunGate));
    assert!(!stuck.admits(Recourse::OverrideVerdict));
}

/// The re-run needs the slot Fleet is standing in, because the baseline the
/// second reading is decided against lives there and nowhere else.
#[test]
fn a_gate_rerun_needs_the_slot_that_holds_the_baseline() {
    let stuck = classify(&stopped_on(EscalationTrigger::GateUndecided), drone_gone());

    assert!(!stuck.admits(Recourse::RerunGate));
    assert_eq!(
        stuck.recourse(),
        [Recourse::RestartStep, Recourse::Redispatch]
    );
}

/// `build` failing is not a matter of opinion. The Checks are read out of the
/// store rather than inferred from the trigger, so a stopped step carrying a
/// failed Check is not overrulable however it was stopped.
#[test]
fn a_failed_check_takes_the_override_away() {
    let stuck = classify(
        &stopped_on(EscalationTrigger::GateFailure),
        Standing {
            checks_passed: false,
            ..all_there()
        },
    );

    assert!(!stuck.admits(Recourse::OverrideVerdict));
    assert!(stuck.admits(Recourse::Redirect), "the Drone is still there");
}

/// Every trigger `overrulable` refuses is a trigger the classification refuses,
/// and the two are the same match rather than two that agree today.
#[test]
fn only_a_machines_ruling_is_overrulable() {
    for trigger in EscalationTrigger::ALL {
        let Some(step_level) = StepLevelTrigger::of(*trigger) else {
            continue;
        };
        let stuck = classify(&stopped_on(*trigger), all_there());
        assert_eq!(
            stuck.admits(Recourse::OverrideVerdict),
            step_level.overrulable(),
            "{}",
            trigger.as_wire()
        );
    }
}

/// A dead end says so. `rejected` never ran, so there is nothing to carry into
/// a replacement — and the empty list is the answer rather than the absence of
/// one, which is why a rejected Job is still classified.
#[test]
fn a_rejected_job_is_classified_and_nothing_moves_it() {
    let stuck = classify(&reach(JobStatus::Rejected), all_there());

    assert!(stuck.recourse().is_empty());
    assert_eq!(
        stuck.stopped_by(),
        None,
        "nothing stopped it — it never ran"
    );
}

/// A terminal failure is replaced and never resumed: the two resume acts take a
/// Job a person is holding, which is an escalated one.
#[test]
fn a_failed_job_is_replaced_and_not_resumed() {
    let stuck = classify(&reach(JobStatus::CompletedFailed), all_there());

    assert_eq!(stuck.recourse(), [Recourse::Redispatch]);
}

/// A workflow renamed or deleted since the Job was created cannot be frozen
/// into a replacement, so the one act left is not offered either.
#[test]
fn a_withdrawn_workflow_takes_the_redispatch_away() {
    let stuck = classify(
        &reach(JobStatus::CompletedFailed),
        Standing {
            workflow_held: false,
            ..all_there()
        },
    );

    assert!(stuck.recourse().is_empty());
}

/// The spellings are the operation inventory's keys, so a surface reading one
/// knows which route to call.
#[test]
fn every_act_is_spelled_as_the_operation_that_performs_it() {
    let spelled: Vec<&str> = Recourse::ALL.iter().map(Recourse::as_wire).collect();
    assert_eq!(
        spelled,
        [
            "override_verdict",
            "rerun_gate",
            "redirect_drone",
            "restart_step",
            "redispatch_job"
        ]
    );
    for act in Recourse::ALL {
        assert_eq!(Recourse::from_wire(act.as_wire()), Some(*act));
    }
    assert_eq!(Recourse::from_wire("pilot"), None);
}
