//! The first turn a Drone is given, assembled from the Job it is being put on.
//!
//! # The baseline is quoted, not written here
//!
//! `docs/contracts/agent-prompt.md` section 5 carries the M1 rendering as
//! sanctioned copy, and the three clauses it contains each passed a membership
//! test this module is in no position to re-run. So [`BASELINE`] is that text,
//! transcribed, and the rest of this module assembles the per-Job blocks around
//! it. **A wording change belongs in the contract**, and a copy here that
//! drifted from it would be the second-vocabulary defect in prose.
//!
//! # Four blocks of six layers, and the two that are missing are missing
//!
//! The contract names six layers: baseline, Kit, Manifest, WorkflowDef framing,
//! the Job brief, and the step. **M1 has no Kit and no Manifest layer**, which
//! the contract's own M1 rendering says in as many words, so what is assembled
//! here is baseline, job brief, where-you-are, step — the four blocks the
//! rendering shows.
//!
//! What is not assembled: the exemplar corpus, `context_paths`, the injected
//! reference material a review step needs, and what the previous step produced.
//! Each needs a record M1 has no type for, and a block rendered empty reads to a
//! Drone as a block that was answered.
//!
//! # A Drone is never told what the Checks are
//!
//! `docs/concepts/drone.md` governs every Drone-facing surface, and nothing in
//! this module has access to a `ResolvedCheck`'s `run` string by accident: the
//! step block is written from [`ResolvedStep::label`] and the Job's own fields.
//! Telling a Drone the Check would let it satisfy the Check rather than do the
//! work, which is the failure the whole gate exists to refuse.

use adapter_traits::{Prompt, SpawnConfigRefused};
use core_model::{FrozenWorkflow, Job, ResolvedStep, StepId};

/// Layer 1, verbatim from the Agent Prompt Contract's M1 rendering.
///
/// **Mechanics, never task content**, and identical on every step of every Job
/// — which is what makes it a constant rather than something assembled.
pub const BASELINE: &str = "\
You are working in a git worktree on a branch of your own. You cannot push, \
open a pull request, or run commands this repository has not declared.

When you have finished the work described below, you must report it using the \
evidence submission tool you have been given. It is the only way to report. \
Work you do not submit is work no one sees, and the task will not move on.

Submitting returns \"recorded\". That is a receipt, not a verdict — your work \
is checked after you submit. If it does not pass you will be told in a later \
turn, with the reason. Wait for that turn.";

/// Assemble the first turn for a Job standing at one step of its workflow.
///
/// **There is no argument through which arbitrary text reaches a Drone.** The
/// blocks are built from the Job record and the resolved workflow, and a caller
/// that wanted to say something else would have to add a block here — which is
/// the same refusal `Turn` makes about a prepared string.
///
/// Refuses only where the assembled text is empty, which
/// [`Prompt::assembled`] decides and this does not restate.
pub fn first_turn(
    job: &Job,
    workflow: &FrozenWorkflow,
    at: &StepId,
) -> Result<Prompt, SpawnConfigRefused> {
    Prompt::assembled(&assemble(job, workflow, at))
}

fn assemble(job: &Job, workflow: &FrozenWorkflow, at: &StepId) -> String {
    let mut text = String::from(BASELINE);
    text.push_str("\n\n");
    text.push_str(&job_brief(job));
    text.push_str("\n\n");
    text.push_str(&where_you_are(workflow, at));
    if let Some(step) = workflow.steps().iter().find(|step| step.id() == at) {
        text.push_str("\n\n");
        text.push_str(&step_block(step));
    }
    text
}

/// What the Job is about, in the requester's own words.
///
/// `facts` is the context the Job carries and `acceptance_criteria` is what the
/// requester said "done" means. Both are the requester's text and both go in:
/// the criteria in particular are layer 5 in the contract's order, and a Drone
/// that cannot see them is being asked to hit a bar it was not shown.
fn job_brief(job: &Job) -> String {
    let mut brief = format!("JOB BRIEF\n\n{}", job.title().as_str());
    if !job.facts().as_str().is_empty() {
        brief.push_str("\n\n");
        brief.push_str(job.facts().as_str());
    }
    if !job.acceptance_criteria().is_empty() {
        brief.push_str("\n\nThis is done when:");
        for criterion in job.acceptance_criteria() {
            brief.push_str("\n  - ");
            brief.push_str(&criterion.text);
        }
    }
    brief
}

/// The rail, with the stop inside it.
///
/// **"Parts", not "steps"** — the contract is explicit that `step` is Armada's
/// word for a plan artifact and that a Drone which learns the system's
/// vocabulary can reason about the machinery.
///
/// **The stop sits inside the list rather than after it**, because where the
/// line falls is the boundary, and later parts carry the specific prohibition
/// rather than a general one.
fn where_you_are(workflow: &FrozenWorkflow, at: &StepId) -> String {
    let steps = workflow.steps();
    let position = steps.iter().position(|step| step.id() == at);
    let mut block = format!("WHERE YOU ARE\n\nThis task runs in {} parts.", steps.len());
    if let Some(index) = position {
        block.push_str(&format!(" You are on part {}.\n", index + 1));
    } else {
        block.push('\n');
    }
    for (index, step) in steps.iter().enumerate() {
        let mark = match position {
            Some(here) if index < here => "done",
            Some(here) if index == here => "you are here",
            _ => "not yours — do not do it",
        };
        block.push_str(&format!("\n  {}. {} — {mark}", index + 1, step.label()));
        if position == Some(index) {
            block.push_str("\n     STOP. Submit when this part is done, then wait.");
        }
    }
    block.push_str(
        "\n\nThe parts after this one happen after you submit, and doing them \
         yourself does not move this task forward. Leave the branch in a state \
         they can start from.",
    );
    block
}

/// The step itself, and the one instruction that is the same on every step:
/// what to claim.
///
/// The closing line is where a work submission's `not_claimed` field comes
/// from — an adjacent problem noticed and left alone has somewhere to land.
fn step_block(step: &ResolvedStep) -> String {
    format!(
        "STEP: {}\n\nWhat you claim should be what the work now does, not that \
         you finished. An adjacent problem you notice and leave alone goes \
         under Not claimed.",
        step.label()
    )
}
