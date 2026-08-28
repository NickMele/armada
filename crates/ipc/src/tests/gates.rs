//! What a step says about its gates before any of them has acted.
//!
//! The rail is drawn from this, and it drew two of a live `feature` Job's four
//! steps wrong: a step gated `human_always` — where the Job halts and waits for
//! a person — rendered as a step with no check on it, because neither the
//! declaration nor the gate was on the wire at all.

use core_model::{
    AdvanceGate, CriterionId, EvidenceType, Facts, FrozenWorkflow, GamingCheck, GamingPattern, Job,
    JobId, JudgeCheck, JudgeCriterion, ManifestId, ModelName, NewJob, ResolvedStep, StepId,
    StepSeed, Title, TopLevelOrigin, Ulid, Urgency, WorkflowId,
};

use crate::tests::{at, job};
use crate::{decode, encode, DeclaredJudge, JobDetail};

/// The four steps of the `feature` workflow as they were declared on the day
/// the rail drew two of them wrong: a judged step, a judged step with a panel
/// and a second look, a step gated on a person, and a step gated on neither.
fn gated_workflow() -> FrozenWorkflow {
    let criterion = |id: &str| JudgeCriterion {
        criterion_id: CriterionId::new(id),
        question: "does the diff do what the step said it would".to_string(),
    };
    let step = |id: &str, gate: AdvanceGate, judge: Vec<JudgeCheck>| {
        ResolvedStep::frozen(
            StepId::new(id),
            id.to_string(),
            Some(EvidenceType::Diff),
            Vec::new(),
            gate,
            judge,
            None,
        )
    };
    FrozenWorkflow::frozen(
        WorkflowId::carried(Ulid::carried("01WF")),
        "feature".to_string(),
        1,
        vec![
            step("scope", AdvanceGate::Auto, Vec::new()),
            step(
                "implement",
                AdvanceGate::AutoIfJudgePasses,
                vec![JudgeCheck::declared(
                    None,
                    1,
                    vec![criterion("c1"), criterion("c2")],
                    None,
                )],
            ),
            step(
                "tests",
                AdvanceGate::AutoIfJudgePasses,
                vec![JudgeCheck::declared(
                    None,
                    3,
                    vec![criterion("c3")],
                    Some(GamingCheck::declared(
                        None,
                        vec![GamingPattern::AssertionWeakened],
                    )),
                )],
            ),
            // Declared, and inert: no criterion and no pattern is how the
            // domain spells a check that was switched off.
            step(
                "handoff",
                AdvanceGate::HumanAlways,
                vec![JudgeCheck::declared(None, 1, Vec::new(), None)],
            ),
        ],
    )
}

fn gated_job() -> Job {
    let seed = |id: &str, ordinal: u32| StepSeed {
        step_id: StepId::new(id),
        ordinal,
    };
    Job::create_top_level(
        NewJob {
            id: JobId::carried(Ulid::carried("01FEAT")),
            title: Title::new("add the thing").expect("a title"),
            workflow: gated_workflow(),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01MF")),
            urgency: Urgency::Normal,
            atomic: false,
            model: ModelName::new("a-model").expect("a model name"),
            acceptance_criteria: Vec::new(),
            steps: vec![
                seed("scope", 0),
                seed("implement", 1),
                seed("tests", 2),
                seed("handoff", 3),
            ],
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            facts: Facts::new(""),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        },
        TopLevelOrigin::Manual,
        at("2026-08-26T09:00:00.000Z"),
    )
}

/// **The defect this closed, on the step it was worst on.** `handoff` is gated
/// `human_always`: the Job halts there and waits for a person. Nothing on the
/// wire said so, so a rail drew the commonest halt in the fleet as a step with
/// no check on it — the opposite of what happens.
#[test]
fn a_step_says_it_will_stop_for_a_person_before_it_stops() {
    let detail = JobDetail::of(&gated_job(), None, None, &[], None, None);
    let json = encode(&detail).expect("a detail is plain data");

    let gates: Vec<&str> = detail
        .steps
        .iter()
        .map(|step| {
            step.advance_gate
                .expect("the workflow declares it")
                .as_wire()
        })
        .collect();
    assert_eq!(
        gates,
        vec![
            "auto",
            "auto_if_judge_passes",
            "auto_if_judge_passes",
            "human_always"
        ],
        "every step names what it takes to get past it: {json}"
    );
    assert_eq!(
        decode::<JobDetail>("a Job in full", json.as_bytes()).expect("it round-trips"),
        detail
    );
}

/// A declared judge check crosses as counts, and the panel is named only where
/// there is one.
#[test]
fn a_declared_judge_check_crosses_as_counts_and_a_panel_only_above_one() {
    let detail = JobDetail::of(&gated_job(), None, None, &[], None, None);
    let judged: Vec<&Vec<DeclaredJudge>> = detail
        .steps
        .iter()
        .map(|step| {
            step.judge_checks
                .as_ref()
                .expect("the workflow declares it")
        })
        .collect();

    assert!(judged[0].is_empty(), "`scope` asks the Judge nothing");
    assert_eq!(judged[1].len(), 1);
    assert_eq!(judged[1][0].criteria, 2);
    assert_eq!(
        judged[1][0].panel_size, None,
        "one judge is the default and is not a panel"
    );
    assert!(!judged[1][0].gaming_check);
    assert_eq!(judged[2][0].criteria, 1);
    assert_eq!(judged[2][0].panel_size, Some(3));
    assert!(
        judged[2][0].gaming_check,
        "the second look rides along and is worth saying so"
    );
    assert!(
        judged[3].is_empty(),
        "a check with no criterion and no pattern calls no Judge, so it is not one"
    );

    let json = encode(&detail).expect("a detail is plain data");
    assert!(
        !json.contains("does the diff do what the step said"),
        "a criterion's question is a prompt and does not cross: {json}"
    );
    assert!(
        !json.contains("\"panel_size\":1"),
        "absent at one, never present-and-default: {json}"
    );
}

/// **Empty and absent are two sentences here too.** A step Fleet holds no
/// workflow for carries neither key, so a client cannot read "this Fleet cannot
/// say" as "nothing will happen".
#[test]
fn a_step_the_workflow_does_not_declare_carries_neither_key() {
    let job = job();
    let declared = encode(&JobDetail::of(&job, None, None, &[], None, None)).expect("plain data");
    assert!(
        declared.contains("\"judge_checks\":[]") && declared.contains("\"advance_gate\":\"auto\""),
        "a declared step answers both: {declared}"
    );

    let mut detail = JobDetail::of(&job, None, None, &[], None, None);
    detail.steps[0].judge_checks = None;
    detail.steps[0].advance_gate = None;
    let unanswerable = encode(&detail).expect("plain data");
    assert!(
        !unanswerable.contains("judge_checks") && !unanswerable.contains("advance_gate"),
        "absent, never present-and-null: {unanswerable}"
    );
    assert_eq!(
        decode::<JobDetail>("a Job in full", unanswerable.as_bytes()).expect("it round-trips"),
        detail
    );
}
