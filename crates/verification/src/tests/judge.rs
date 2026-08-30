//! What a Judge is told, what it may answer, and what an answer may do.
//!
//! Every case here is about a property the design states as constitutional, so
//! each one names the rule it holds: veto-only, blind to the Drone, a narrow
//! question, a refusal that cites, and unanimity rather than a majority.

use adapter_traits::Patch;
use config::ResolvedWorkflow;
use core_model::{
    CheckOutcome, CriterionId, JudgeCriterion, JudgeVerdict, Judgment, StepCheck, StepEvidence,
    StepId,
};
use testkit::{Gate, Sketch};

use crate::mechanical::CheckFailed;
use crate::{
    Accepted, Brief, Claimed, NotClaimed, Product, Reference, Refusals, Request, ShownBy,
    Submission, Unreadable, Verdict,
};

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

/// A `diff` submission, which is the shape the fixture's first step declares.
///
/// Its wording is deliberately distinctive, because the assertion that matters
/// most here is that **none of it** reaches the question.
fn submitted() -> Submission {
    Submission::submitted(
        config::EvidenceType::Diff,
        Claimed("I fixed the off-by-one and I am confident it is right"),
        ShownBy("`cargo test -p log` exit 0"),
        NotClaimed("I did not check the callers"),
    )
    .expect("a submission")
}

fn patch() -> Patch {
    Patch::of("--- a/src/log.rs\n+++ b/src/log.rs\n+    let n = n - 1;\n".to_string())
}

fn brief(workflow: &ResolvedWorkflow) -> Brief {
    brief_measured_against(workflow, &[])
}

