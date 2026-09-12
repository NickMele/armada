//! Where a Job is in its frozen workflow, and what it can reach from there.
//!
//! A cursor rather than a decision: nothing here rules on anything. It is a
//! separate type from [`Ruling`](crate::Ruling) because a position is what the
//! gate, the dispatch loop and the briefing all need, and only one of the three
//! decides anything.

use adapter_traits::Worktree;
use core_model::{Attempt, EvidenceRef, FrozenWorkflow, ResolvedStep, Spent, StepEvidence, StepId};

/// Where a Job is: which step of its frozen workflow, and the worktree the work
/// is in.
///
/// **No constructor takes a step index** — a position comes from a step id the workflow
/// actually declares, so a gate cannot point at a step outside the definition the Job froze.
///
/// **"Which step" alone does not locate a Job** — a step can be worked more than once.
/// [`Attempt`] is the coordinate `store::attempt` files each run under; [`Spent`] is what the
/// retry budget is asked against, resetting where an attempt climbs across a return. Both are
/// carried so the gate cannot reach for whichever is nearer — the types make taking the wrong
/// one a compile error.
///
/// **No constructor invents a number** — all three answer the first of each, the only value a
/// position with no history could have; [`on_attempt`](AtStep::on_attempt) says otherwise for a
/// caller who read the log.
#[derive(Clone, Copy, Debug)]
pub struct AtStep<'a> {
    workflow: &'a FrozenWorkflow,
    at: usize,
    worktree: &'a Worktree,
    attempt: Attempt,
    spent: Spent,
}

impl<'a> AtStep<'a> {
    /// The first step, where a Job starts. `None` for a workflow with no steps.
    pub fn first(workflow: &'a FrozenWorkflow, worktree: &'a Worktree) -> Option<AtStep<'a>> {
        (!workflow.steps().is_empty()).then_some(AtStep {
            workflow,
            at: 0,
            worktree,
            attempt: Attempt::FIRST,
            spent: Spent::FIRST,
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
            spent: Spent::FIRST,
        })
    }

    /// The same position, on the run the step's log says it is on and on the
    /// runs the current pass has spent.
    ///
    /// **Both together**, because reading one without the other is the bug this
    /// pair exists to stop — and because the two callers that have read the log
    /// have read it once for each.
    pub fn on_attempt(self, attempt: Attempt, spent: Spent) -> AtStep<'a> {
        AtStep {
            attempt,
            spent,
            ..self
        }
    }

    /// Which run of this step is being ruled on. One-based. **The coordinate
    /// every per-run record is filed under**, and never the retry budget.
    pub fn attempt(&self) -> Attempt {
        self.attempt
    }

    /// How many runs the pass this step is on has spent. One-based. **The
    /// number `ResolvedStep::may_hand_back` is asked against**, and never the
    /// attempt.
    pub fn spent(&self) -> Spent {
        self.spent
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
            // A different step is on its own first run, and on the first run of
            // its own first pass. Carrying either count forward would file the
            // next step's records under a run it has not had, or charge it a
            // budget it has not spent.
            attempt: Attempt::FIRST,
            spent: Spent::FIRST,
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
