//! What the call was shown, and what it was not.
//!
//! Every case here reads the question the fake was handed rather than the
//! verdict that came back. What has to be in it is the step's own work product
//! and the facts around it; what has to be absent is the Drone's own account of
//! the work, which is constitutional rule 2 — a verifier reading the
//! defendant's testimony is not independent.
//!
//! The last of them are #153's: a step whose work product is a note is judged
//! against the note, and a later step is judged against what an earlier one
//! established. Both go through Fleet's real runner, because both were built at
//! the type level and the thing that shipped broken was the wiring.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Footprint, Model};
use config::ResolvedWorkflow;
use core_model::{DeclaredPaths, RepoPath, StepId};
use testkit::{FakeJudge, FakeWorkProduct, Gate, Scoped, Sketch};

use verification::{Lifted, Request};

use crate::asked::Asked;
use crate::at_step::AtStep;
use crate::gate::rule_on;
use crate::judging::{JudgeBudget, Judging, Marking};
use crate::policy::Policies;
use crate::tests::gate::{budget, diff_evidence, judged_by_shared, note_evidence, worktree};
use crate::tests::judging::{judged_workflow, THE_QUESTION};
use crate::tests::keeping::keeping_nowhere;

#[tokio::test]
async fn the_call_carries_the_patch_and_the_facts_and_nothing_the_drone_wrote() {
    let workflow = judged_workflow();
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n");
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = Judging {
        client: Arc::clone(&judge) as Arc<_>,
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
        marking: Marking::detached(),
        asked: Asked::nowhere(),
    };

    rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging,
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
    )
    .await;

    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one criterion is one call");
    let question = &asked[0];
    assert!(question.contains("let n = n - 1"), "{question}");
    assert!(question.contains(THE_QUESTION), "{question}");
    // The submission's own words. Constitutional rule 2: a verifier that reads
    // the defendant's testimony is not independent.
    assert!(!question.contains("The loop is a fold."), "{question}");
    assert!(!question.contains("34 passing"), "{question}");
}

// ---------------------------------------------- a work product that is a note

/// Feature's opening pair, reduced: a step that writes a scope note and is
/// judged on it, then a step whose diff is measured against that note.
fn writing_workflow() -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "scope",
            label: "Scope the change",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[(
                "names_what_it_will_touch",
                "Does this scope note name the specific files it will touch?",
            )],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[(
                "implements_the_scope",
                "Does this diff implement the scope note?",
            )],
            scope: Some(Scoped {
                diff_check: false,
                at_step_start: false,
                exclude: &[],
                references: &["scope.evidence"],
            }),
            gaming: None,
        },
    ])
}

/// **The defect this issue names, end to end.** A `facts_note` step has no
/// diff, and the Judge used to be handed an empty one and refuse — every time,
/// on the first gated step of four workflows. The note is what the step
/// produced, so the note is what the call carries.
#[tokio::test]
async fn a_step_whose_work_product_is_a_note_is_judged_against_the_note() {
    let workflow = writing_workflow();
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    // Nothing on disk. That is the whole shape of the failure: the step is
    // finished and the worktree has not moved.
    let work = FakeWorkProduct::untouched();
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = judged_by_shared(Arc::clone(&judge));

    let ruling = rule_on(
        at,
        Request::of(testkit::asked_for()),
        &note_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging,
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
    )
    .await;

    assert!(
        ruling.advanced(),
        "a note the Judge did not refuse advances the step: {ruling:?}"
    );
    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one criterion is one call");
    let question = &asked[0];
    assert!(
        question.contains("The path is derived from the repo name."),
        "the note is the work product and did not reach the call: {question}"
    );
    assert!(question.contains("worktree.rs:40"), "{question}");
    assert!(
        !question.contains("The change, as a diff"),
        "there is no diff on this step: {question}"
    );
    assert_eq!(ruling.judged().len(), 1, "the record says the Judge ran");
}

