//! Which model a step's Drone is run as.
//!
//! Its own file rather than a section of [`super::record`], for the reason that
//! one is about what a transition writes down: this is a read, it has one
//! rule — the step's, or the Job's — and the rule is the whole subject.
//!
//! **It could not be asked before a step was its own process.** One session
//! spanned a whole Job and a session cannot change model partway, so the Job's
//! model was the answer at every step by construction and there was no fallback
//! to spell.

use super::*;

/// The fixture workflow with `fix` naming a model of its own and `repro`
/// naming none, which is the shape every shipped workflow has: the annotation
/// is on the step that wants something other than the Job's.
fn workflow_with_a_modelled_step() -> FrozenWorkflow {
    let steps: Vec<ResolvedStep> = workflow()
        .steps()
        .iter()
        .map(|step| {
            let model = match step.id().as_str() {
                "fix" => Some(ModelName::new("the-steps-own-model").expect("a model name")),
                _ => None,
            };
            ResolvedStep::frozen(
                step.id().clone(),
                step.label().to_string(),
                step.evidence_type(),
                step.checks().to_vec(),
                step.advance_gate(),
                step.judge_checks().to_vec(),
                step.evidence_scope().cloned(),
                step.retry_limit(),
                model,
            )
        })
        .collect();
    FrozenWorkflow::frozen(
        workflow().id().clone(),
        workflow().name().to_string(),
        workflow().version(),
        steps,
    )
}

fn job_whose_fix_step_names_a_model() -> Job {
    let mut draft = draft();
    draft.workflow = workflow_with_a_modelled_step();
    Job::create_top_level(
        draft,
        TopLevelOrigin::Manual,
        at("2026-08-26T09:00:00.000Z"),
    )
}

/// **A step that names none is run as the Job was proposed.** This is what
/// every step did while one process spanned a whole Job, and removing it would
/// make every workflow have to state a model on every step.
#[test]
fn a_step_naming_no_model_falls_back_to_the_jobs() {
    let job = job_whose_fix_step_names_a_model();
    assert_eq!(
        job.model_at(&StepId::new("repro")).as_str(),
        job.model().as_str()
    );
}

/// **A step that names one gets it**, which is the whole of what the dial does.
#[test]
fn a_step_naming_a_model_is_run_as_that_one() {
    let job = job_whose_fix_step_names_a_model();
    assert_eq!(
        job.model_at(&StepId::new("fix")).as_str(),
        "the-steps-own-model"
    );
    assert_ne!(job.model().as_str(), "the-steps-own-model");
}

/// A step id the workflow does not declare answers with the Job's rather than
/// panicking or inventing one. **Not a case to rely on**: a Drone is only ever
/// put on a step the frozen workflow names, and a caller that reached here has
/// a larger problem than which model it got.
#[test]
fn a_step_this_workflow_does_not_declare_answers_with_the_jobs() {
    let job = job_whose_fix_step_names_a_model();
    assert_eq!(
        job.model_at(&StepId::new("no-such-step")).as_str(),
        job.model().as_str()
    );
}
