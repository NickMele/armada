//! `plan_recorded`: the plan step's gate reads Fleet's record of the plan and
//! nothing the Drone wrote. The step is built frozen, because no workflow file
//! can declare the check until `config` reads it (#895).

use std::num::NonZeroU32;

use core_model::{AdvanceGate, EvidenceType, ResolvedCheck, ResolvedStep, StepId};

use crate::mechanical::{CheckFailed, Observed, Ran};

fn a_plan_step(min_tasks: u32) -> ResolvedStep {
    ResolvedStep::frozen(
        StepId::new("plan"),
        "Plan the change".to_string(),
        Some(EvidenceType::Plan),
        vec![ResolvedCheck::PlanRecorded {
            min_tasks: NonZeroU32::new(min_tasks).expect("at least one"),
        }],
        AdvanceGate::Auto,
        Vec::new(),
        None,
        0,
        None,
    )
}

#[test]
fn a_plan_holding_enough_tasks_passes() {
    let step = a_plan_step(2);
    assert!(
        step.records_plan(),
        "a step whose product is a plan records it"
    );
    let ran = Ran::of(&step, &[Observed::Plan { tasks: Some(2) }]).expect("the check ran");
    assert!(ran.all_passed());
    assert_eq!(ran.recorded()[0].name, "plan_recorded");
}

/// **No plan is not a plan of none**, and each says so in its own words.
#[test]
fn no_plan_and_too_few_tasks_each_stop_the_step_and_say_which() {
    let step = a_plan_step(3);
    for (tasks, produced) in [
        (None, "no plan was recorded"),
        (Some(2), "the plan holds 2 tasks not dropped"),
    ] {
        let ran = Ran::of(&step, &[Observed::Plan { tasks }]).expect("the check ran");
        assert!(!ran.advances(), "{tasks:?} advanced the step");
        let failed = ran.failures();
        assert_eq!(
            failed.as_slice(),
            [CheckFailed::TooFewTasks {
                min_tasks: NonZeroU32::new(3).expect("three"),
                recorded: tasks,
            }]
        );
        assert_eq!(
            failed[0].expected(),
            "the step records a plan of at least 3 tasks with `record_plan`"
        );
        assert_eq!(failed[0].produced(), produced);
        assert!(failed[0].the_drone_can_answer());
    }
}

#[test]
fn a_plan_check_handed_another_kind_of_observation_is_refused() {
    let step = a_plan_step(1);
    assert!(Ran::of(&step, &[Observed::Diff { moved: true }]).is_err());
}
