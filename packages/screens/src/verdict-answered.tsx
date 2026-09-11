// The verdict sheet's fifth arrangement — a Job that is done, after a person
// answered it somewhere along the way: approved, merged, or overruled a
// verdict that had stopped a step.
//
// **Split out of `verdict.tsx` rather than added to it.** That file's own
// header already names the seam this follows: `checks.tsx`, `gates.ts` and
// `verdicts.tsx` are each one region of the panel read off one `StepDetail`,
// and this is the one region that also reads the Job's own transitions and
// its log — `humanGateStepOf` off `JobDetail.steps`, `overruleReasonOf` off
// the Job's log, neither of which the other four blocks ever need.
//
// **`verdictOf` is still the whole of what the four shared blocks draw.**
// This file adds only what states 1 through 4 never draw: the header, the
// settled pull request, and the record that replaces the buttons.

import type { ReactNode } from "react";
import { VerdictSheet, type VerdictSheetProps } from "@armada/components";
import type {
  JobDetail as JobWhole,
  JobSummary,
  Noted,
  Settled,
  StepDetail,
  Submitted,
} from "@armada/protocol";

import { pullRequestNumber } from "./facts";
import { absoluteOf } from "./duration";
import { LANDED } from "./Row";
import type { Opens } from "./phases";
import type { Render } from "./render";
import { verdictOf, type VerdictReads } from "./verdict";

/**
 * The step a person answers at — `advance_gate: human_always` — where this
 * Job's frozen workflow carries one. **The Job's own fact and not the open
 * step's**: a reader may be looking at any step's record, and when the Job
 * itself was answered does not move with them. `neverAsksAPerson` already
 * establishes at most one workflow answers this question one way; this is
 * the same read, keeping the step rather than only the boolean.
 */
export function humanGateStepOf(steps: readonly StepDetail[]): StepDetail | undefined {
  return steps.find((one) => one.advance_gate === "human_always");
}

/** The one line Fleet writes when a person overrules a gate. `crate::overruling`'s own. */
const OVERRULE_NOTE = "a person overruled the gate and the step advanced";

/**
 * The reason a person gave for overruling the named step, where the Job's own
 * log kept one. **`undefined` is a real answer, never a placeholder** — an
 * overrule with nothing typed into it writes no `said` field at all
 * (`Overruling::saying`), and a step overruled before Fleet logged the note
 * carries none either. Both draw the finding alone, with no invented reason
 * standing in for a person's own words.
 *
 * **Scoped to one step and the last note about it.** A step retried after an
 * overrule can be overruled again, and the most recent note is the one that
 * answers "why does this step's record say what it says now".
 */
export function overruleReasonOf(notes: readonly Noted[], stepId: string): string | undefined {
  const note = [...notes].reverse().find((one) => one.step === stepId && one.msg === OVERRULE_NOTE);
  return note?.fields?.find((field) => field.name === "said")?.value;
}

/**
 * The pull request block for a Job that is over — case 5 of `why-b.md`.
 * **`pullRequestBlockOf`'s counterpart for a settled one**, not a variant of
 * it: that block reads `mergeable` and open reviews, which a merged or closed
 * pull request no longer carries (`JobDelivery.pull_request_detail` is absent
 * exactly where `landed` is present). `LANDED` is the one place "merged" and
 * "closed without merging" are spelled, so this never writes its own copy of
 * either word.
 */
export function settledPullRequestBlockOf(
  address: string | undefined,
  landed: Settled | undefined,
): ReactNode | undefined {
  if (address === undefined || landed === undefined) return undefined;
  const said = LANDED[landed];
  if (said === undefined) return undefined;
  const number = pullRequestNumber(address) ?? "Pull request";
  return (
    <p className="text-xs text-fg-muted">
      <span className="mono">{number}</span>, {said}.
    </p>
  );
}

/** What "Done" stands beside, in the header. */
const DONE = "Done";

/**
 * The header's own headline and moment, off the step a person answers at.
 * `undefined` where this Job's frozen workflow names no such step — which
 * `verdictSlotAfterAnswer`'s own caller never hands this, since that is
 * exactly what routes a finished Job to case 4 instead.
 */
function headerOf(gate: StepDetail | undefined): VerdictSheetProps["header"] {
  if (gate === undefined) return undefined;
  const when = absoluteOf(gate.updated_at) ?? gate.updated_at;
  return { done: DONE, when: gate.overridden ? `overruled ${when}` : `approved ${when}` };
}

/** The caps label over the record that replaces the buttons. `Block`'s own class. */
const YOUR_ANSWER = "Your answer";

/**
 * What replaces the buttons once a Job is done and a person answered it.
 * **What you did and when, never a second set of controls** — the acts
 * already happened, and a record that offered them again would be a page
 * asking a question nobody is asking.
 */
function yourAnswerOf(gate: StepDetail | undefined, landed: Settled | undefined): ReactNode {
  const when = gate === undefined ? undefined : absoluteOf(gate.updated_at);
  const said =
    gate === undefined || when === undefined
      ? "You answered this Job. Nothing is asked of anyone now."
      : gate.overridden
        ? `You overruled the verdict and let the work stand, on ${when}. Nothing is asked of anyone now.`
        : landed === undefined
          ? `You approved the work on ${when}. Nothing is asked of anyone now.`
          : `You approved the work on ${when}, and the pull request ${LANDED[landed] ?? "settled"}. ` +
            "Nothing is asked of anyone now.";
  return (
    <>
      <span className="armada-verdict__label">{YOUR_ANSWER}</span>
      <p className="armada-verdict__said">{said}</p>
    </>
  );
}

export type VerdictSlotAfterAnswerArgs = {
  job: JobSummary;
  whole: JobWhole | null;
  open: StepDetail;
  render: Render;
  recorded: VerdictReads;
  opensRecords: Opens;
  now: number;
  claimed: Submitted | undefined;
  /** Fleet's own reason the gate could not decide, scoped to this step. */
  undecided?: string;
  /**
   * The Job's own log. **Only for the open step's overrule reason** — the one
   * thing on this screen this file does not already have off `JobDetail` or
   * `JobSummary`.
   */
  notes: readonly Noted[];
};

/**
 * The verdict sheet once a Job is done and a person answered it along the
 * way — the fifth arrangement, drawn 2026-09-11. **Everything `verdictSlotFinished`
 * draws, plus the record of the answer**: the settled pull request in place of
 * the open one, an overruled criterion's own row where the open step carries
 * one, how many Drones ran, and what you did in place of the buttons.
 */
export function verdictSlotAfterAnswer({
  job,
  whole,
  open,
  render,
  recorded,
  opensRecords,
  now,
  claimed,
  undecided,
  notes,
}: VerdictSlotAfterAnswerArgs): ReactNode {
  const address = whole?.delivery?.pull_request;
  const landed = whole?.delivery?.landed;
  const gate = humanGateStepOf(whole?.steps ?? []);
  const reason = overruleReasonOf(notes, open.step_id);
  return (
    <VerdictSheet
      {...verdictOf({
        job,
        whole,
        step: open,
        render,
        diff: recorded.diff,
        opens: opensRecords,
        now,
        claim: claimed,
        undecided,
        reason,
        drones: true,
      })}
      header={headerOf(gate)}
      {...(address === undefined || landed === undefined
        ? {}
        : { pullRequest: settledPullRequestBlockOf(address, landed) })}
      recordNote={yourAnswerOf(gate, landed)}
    />
  );
}
