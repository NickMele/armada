//! What a Judge is told, what it may answer, and what an answer may do.
//!
//! Every case here is about a property the design states as constitutional, so
//! each one names the rule it holds: veto-only, blind to the Drone, a narrow
//! question, a refusal that cites, and unanimity rather than a majority.

use adapter_traits::Patch;
use config::ResolvedWorkflow;
use core_model::{
    CheckOutcome, CriterionId, JudgeCriterion, JudgeVerdict, Judgment, StepCheck, StepId,
};
use testkit::{Gate, Sketch};

use crate::mechanical::CheckFailed;
use crate::{Brief, Refusals, Unreadable, Verdict};

/// A step that asks one question, and one that asks none.
fn workflow() -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "fix",
            label: "Fix",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[("c1", "Does the fix address the cause the note names?")],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "merge",
            label: "Merge",
            evidence_type: None,
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

fn checks() -> Vec<StepCheck> {
    vec![StepCheck {
        name: "build".to_string(),
        outcome: CheckOutcome::Passed,
        expected: None,
        produced: None,
        output_path: None,
    }]
}

fn brief(workflow: &ResolvedWorkflow) -> Brief {
    let step = &workflow.steps()[0];
    Brief::about(
        step,
        &step.judge_checks()[0].criteria()[0],
        &Patch::of("--- a/src/log.rs\n+++ b/src/log.rs\n+    let n = n - 1;\n".to_string()),
        &checks(),
    )
}

fn refusal(id: &str) -> Judgment {
    Judgment {
        criterion_id: CriterionId::new(id),
        verdict: JudgeVerdict::NotMet,
        expected: Some("the loop stops at n".into()),
        produced: Some("the loop stops at n - 1".into()),
        consequence: Some("the last row is dropped".into()),
    }
}

fn met(id: &str) -> Judgment {
    Judgment {
        criterion_id: CriterionId::new(id),
        verdict: JudgeVerdict::Met,
        expected: None,
        produced: None,
        consequence: None,
    }
}

// ------------------------------------------------------- what it is told

/// Rule 2. The Drone's own account is not a parameter of `Brief::about`, so
/// this asserts what a compiler cannot: that no wording from a submission
/// reaches the question by some other route.
#[test]
fn the_question_carries_the_diff_and_the_facts_and_nothing_the_drone_said() {
    let workflow = workflow();
    let question = brief(&workflow).question().to_string();
    assert!(question.contains("let n = n - 1"), "{question}");
    assert!(question.contains("build"), "{question}");
    assert!(question.contains("passed"), "{question}");
    assert!(
        question.contains("Does the fix address the cause the note names?"),
        "{question}"
    );
}

/// Rule 3. One criterion per call, so the answer has a wrong value.
#[test]
fn one_call_asks_one_criterion_and_the_answer_has_two_legal_words() {
    let workflow = workflow();
    let brief = brief(&workflow);
    assert_eq!(brief.criterion(), &CriterionId::new("c1"));
    assert!(brief.question().contains("verdict: met"));
    assert!(brief.question().contains("verdict: not_met"));
}

// ------------------------------------------------------ what it may answer

#[test]
fn a_refusal_is_read_back_as_its_three_named_fields() {
    let workflow = workflow();
    let judged = brief(&workflow)
        .read(
            "verdict: not_met\nexpected: the loop stops at n\n\
             produced: the loop stops at n - 1\nconsequence: the last row is dropped",
        )
        .expect("a cited refusal");
    assert_eq!(judged.verdict, JudgeVerdict::NotMet);
    assert_eq!(judged.criterion_id, CriterionId::new("c1"));
    assert_eq!(
        judged.consequence.as_deref(),
        Some("the last row is dropped")
    );
}

