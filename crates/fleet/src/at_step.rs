//! Where a Job is in its frozen workflow, and what it can reach from there.
//!
//! A cursor rather than a decision: nothing here rules on anything. It is a
//! separate type from [`Ruling`](crate::Ruling) because a position is what the
//! gate, the dispatch loop and the briefing all need, and only one of the three
//! decides anything.

use adapter_traits::Worktree;
use core_model::{Attempt, EvidenceRef, FrozenWorkflow, ResolvedStep, StepEvidence, StepId};

/// Where a Job is: which step of its frozen workflow, and the worktree the work
/// is in.
///
/// **There is no constructor taking a step index.** A position comes from a
/// step id the workflow actually declares, so a gate cannot be pointed at a
/// step that is not in the definition the Job froze.
///
/// # Which run of the step is part of where the Job is
///
/// A step can be worked more than once, so "which step" does not locate a Job
/// on its own. [`Attempt`] is the third coordinate and it is the same one
/// `store::attempt` files every per-run record under — derived from the step's
/// own log, with no constructor that invents a number. A caller cannot tell
/// this type it is on the fourth run when the log says the second.
///
/// Both constructors answer [`Attempt::FIRST`], which is not a default so much
/// as the only value a position with no history could have — `Attempt`'s own
/// rule. [`on_attempt`](AtStep::on_attempt) is how the one caller that has read
/// the log says so.
#[derive(Clone, Copy, Debug)]
pub struct AtStep<'a> {
    workflow: &'a FrozenWorkflow,
    at: usize,
    worktree: &'a Worktree,
    attempt: Attempt,
}

impl<'a> AtStep<'a> {
    /// The first step, where a Job starts. `None` for a workflow with no steps.
    pub fn first(workflow: &'a FrozenWorkflow, worktree: &'a Worktree) -> Option<AtStep<'a>> {
        (!workflow.steps().is_empty()).then_some(AtStep {
            workflow,
            at: 0,
            worktree,
            attempt: Attempt::FIRST,
        })
    }

    /// A named step. `None` where the workflow declares no step by that id.
    pub fn named(
        workflow: &'a FrozenWorkflow,
        step: &StepId,
        worktree: &'a Worktree,
    ) -> Option<AtStep<'a>> {
        let at = workflow.steps().iter().position(|s| s.id() == step)?;
        Some(AtStep {
            workflow,
            at,
            worktree,
            attempt: Attempt::FIRST,
        })
    }

    /// The same position, on the run the step's log says it is on.
    ///
    /// One caller: `crate::settling`, which reads it off the store inside the
    /// same turn it rules. Everything else is standing at a step for the first
    /// time and says nothing.
    pub fn on_attempt(self, attempt: Attempt) -> AtStep<'a> {
        AtStep { attempt, ..self }
    }

    /// Which run of this step is being ruled on. One-based.
    pub fn attempt(&self) -> Attempt {
        self.attempt
    }

    /// The step being gated.
    pub fn step(&self) -> &'a ResolvedStep {
        &self.workflow.steps()[self.at]
    }

    /// The step after it, or `None` at the last one.
    pub fn next(&self) -> Option<&'a ResolvedStep> {
        self.workflow.steps().get(self.at + 1)
    }

    /// Where the Job is once this step has advanced. `None` at the last step,
    /// which is the workflow being finished rather than a position.
    pub fn advanced(&self) -> Option<AtStep<'a>> {
        self.next().map(|_| AtStep {
            workflow: self.workflow,
            at: self.at + 1,
            worktree: self.worktree,
            // A different step is on its own first run. Carrying this one's
            // count forward would file the next step's records under a run it
            // has not had.
            attempt: Attempt::FIRST,
        })
    }

    pub fn worktree(&self) -> &'a Worktree {
        self.worktree
    }

    /// The evidence a `baseline_ref` or a `reference_docs` entry names, **and
    /// only where it names a step strictly earlier than this one**.
    ///
    /// Both keys are spelled `<step_id>.evidence` through the same
    /// [`EvidenceRef`], and both resolve here, so the gaming baseline and the
    /// Judge's yardstick cannot come to disagree about which steps are
    /// reachable from where.
    ///
    /// A reference forward, or at this step, answers `None`: a baseline that
    /// has not happened yet is not a baseline, and a step comparing against
    /// itself is comparing against nothing. That is the whole check — there is
    /// no way to reach a later step's evidence through this type.
    pub fn baseline<'e>(
        &self,
        reference: &EvidenceRef,
        recorded: &'e [(StepId, StepEvidence)],
    ) -> Option<(&'a StepId, &'e StepEvidence)> {
        let named = self
            .workflow
            .steps()
            .iter()
            .position(|step| step.id() == reference.step())
            .filter(|position| *position < self.at)?;
        let id = self.workflow.steps()[named].id();
        let evidence = recorded
            .iter()
            .find(|(step, _)| step == id)
            .map(|(_, evidence)| evidence)?;
        Some((id, evidence))
    }
}
