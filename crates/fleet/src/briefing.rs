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
//! the contract's own M1 rendering says, so what is assembled here is baseline,
//! job brief, where-you-are, step — the four blocks the rendering shows.
//!
//! What is not assembled: the exemplar corpus, the injected reference material
//! a review step needs, and what the previous step produced. Each needs a
//! record M1 has no type for, and a block rendered empty reads to a Drone as a
//! block that was answered. The scope block is written **only** where the step
//! declares one — the only step where the call exists.
//!
//! # A Drone is never told what the Checks are
//!
//! `docs/concepts/drone.md` governs every Drone-facing surface, and nothing in
//! this module has access to a `ResolvedCheck`'s `run` string by accident: the
//! step block is written from [`ResolvedStep::label`] and the Job's own fields.
//! Telling a Drone the Check would let it satisfy the Check rather than do the
//! work, which is the failure the whole gate exists to refuse.

use adapter_traits::{Prompt, SpawnConfigRefused};
use core_model::{FrozenWorkflow, GamingFlag, Job, Judgment, ResolvedStep, StepId, StepVerdict};

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

/// Assemble the first turn for a Drone taking over a step that stopped.
///
/// **The reason is not optional and there is no constructor without one.** A
/// restarted Drone has no session and no history: it knows nothing about the
/// attempt it is replacing, and a brief that did not say what stopped would
/// send it to reproduce the work that was refused.
///
/// [`Stopped`] is read off the record rather than composed by a caller, so
/// nothing here is a claim about the Job that the log does not already carry.
pub fn resuming_turn(
    job: &Job,
    workflow: &FrozenWorkflow,
    at: &StepId,
    stopped: &Stopped,
) -> Result<Prompt, SpawnConfigRefused> {
    let mut text = assemble(job, workflow, at);
    text.push_str("\n\n");
    text.push_str(&stopped.block());
    Prompt::assembled(&text)
}

/// Why the step a Drone is being put on stopped, as the record holds it.
///
/// Built by `crate::resume` from `last_verdict`, `job_step_judgments` and
/// `job_step_gaming_flags` — the same three a person reads on the detail view.
#[derive(Clone, Debug, Default)]
pub struct Stopped {
    /// What the gate said stopped it, spelled as the registry spells it.
    pub verdict: Option<StepVerdict>,
    /// Every criterion the Judge answered on the step. Only the refused ones
    /// are rendered.
    pub judged: Vec<Judgment>,
    /// Every gaming pattern the step's evidence tripped.
    pub flagged: Vec<GamingFlag>,
}

impl Stopped {
    /// The block, in the shape the Agent Prompt Contract's refusal reprompt
    /// specifies: `expected` and `produced`, **never `consequence`**, which is
    /// written for a person deciding whether to care, and **never a counter**.
    ///
    /// The gaming half has no sanctioned wording and is drafted. It renders
    /// the pattern and what it cited, which is the same two-column shape and
    /// the whole of what a flag is.
    fn block(&self) -> String {
        let mut block = String::from(
            "WHY THIS PART IS BEING DONE AGAIN\n\nAn earlier attempt at this part was \
             checked and did not pass. Its work is on the branch you are in.",
        );
        for judgment in self.judged.iter().filter(|judged| judged.verdict.refuses()) {
            if let (Some(expected), Some(produced)) = (&judgment.expected, &judgment.produced) {
                block.push_str(&format!(
                    "\n\n  Expected   {expected}\n  Produced   {produced}"
                ));
            }
        }
        for flag in &self.flagged {
            block.push_str(&format!(
                "\n\n  Pattern    {}\n  Found in   {}",
                flag.pattern.as_wire(),
                flag.cited
            ));
        }
        block.push_str(
            "\n\nAddress this and submit again. Say what changed since the last \
             submission.",
        );
        block
    }
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
        if let Some(block) = scope_block(step) {
            text.push_str("\n\n");
            text.push_str(&block);
        }
    }
    text
}

/// What to declare before starting, where the step asks the Drone to.
///
/// **The obligation is here rather than in the tool's description**, for the
/// reason the baseline carries the Evidence obligation: spike 6 measured that a
/// description alone does not make a Drone call a tool.
///
/// The consequence is stated plainly and without a threat: a plan that turns
/// out wrong is fixed by declaring again, and work belonging to a later part
/// does not become this part's by being named.
fn scope_block(step: &ResolvedStep) -> Option<String> {
    let scope = step.evidence_scope()?;
    if !scope.wants_a_declaration() {
        return None;
    }
    let mut block = String::from(
        "BEFORE YOU START\n\nCall the scope tool with the repository-relative \
         paths this part's work will be in. Include what you will change and \
         what has to be read to judge the change.",
    );
    if scope.scope_diff_check() {
        block.push_str(
            " Files you change outside them are compared against what you \
             declared. If the work turns out to be somewhere else, call the tool \
             again — a plan that changed is fine, and a file changed for the \
             next part is not.",
        );
    }
    if !scope.exclude_paths().is_empty() {
        block.push_str("\n\nDo not name these, and do not change them:");
        for path in scope.exclude_paths() {
            block.push_str("\n  - ");
            block.push_str(path.as_str());
        }
    }
    Some(block)
}

/// What the Job is about, in the requester's own words.
///
/// `facts` is the context the Job carries and `acceptance_criteria` is what the
/// requester said "done" means. Both are the requester's text and both go in:
/// the criteria in particular are layer 5 in the contract's order, and a Drone
/// that cannot see them is being asked to hit a bar it was not shown.
///
/// Between the two, a line per attachment — a screenshot, a log capture,
/// whatever a person picked when the brief was written. Naming the
/// worktree-relative path is the whole of what this owes a Drone: `dispatch`
/// already copied the file to that path, and a Drone opens it with its own
/// tools rather than being handed anything more than where to look.
fn job_brief(job: &Job) -> String {
    let mut brief = format!("JOB BRIEF\n\n{}", job.title().as_str());
    if !job.facts().as_str().is_empty() {
        brief.push_str("\n\n");
        brief.push_str(job.facts().as_str());
    }
    if !job.attachments().is_empty() {
        brief.push_str("\n\nFiles attached to this brief, copied into your worktree:");
        for attachment in job.attachments() {
            brief.push_str("\n  - .armada/attachments/");
            brief.push_str(&attachment.filename);
        }
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
