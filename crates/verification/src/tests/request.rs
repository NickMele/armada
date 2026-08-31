//! Rule 2's other half: the Judge receives the original task text.
//!
//! Every case here is written against the two criteria that were dropped for
//! wanting it — Feature's *does this scope note address what was actually
//! requested* and Bug's *does this plan address what was actually asked*. Both
//! compare a document to the request, and both were unanswerable while a brief
//! could only show the document.

use adapter_traits::Patch;
use config::ResolvedWorkflow;
use core_model::Job;
use testkit::Sketch;

use crate::{Accepted, Brief, Claimed, NotClaimed, Product, Request, ShownBy, Submission};

/// Feature's opening step, reduced: a scope note, judged on whether it answers
/// the request.
fn workflow() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "scope",
        label: "Scope the work",
        evidence_type: Some("facts_note"),
        gates: &[],
        judged_on: &[(
            "scope",
            "Does this scope note address what was actually requested, without \
             expanding beyond it?",
        )],
        scope: None,
        gaming: None,
    }])
}

/// A Job whose request is distinctive in all three of its parts, so an
/// assertion can tell which one reached the brief.
fn asked() -> Job {
    testkit::asking(
        "The log reader drops the last line",
        "`read_all` stops one row short when the file has no trailing newline.",
        &[
            "the last line of a file with no trailing newline is returned",
            "no existing caller changes behaviour",
        ],
    )
}

fn note() -> Submission {
    Submission::submitted(
        config::EvidenceType::FactsNote,
        Claimed("The change is confined to the cursor bound in `read_all`"),
        ShownBy("`crates/store/src/read.rs`"),
        NotClaimed("nothing about the writer"),
    )
    .expect("a submission")
}

fn brief_of(workflow: &ResolvedWorkflow, job: &Job) -> String {
    let step = &workflow.steps()[0];
    let submitted = note();
    let patch = Patch::of(String::new());
    let accepted = Accepted::of(step, &submitted).expect("the step asks for a note");
    let product = Product::of(step, &patch, accepted, None).expect("the note is the work product");
    Brief::about(
        step,
        &step.judge_checks()[0].criteria()[0],
        Request::of(job),
        &product,
        &[],
        &[],
    )
    .question()
    .to_string()
}

/// **The defect this module exists for.** The criterion in the fixture is the
/// designed `feature/scope` wording, restored. It asks about the request, so
/// the request has to be in the same question — all three parts of it, because
/// the title alone is a headline and the criteria alone are a checklist.
#[test]
fn a_criterion_asking_about_the_request_is_shown_the_request() {
    let workflow = workflow();
    let job = asked();
    let question = brief_of(&workflow, &job);

    assert!(
        question.contains("Does this scope note address what was actually requested"),
        "{question}"
    );
    assert!(
        question.contains("The log reader drops the last line"),
        "the title never reached the Judge: {question}"
    );
    assert!(
        question.contains("stops one row short"),
        "the requester's facts never reached the Judge: {question}"
    );
    assert!(
        question.contains("no existing caller changes behaviour"),
        "the acceptance criteria never reached the Judge: {question}"
    );
}

/// The request is a standard and the note is the thing being measured, and a
/// Judge that cannot tell them apart refuses a first step for not having
/// finished the Job. Both are labelled, and the request is labelled twice —
/// once as the requester's rather than anyone else's, and once as the *Job's*
/// bar rather than this step's, because no criterion here names which
/// acceptance criterion it tests.
#[test]
fn the_request_is_labelled_as_the_standard_and_as_the_whole_jobs_bar() {
    let workflow = workflow();
    let job = asked();
    let question = brief_of(&workflow, &job);

    assert!(
        question.contains("in the requester's own words"),
        "{question}"
    );
    assert!(
        question.contains("measured against rather than something under judgment"),
        "{question}"
    );
    assert!(
        question.contains("not this step's bar on its own"),
        "a first step judged against the whole Job's criteria refuses every \
         time: {question}"
    );
}

/// The request comes before the work, for the reason `Reference::all` comes
/// before the product: it is the standard everything below it is read against,
/// including an earlier step's evidence.
#[test]
fn the_request_is_read_before_anything_it_is_the_standard_for() {
    let workflow = workflow();
    let job = asked();
    let question = brief_of(&workflow, &job);

    let request = question
        .find("The log reader drops the last line")
        .expect("the request");
    let product = question
        .find("What this step produced")
        .expect("the work product");
    assert!(request < product, "{question}");
}

/// A Job whose requester wrote no facts and set no criteria. The title is a
/// `Title` and cannot be blank, so there is still a request — and the brief
/// carries no empty headings for the two that are absent.
#[test]
fn a_request_with_nothing_but_a_title_still_reaches_the_brief() {
    let workflow = workflow();
    let job = testkit::asking("Make the reader read the last line", "", &[]);
    let question = brief_of(&workflow, &job);

    assert!(
        question.contains("Make the reader read the last line"),
        "{question}"
    );
    assert!(!question.contains("what they said about it"), "{question}");
    assert!(
        !question.contains("the whole Job is done when"),
        "{question}"
    );
}
