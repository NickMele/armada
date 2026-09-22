// What the classifying screen says, read out of the draft a moment carries.
//
// **The reading is here and the open state is `tab-proposal.tsx`**, which is
// how every other destination is arranged. What is unusual is that this one is
// also *editable*: nothing on this screen is on the wire — `approve_dispatch`
// takes no body and a proposal in flight is not a Job — so a person's changes
// live in the screen until the press, and these functions are the join between
// the draft's shapes and the control's own.

import type { CompleteChoice, ProposalCriterion, ProposalGateRow } from "@armada/components";
import type { ProposalLandingValue } from "@armada/components";
import type { JobDetail as JobWhole } from "@armada/protocol";

import { originSaidOf } from "./draft/criterion";
import type { CriterionView } from "./draft/criterion";
import type { JobDraft } from "./draft/held";
import { COMPLETE_WHEN_SERVED } from "./draft/landing";
import type { CompleteWhen, LandingRule } from "./draft/landing";
import { gateReadingOf, unmeantOf } from "./draft/proposal";
import type { GateView, ProposalView } from "./draft/proposal";
import { absoluteOf } from "./duration";

/**
 * Everything on the classifying screen a person may still move.
 *
 * **Held as the draft's own shapes, not the controls'.** These are what
 * `#1545` promotes onto the wire, so holding the edits in them is what makes
 * the screen's state the thing that will one day be sent — rather than a
 * second vocabulary that has to be translated back at the press.
 */
export type ProposalEdits = {
  proposal: ProposalView;
  landing: LandingRule;
  criteria: readonly CriterionView[];
};

/**
 * What the screen opens on, or `undefined` where this Job is not at a
 * proposal at all — which is every Job against a real Fleet today.
 */
export function proposalEditsOf(draft: JobDraft | undefined): ProposalEdits | undefined {
  if (draft?.proposal === undefined) return undefined;
  return {
    proposal: draft.proposal,
    // A Job whose Manifest has not been read lands in nothing and says so,
    // rather than drawing a branch name nobody chose.
    landing: draft.landing ?? {
      target: null,
      from_ref: draft.proposal.from_ref,
      prs: "job",
      branching: "job",
      pr_mode: draft.proposal.pr_mode,
      complete_when: "delivered",
      land_together: [],
    },
    criteria: draft.criteria ?? [],
  };
}

/**
 * One row per step, in the order the workflow runs them.
 *
 * **The label comes off the frozen step and the gate off the draft.** A step
 * the draft has a gate for and Fleet has no record of reads by its id: the
 * gate is still this Job's answer, and dropping the row would draw a workflow
 * with a step missing.
 */
export function gateRowsOf(
  gates: readonly GateView[],
  whole: JobWhole | null,
): ProposalGateRow[] {
  return gates.map((gate) => {
    const step = whole?.steps.find((one) => one.step_id === gate.step_id);
    const reading = gateReadingOf(gate);
    // What the step declares, which a tick cannot change. A step Fleet holds
    // no record of declares nothing, and a box ticked on it says so.
    const unmeant = unmeantOf(gate, {
      checks: (step?.checks?.length ?? 0) > 0,
      judge: (step?.judge_checks?.length ?? 0) > 0,
    });
    const row: ProposalGateRow = {
      id: gate.step_id,
      label: step?.label ?? gate.step_id,
      checks: gate.checks,
      judge: gate.judge,
      you: gate.you,
      advanceGate: reading.advance_gate,
      does: reading.does,
    };
    if (gate.repository_decides !== undefined) row.repositoryDecides = gate.repository_decides;
    if (gate.overridden !== undefined) row.overridden = gate.overridden;
    if (unmeant !== undefined) row.unmeant = unmeant;
    return row;
  });
}

/** One box moved on one step, with every other step carried through. */
export function gatesWith(
  gates: readonly GateView[],
  stepId: string,
  change: Partial<GateView>,
): GateView[] {
  return gates.map((gate) => (gate.step_id === stepId ? { ...gate, ...change } : gate));
}

/** What each answer to "complete when" is called, and whether Fleet can tell. */
const COMPLETE_LABEL: Readonly<Record<CompleteWhen, string>> = {
  pr_merged: "Its pull request merges",
  all_members_landed: "Every Job it holds has landed",
  pr_opened: "Its pull request is opened",
  delivered: "The step that delivers has delivered",
};

/**
 * The four answers, each said with whether anything on a Job's record answers
 * it today. **`COMPLETE_WHEN_SERVED` is read rather than restated**, so an
 * answer Fleet learns to observe stops being flagged here without this file
 * being touched.
 */
export function completeChoices(): CompleteChoice[] {
  return (Object.keys(COMPLETE_LABEL) as CompleteWhen[]).map((value) => ({
    value,
    label: COMPLETE_LABEL[value],
    served: COMPLETE_WHEN_SERVED[value],
  }));
}

/** The landing rule as the controls hold it. `null` is drawn as empty, never as a name. */
export function landingValueOf(landing: LandingRule): ProposalLandingValue {
  return {
    target: landing.target ?? "",
    from: landing.from_ref ?? "",
    branching: landing.branching,
    completeWhen: landing.complete_when,
    prMode: landing.pr_mode,
  };
}

/**
 * The controls' answer, back on the rule.
 *
 * **An emptied field is `null` and not `""`.** `null` is the Manifest naming
 * no base, which is a state `amending.ts` already spells that way; an empty
 * string would be a branch with no name.
 */
export function landingWith(
  landing: LandingRule,
  moved: ProposalLandingValue,
): LandingRule {
  return {
    ...landing,
    target: moved.target === "" ? null : moved.target,
    from_ref: moved.from === "" ? null : moved.from,
    branching: moved.branching,
    complete_when: moved.completeWhen as CompleteWhen,
    pr_mode: moved.prMode,
  };
}

/** What the Job is held to, with where each line's words came from. */
export function criteriaRowsOf(criteria: readonly CriterionView[]): ProposalCriterion[] {
  return criteria.map((criterion, at) => {
    const row: ProposalCriterion = {
      id: criterion.criterion_id ?? String(at),
      text: criterion.text,
      origin: originSaidOf(criterion),
      verifiedBy: criterion.verified_by,
    };
    // The instant is the issue's own edit and never the freeze — absent is the
    // ordinary case, where nothing has moved since.
    const moved =
      criterion.origin_moved_at === undefined
        ? null
        : absoluteOf(criterion.origin_moved_at);
    if (moved !== null) row.movedSince = moved;
    return row;
  });
}

/** One criterion reworded, with the rest carried through. */
export function criteriaWith(
  criteria: readonly CriterionView[],
  at: number,
  text: string,
): CriterionView[] {
  return criteria.map((criterion, index) =>
    index === at ? { ...criterion, text } : criterion,
  );
}

/**
 * When the Job was approved, written out — or `undefined` where nobody has.
 *
 * **The instant is the whole of what separates the two moments**, so a
 * proposal carrying one whose timestamp cannot be read draws as frozen with no
 * date rather than as still editable.
 */
export function frozenAtOf(proposal: ProposalView): string | undefined {
  if (proposal.approved_at === undefined) return undefined;
  return absoluteOf(proposal.approved_at) ?? proposal.approved_at;
}