/// Rule 4. An uncited refusal is unactionable for the Drone and for the person,
/// so it is an answer this cannot act on rather than a refusal that is empty.
#[test]
fn a_refusal_that_cites_nothing_is_not_a_refusal() {
    let workflow = workflow();
    assert_eq!(
        brief(&workflow).read("verdict: not_met"),
        Err(Unreadable::RefusalCitesNothing)
    );
    assert_eq!(
        brief(&workflow).read("verdict: not_met\nexpected: \nproduced: x\nconsequence: y"),
        Err(Unreadable::RefusalCitesNothing)
    );
}

#[test]
fn prose_instead_of_a_verdict_is_neither_a_refusal_nor_a_pass() {
    let workflow = workflow();
    assert_eq!(
        brief(&workflow).read("Looks good to me — I'd ship it."),
        Err(Unreadable::NoVerdict)
    );
    assert_eq!(brief(&workflow).read(""), Err(Unreadable::NoVerdict));
}

// -------------------------------------------------------------- the veto

/// Rule 1, stated as the property that matters: there is no value of the
/// second argument that turns a mechanical failure into an advance.
#[test]
fn no_judge_answer_can_advance_a_step_whose_check_failed() {
    let failed = || Verdict::Failed(vec![CheckFailed::DiffEmpty]);
    assert!(!failed().but_for(None).advanced());
    assert!(!failed()
        .but_for(Refusals::among(&[refusal("c1")]))
        .advanced());
    assert!(!failed().but_for(Refusals::among(&[met("c1")])).advanced());
}

#[test]
fn a_refusal_takes_a_mechanical_pass_away_and_a_no_objection_leaves_it() {
    assert!(matches!(
        Verdict::Advance.but_for(Refusals::among(&[refusal("c1")])),
        Verdict::Refused(_)
    ));
    assert!(Verdict::Advance
        .but_for(Refusals::among(&[met("c1")]))
        .advanced());
    assert!(Verdict::Advance.but_for(None).advanced());
}

/// Rule 5, on both axes: any single refusal fails the step, however many
/// judges or criteria answered otherwise.
#[test]
fn one_refusal_among_many_no_objections_fails_the_step() {
    let heard = vec![met("c1"), met("c1"), refusal("c1"), met("c2")];
    let refusals = Refusals::among(&heard).expect("a refusal among them");
    assert_eq!(refusals.cited().len(), 1);
    assert_eq!(refusals.criteria(), vec![&CriterionId::new("c1")]);
    assert!(matches!(
        Verdict::Advance.but_for(Some(refusals)),
        Verdict::Refused(_)
    ));
}

#[test]
fn a_panel_that_all_declined_to_refuse_produces_no_refusals_at_all() {
    assert!(Refusals::among(&[met("c1"), met("c1"), met("c1")]).is_none());
    assert!(Refusals::among(&[]).is_none());
}

// ----------------------------------------------------------- cold by default

/// The trigger, read off the declaration. A step that asks nothing costs
/// nothing, and most steps ask nothing.
#[test]
fn a_step_that_declares_no_criterion_asks_the_judge_nothing() {
    let workflow = workflow();
    let merge = workflow
        .steps()
        .iter()
        .find(|step| step.id() == &StepId::new("merge"))
        .expect("the second step");
    assert!(!merge.asks_the_judge());
    assert_eq!(merge.judge_calls(), 0);

    let fix = &workflow.steps()[0];
    assert!(fix.asks_the_judge());
    assert_eq!(fix.judge_calls(), 1);
}

/// The cost, stated where it can be checked rather than in prose: the call
/// count is criteria times panel size, and it multiplies.
#[test]
fn the_call_count_is_criteria_times_panel_size() {
    let criteria: Vec<JudgeCriterion> = ["c1", "c2", "c3", "c4"]
        .iter()
        .map(|id| JudgeCriterion {
            criterion_id: CriterionId::new(*id),
            question: "Is it right?".to_string(),
        })
        .collect();
    let check = core_model::JudgeCheck::declared(None, 3, criteria, None);
    assert_eq!(check.calls(), 12);
    // A panel of nobody is not a legal reading of a declared check.
    assert_eq!(
        core_model::JudgeCheck::declared(None, 0, Vec::new(), None).panel_size(),
        1
    );
}