/// The same brief with an earlier step's note in front of it — the shape of the
/// Job the quotation check was written for, where a scope note is the yardstick.
fn brief_measured_against(workflow: &ResolvedWorkflow, references: &[Reference<'_>]) -> Brief {
    let step = &workflow.steps()[0];
    let submitted = submitted();
    let patch = patch();
    let accepted = Accepted::of(step, &submitted).expect("the step asks for a diff");
    let product = Product::of(step, &patch, accepted).expect("the step changed something");
    Brief::about(
        step,
        &step.judge_checks()[0].criteria()[0],
        Request::of(testkit::asked_for()),
        &product,
        references,
        &checks(),
    )
}

/// A scope note in the shape of the one that was misquoted: it names the wiring
/// explicitly, which is what made the invented sentence a reversal of it.
fn scope_note() -> StepEvidence {
    StepEvidence {
        evidence_type: config::EvidenceType::FactsNote,
        claimed: "the jobs list gains an action removing every terminal job, \
                  wired through the routes and the daemon"
            .to_string(),
        shown_by: "the operation is declared and the handler answers it".to_string(),
        not_claimed: "nothing about a job that is still running".to_string(),
    }
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
    // The whole of rule 2, as an assertion: the submission exists, the step is
    // a `diff` step, and not one word of what the Drone said about its own work
    // is in the question.
    for said in [
        "I am confident it is right",
        "cargo test -p log",
        "I did not check the callers",
    ] {
        assert!(
            !question.contains(said),
            "{said} reached the Judge: {question}"
        );
    }
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

/// The other half of the fabricated quotation, and the half a check cannot
/// hold. A call told to name something in the work will quote in order to do
/// it, so what a quotation mark commits it to is stated — and the consequence
/// is stated as discarded rather than as either verdict, because a model told
/// that a bad quotation passes the work has been given a lever.
#[test]
fn the_answer_format_says_what_putting_words_in_quotation_marks_commits_to() {
    let workflow = workflow();
    let question = brief(&workflow).question().to_string();
    assert!(
        question.contains("appear, exactly as written, in the material above"),
        "{question}"
    );
    assert!(
        question.contains("neither passed nor refused"),
        "a model told the consequence is a pass would have a lever: {question}"
    );
}

/// The half the first version of that instruction left out, and what two Jobs
/// stopped on. Both refusals were honest: one quoted its own expectation, the
/// other quoted a faithful restatement of what it had been shown. Neither
/// invented anything, and a call told only that a quotation must be verbatim —
/// while also being told to name something in the work above — reads the two
/// together as an instruction to quote.
///
/// So the escape is stated, and stated as sufficient: a line in the Judge's own
/// words, unquoted, is a whole answer. What is asserted here is the wording of
/// the instruction and nothing about the verdict, because the containment check
/// under it is unchanged.
#[test]
fn the_answer_format_says_a_line_in_its_own_words_needs_no_quotation_marks() {
    let workflow = workflow();
    let question = brief(&workflow).question().to_string();
    assert!(
        question.contains("write the line with no quotation marks at all"),
        "{question}"
    );
    assert!(
        question.contains("a complete answer and not a lesser one"),
        "a call that reads an unquoted line as the weaker answer will quote: {question}"
    );
    assert!(
        question.contains("words you assembled or reworded"),
        "a restatement inside quotation marks is the case both Jobs hit: {question}"
    );
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

/// Rule 4, one turn further on, and the Job it was written for. A refusal
/// quoted a sentence attributed to the Drone's scope note that was in no note,
/// no submission, no evidence row and no file, and the step was refused on it.
/// An invented citation is not a stricter reading — it is one nobody outside
/// the call can check, so this is a call that failed and not a verdict.
#[test]
fn a_refusal_quoting_words_the_call_was_never_shown_is_not_a_refusal() {
    let workflow = workflow();
    let note = scope_note();
    let brief = brief_measured_against(&workflow, &[Reference::to("scope", &note)]);
    // The real note is in front of it, and the quotation is not in the note.
    assert!(brief
        .question()
        .contains("wired through the routes and the daemon"));
    let read = brief.read(
        "verdict: not_met\n\
         expected: backend only, as scope notes \"Implementation, tests, and the \
         IPC/UI wiring itself are not done — that is parts…\"\n\
         produced: the change wires the frontend as well\n\
         consequence: the step lands work the note never described",
    );
    let Err(Unreadable::RefusalQuotesWhatIsNotThere { span }) = read else {
        panic!("an invented quotation is not a verdict: {read:?}");
    };
    // It names the words it did not find, or the person is left re-reading the
    // answer to work out which of them were invented.
    assert!(span.starts_with("Implementation, tests"), "{span}");
    assert!(
        format!("{}", Unreadable::RefusalQuotesWhatIsNotThere { span }).contains("invented"),
        "this is what a person reads in place of the refusal"
    );
}

/// The same check, on the refusal it must not touch. Every quotation here is in
/// the material — the note and the diff — so the refusal stands.
#[test]
fn a_refusal_quoting_what_it_was_actually_shown_still_refuses() {
    let workflow = workflow();
    let note = scope_note();
    let judged = brief_measured_against(&workflow, &[Reference::to("scope", &note)])
        .read(
            "verdict: not_met\n\
             expected: what the note called \"wired through the routes and the daemon\"\n\
             produced: a diff whose whole change is \"let n = n - 1;\"\n\
             consequence: whoever reads the note is promised wiring that is not there",
        )
        .expect("a refusal quoting the material");
    assert_eq!(judged.verdict, JudgeVerdict::NotMet);
}

/// **The two honest refusals this check stopped**, at the level where the cost
/// was paid: both were demoted to [`Unreadable`] and neither Job reached a
/// Check. Each quotes the Judge's own wording — a standard it wanted met, and a
/// paraphrase of the facts it had been shown — and neither attributes it to
/// anything, so there is no claim under the quotation marks for containment to
/// test. The spans are the ones the two Jobs actually produced.
#[test]
fn a_refusal_quoting_its_own_wording_rather_than_a_source_still_refuses() {
    let workflow = workflow();
    let note = scope_note();
    for answer in [
        "verdict: not_met\n\
         expected: \"A plan addressing all five variants as stated: NotResumable, \
         NoDroneToRedirect, DroneStillThere, WorktreeGone and NoStepStopped, each \
         currently answering 500 and requiring 409, with clear evidence they all \
         reach the catch-all\"\n\
         produced: the plan addresses three of them\n\
         consequence: two variants keep answering 500",
        "verdict: not_met\n\
         expected: every variant that falls through is mapped\n\
         produced: \"these three variants are unmapped in the match statement at \
         serving.rs and fall through to the catch-all 500 handler\"\n\
         consequence: a client is told Armada broke when it declined",
    ] {
        let judged = brief_measured_against(&workflow, &[Reference::to("scope", &note)])
            .read(answer)
            .expect("a refusal in the Judge's own words");
        assert_eq!(judged.verdict, JudgeVerdict::NotMet, "{answer}");
    }
}

/// A quotation short enough to be a term rather than a claim about wording is
/// left alone. Failing an honest refusal for the shape of its prose costs more
/// than the fabrications that would catch, and the invented one was fourteen
/// words long.
#[test]
fn a_quoted_word_or_two_is_a_term_and_is_not_checked() {
    let workflow = workflow();
    let judged = brief(&workflow)
        .read(
            "verdict: not_met\n\
             expected: the loop stops at \"n\", not before it\n\
             produced: it stops \"one early\"\n\
             consequence: the last row is dropped",
        )
        .expect("a refusal whose quotations are terms")
        .verdict;
    assert_eq!(judged, JudgeVerdict::NotMet);
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
