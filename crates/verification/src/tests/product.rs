//! What the Judge is shown on a step whose work product is not a diff.
//!
//! The rule under every case here is **source, not shape**. A facts note
//! submitted as the step's artifact is a deliverable and the Judge reads it; the
//! same three fields on a `diff` step are a claim about work the diff already
//! shows, and the Judge does not. One `match`, in `Written::of`, is the whole
//! of it — so these cases are that match asserted from outside.

use adapter_traits::Patch;
use config::{EvidenceType, ResolvedWorkflow};
use core_model::StepEvidence;
use testkit::{Gate, Scoped, Sketch};

use crate::{
    Accepted, Brief, Claimed, NotClaimed, NothingToJudge, Product, Reference, ShownBy, Submission,
};

/// Feature's opening pair, reduced: a step that writes a scope note, and a step
/// whose diff is measured against it.
fn workflow() -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "scope",
            label: "Scope the change",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[(
                "addresses_the_request",
                "Does this scope note address what was actually requested?",
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
                "Does this diff implement what the scope note described?",
            )],
            scope: Some(Scoped {
                diff_check: false,
                at_step_start: false,
                exclude: &[],
                references: &["scope.evidence"],
            }),
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

fn note() -> Submission {
    Submission::submitted(
        EvidenceType::FactsNote,
        Claimed("The change is confined to the retry backoff in `dispatch.rs`"),
        ShownBy("`docs/scope-01M14.md`"),
        NotClaimed("Nothing is said about the queue's own timeout"),
    )
    .expect("a submission")
}

fn diff() -> Submission {
    Submission::submitted(
        EvidenceType::Diff,
        Claimed("Backoff doubles rather than resetting"),
        ShownBy("`cargo test -p dispatch` exit 0"),
        NotClaimed(""),
    )
    .expect("a submission")
}

const PATCH: &str = "--- a/src/dispatch.rs\n+++ b/src/dispatch.rs\n+    wait *= 2;\n";

/// **The defect this module exists for.** A `facts_note` step has no diff, and
/// the Judge used to be handed an empty one and refuse — four times over, on
/// the first gated step of four workflows. The note is what the step produced,
/// so the note is what it is shown.
#[test]
fn a_step_whose_work_product_is_a_note_is_judged_against_the_note() {
    let workflow = workflow();
    let step = &workflow.steps()[0];
    let submitted = note();
    let patch = Patch::of(String::new());
    let accepted = Accepted::of(step, &submitted).expect("the step asks for a note");
    let product = Product::of(step, &patch, accepted).expect("the note is the work product");

    let brief = Brief::about(
        step,
        &step.judge_checks()[0].criteria()[0],
        &product,
        &[],
        &[],
    );
    let question = brief.question();
    assert!(
        question.contains("The change is confined to the retry backoff"),
        "{question}"
    );
    assert!(question.contains("docs/scope-01M14.md"), "{question}");
    assert!(
        question.contains("Nothing is said about the queue's own timeout"),
        "{question}"
    );
    assert!(
        question.contains("Does this scope note address what was actually requested?"),
        "{question}"
    );
    assert!(
        !question.contains("The change, as a diff"),
        "there is no diff on this step and the brief must not pretend there is: {question}"
    );
}

/// The other half of the same rule. On a step whose product is the change, the
/// submission is prose *about* the change and there is no route from it into a
/// brief — `Written::of` answers `None`, and nothing else can build one.
#[test]
fn a_step_whose_work_product_is_the_change_shows_the_judge_no_submission() {
    let workflow = workflow();
    let step = &workflow.steps()[1];
    let submitted = diff();
    let patch = Patch::of(PATCH.to_string());
    let accepted = Accepted::of(step, &submitted).expect("the step asks for a diff");
    let product = Product::of(step, &patch, accepted).expect("the worktree moved");

    assert!(
        product.written().is_none(),
        "a diff step's submission is a claim about the work, not the work"
    );
    let brief = Brief::about(
        step,
        &step.judge_checks()[0].criteria()[0],
        &product,
        &[],
        &[],
    );
    assert!(brief.question().contains("wait *= 2"));
    assert!(!brief.question().contains("Backoff doubles rather than"));
}