/// The other half of the same rule, through the runner. A step whose product is
/// the change gets the diff, and its submission — prose about work the diff
/// already shows — reaches nothing.
#[tokio::test]
async fn a_later_step_is_measured_against_what_an_earlier_one_established() {
    let workflow = writing_workflow();
    let worktree = worktree();
    let at = AtStep::named(workflow.frozen(), &StepId::new("implement"), &worktree)
        .expect("the second step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n");
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = judged_by_shared(Arc::clone(&judge));
    let recorded = vec![(StepId::new("scope"), note_evidence().recorded())];
    // The step's scope asks the Drone where its work will be, so it declares.
    // That is unrelated to the yardstick and is what the block's other key is
    // for — `context_paths` and `reference_docs` stay separate on purpose.
    let declared = DeclaredPaths::of(vec![RepoPath::new("src")]);

    let ruling = rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        Some(&declared),
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &recorded,
        &work,
        budget(),
        &judging,
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    let question = &judge.asked()[0];
    assert!(
        question.contains("`scope` established: The path is derived from the repo name."),
        "the yardstick the step names did not reach the call: {question}"
    );
    assert!(
        question.contains("is not itself under judgment"),
        "the yardstick has to be told apart from the target: {question}"
    );
    assert!(question.contains("let n = n - 1"), "{question}");
    // Rule 2, unchanged: this step's product is the change, so this step's own
    // words are a claim about it and stay out.
    assert!(!question.contains("The loop is a fold."), "{question}");
}

/// **A guaranteed refusal is now a call that was never made.** A step whose
/// product is the change, with nothing changed, used to reach the Judge with an
/// empty patch. It decides neither way instead, which is what a person needs to
/// see.
#[tokio::test]
async fn a_step_with_nothing_to_show_costs_no_call_and_draws_no_verdict() {
    let workflow = writing_workflow();
    let worktree = worktree();
    let at = AtStep::named(workflow.frozen(), &StepId::new("implement"), &worktree)
        .expect("the second step");
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = judged_by_shared(Arc::clone(&judge));
    let declared = DeclaredPaths::of(vec![RepoPath::new("src")]);
    // The step declares `diff_nonempty`, so the mechanical tier stops it first
    // and the Judge is never reached — which is the cheaper of the two guards
    // and the one that fires here.
    let ruling = rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        Some(&declared),
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::untouched(),
        budget(),
        &judging,
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
    )
    .await;

    assert!(!ruling.advanced(), "{ruling:?}");
    assert!(
        judge.asked().is_empty(),
        "a call was bought against an empty page"
    );
}

/// **The request reaches the model, not just the brief.** `verification` proves
/// `Brief::about` renders it; this proves the whole path — a Job row, `rule_on`,
/// Fleet's own process runner — puts it in front of the thing that answers.
///
/// The criterion is Feature's designed `scope` wording, restored on #169. It is
/// unanswerable unless the request is in the same question, which is why it was
/// narrowed the first time the workflows were ported.
#[tokio::test]
async fn a_criterion_asking_what_was_requested_reaches_a_call_that_carries_it() {
    let workflow = testkit::resolved(&[Sketch {
        id: "scope",
        label: "Scope the change",
        evidence_type: Some("facts_note"),
        gates: &[],
        judged_on: &[(
            "scope",
            "Does this scope note address what was actually requested, without \
             expanding beyond it?",
        )],
        scope: None,
        gaming: None,
    }]);
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    // A real record, not the shared fixture: the three parts of this request
    // are what the assertions below look for, one each.
    let asked_for = testkit::asking(
        "The log reader drops the last line",
        "`read_all` stops one row short with no trailing newline.",
        &["the last line is returned"],
    );
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = judged_by_shared(Arc::clone(&judge));

    let ruling = rule_on(
        at,
        Request::of(&asked_for),
        &note_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::untouched(),
        budget(),
        &judging,
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    let question = &judge.asked()[0];
    for part in [
        "The log reader drops the last line",
        "stops one row short",
        "the last line is returned",
    ] {
        assert!(
            question.contains(part),
            "the Judge was asked about the request and not shown it: {question}"
        );
    }
}
