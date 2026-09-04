//! What the scope look is told, and what it may answer.
//!
//! Two properties, both held by every other Judge call here and neither of them
//! free on this one: the brief carries what the decision is about and nothing
//! about how the step has been going, and an answer that establishes nothing is
//! an error rather than a decision.

use config::ResolvedWorkflow;
use core_model::{Job, RepoPath, WriteTargets};
use testkit::Sketch;

use crate::{NotWidened, Request, Unreadable, Widened, WideningBrief};

fn workflow() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "fix",
        label: "Fix the reader",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn asked() -> Job {
    testkit::asking(
        "The log reader drops the last line",
        "`read_all` stops one row short when the file has no trailing newline.",
        &["the last line of a file with no trailing newline is returned"],
    )
}

fn held() -> WriteTargets {
    WriteTargets::of(vec![RepoPath::new("crates/fleet/src")])
}

fn brief(job: &Job, workflow: &ResolvedWorkflow) -> WideningBrief {
    WideningBrief::about(
        &workflow.steps()[0],
        Request::of(job),
        &held(),
        &[RepoPath::new("crates/store/src/schema.rs")],
        "the column the fix needs is declared here",
    )
}

/// The four things the decision named, and nothing else. The reason is the
/// asker's own words and is labelled as an argument.
///
/// **That the diff is absent is a property of the signature and not of this
/// assertion.** [`WideningBrief::about`] has no parameter for a patch, a
/// transcript or a count, so how the step has been going cannot reach the call
/// that decides whether the request makes sense — there is nothing a test could
/// pass in to check for.
#[test]
fn the_brief_carries_the_request_the_scope_the_paths_and_the_reason() {
    let job = asked();
    let workflow = workflow();
    let asked = brief(&job, &workflow).question().to_string();
    assert!(
        asked.contains("The log reader drops the last line"),
        "{asked}"
    );
    assert!(asked.contains("Fix the reader"), "{asked}");
    assert!(asked.contains("crates/fleet/src"), "{asked}");
    assert!(asked.contains("crates/store/src/schema.rs"), "{asked}");
    assert!(
        asked.contains("the column the fix needs is declared here"),
        "{asked}"
    );
    assert!(asked.contains("own words"), "the reason is attributed");
}

/// The question it asks and the three it refuses to ask. A look that decided
/// desirability would be the person's decision made by a model, which is the
/// one thing this must not become.
#[test]
fn the_question_is_consistency_and_says_what_it_is_not() {
    let job = asked();
    let workflow = workflow();
    let asked = brief(&job, &workflow).question().to_string();
    assert!(asked.contains("part of the step"), "{asked}");
    assert!(asked.contains("not"), "{asked}");
    assert!(asked.contains("a good idea"), "{asked}");
    assert!(asked.contains("allowed to write"), "{asked}");
}

#[test]
fn a_declaration_of_nothing_reads_as_nothing_rather_than_as_absence() {
    let job = asked();
    let workflow = workflow();
    let brief = WideningBrief::about(
        &workflow.steps()[0],
        Request::of(&job),
        &WriteTargets::nothing(),
        &[RepoPath::new("crates/store/src/schema.rs")],
        "the column the fix needs is declared here",
    );
    assert!(brief.question().contains("change nothing"), "{brief:?}");
}

#[test]
fn both_answers_read_back_as_themselves() {
    let job = asked();
    let workflow = workflow();
    let brief = brief(&job, &workflow);
    assert_eq!(brief.read("answer: consistent"), Ok(Widened::Consistent));
    assert_eq!(
        brief.read("answer: inconsistent\nbecause: the schema is step four's"),
        Ok(Widened::Inconsistent(NotWidened::because(
            "the schema is step four's"
        )))
    );
}

/// An answer that establishes nothing is an error, in both of its shapes. A
/// chain that read either as a decision would escalate a Job on a parse
/// failure, or would widen a Job's scope because a model wrote a paragraph.
#[test]
fn an_answer_that_establishes_nothing_is_not_a_decision() {
    let job = asked();
    let workflow = workflow();
    let brief = brief(&job, &workflow);
    assert_eq!(
        brief.read("it seems reasonable to me"),
        Err(Unreadable::NoScopeAnswer)
    );
    assert_eq!(brief.read("answer: maybe"), Err(Unreadable::NoScopeAnswer));
    assert_eq!(
        brief.read("answer: inconsistent"),
        Err(Unreadable::ScopeAnswerSaysNoWhy)
    );
}