/// **The yardstick, in the brief at last.** `implement`'s shipped criterion
/// asks whether the diff does what the scope note described, and until
/// `reference_docs` was read the Judge had never seen the scope note. It is
/// labelled as the standard rather than the target, which is the separation
/// `docs/concepts/judge.md` insists on.
#[test]
fn what_an_earlier_step_established_reaches_the_brief_labelled_as_the_yardstick() {
    let workflow = workflow();
    let step = &workflow.steps()[1];
    let submitted = diff();
    let patch = Patch::of(PATCH.to_string());
    let accepted = Accepted::of(step, &submitted).expect("the step asks for a diff");
    let product = Product::of(step, &patch, accepted).expect("the worktree moved");
    let earlier = StepEvidence {
        evidence_type: EvidenceType::FactsNote,
        claimed: "The change is confined to the retry backoff in `dispatch.rs`".to_string(),
        shown_by: "`docs/scope-01M14.md`".to_string(),
        not_claimed: String::new(),
    };

    let brief = Brief::about(
        step,
        &step.judge_checks()[0].criteria()[0],
        &product,
        &[Reference::to("scope", &earlier)],
        &[],
    );
    let question = brief.question();
    assert!(
        question.contains("`scope` established: The change is confined to the retry backoff"),
        "{question}"
    );
    assert!(
        question.contains("is not itself under judgment"),
        "the yardstick has to be told apart from the target: {question}"
    );
    assert!(question.contains("wait *= 2"), "{question}");
}

/// A step that declares no evidence type produces nothing a Judge could read —
/// bug's `merge` is the shipped example. **Not a refusal**: nothing was
/// verified and nothing failed, and the two are read by different people.
#[test]
fn a_step_that_produces_nothing_yields_no_product_rather_than_an_empty_one() {
    let workflow = workflow();
    let step = &workflow.steps()[2];
    let submitted = diff();
    let patch = Patch::of(PATCH.to_string());
    let accepted = Accepted::of(step, &submitted).expect("a step declaring none accepts anything");
    assert_eq!(
        Product::of(step, &patch, accepted),
        Err(NothingToJudge::StepProducesNothing)
    );
}

/// The `diff`-shaped mirror of the same guard. A step whose product is the
/// change, with nothing changed, is a call that cannot be made rather than one
/// that comes back refused.
#[test]
fn a_diff_step_with_an_empty_patch_is_a_call_that_cannot_be_made() {
    let workflow = workflow();
    let step = &workflow.steps()[1];
    let submitted = diff();
    let patch = Patch::of("   \n".to_string());
    let accepted = Accepted::of(step, &submitted).expect("the step asks for a diff");
    assert_eq!(
        Product::of(step, &patch, accepted),
        Err(NothingToJudge::NothingChanged {
            declared: EvidenceType::Diff
        })
    );
}

/// A written step whose worktree also moved carries both, and says which is
/// which. The mandatory drift look asks whether files this step changed were
/// its task to change, and that question is unanswerable without the change.
#[test]
fn a_written_step_that_also_changed_files_carries_both_and_labels_them() {
    let workflow = workflow();
    let step = &workflow.steps()[0];
    let submitted = note();
    let patch = Patch::of(PATCH.to_string());
    let accepted = Accepted::of(step, &submitted).expect("the step asks for a note");
    let product = Product::of(step, &patch, accepted).expect("the note is the work product");

    assert!(product.written().is_some());
    assert!(product.changed().is_some());
    let brief = Brief::about(
        step,
        &step.judge_checks()[0].criteria()[0],
        &product,
        &[],
        &[],
    );
    let question = brief.question();
    assert!(
        question.contains("The step also changed these files"),
        "the diff beside a deliverable is not the deliverable: {question}"
    );
    assert!(question.contains("wait *= 2"), "{question}");
}
