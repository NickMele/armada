// Where a step is — its phases and its gate tiers, from the one `StepDetail`
// the panel is showing.
//
// **The phases are derived and the tiers are served.** `job_steps.state` says
// whether the step is instructed, working or submitted; `checks`, `check_runs`,
// `judge_checks`, `judged` and `advance_gate` say what looks at it afterwards.
// Nothing here invents a tier: a step declaring no Check and no Judge draws
// three stages and a sentence saying what does advance it, which is the whole
// of "an absent tier is not a failed tier".
//
// # The three records open here, each beside the thing it is evidence for
//
// A person reads a Check's output, the document the Judge read and the question
// it was asked **because a verdict went against them**, and until `#246` all
// three were a path on a screen that nothing opened — the owner hit it on a
// real Job and lost the thread on a race the log would have named.
//
// They are not a fourth region. What a Check printed belongs on the Check's own
// row, the brief belongs on the criterion it answers, and the deliverable
// belongs to Submitted, which is the phase where the Drone handed the work
// over. **Split up rather than listed together**, because the question being
// asked is never "what files does this step have" — it is "why did that one
// say that", and the answer is next to the thing that said it.
//
// # A `.tsx` because the rows carry a control now
//
// `PhaseCardRow.label`, `result` and `cited` are all `ReactNode`, so the strip
// takes an element without any change to the component. That is the whole
// reason this file grew an extension: the alternative was a path on a row and a
// second surface to open it from.

import type { ReactNode } from "react";
import { Button } from "@armada/components";

import { CRITERION_VERDICT_JUDGE } from "@armada/components";
import type { StepDetail } from "@armada/protocol";
import type { Kept } from "@armada/protocol";
// The one reading of `check_runs` and `judged`. The Checks and Verdicts
// chapters draw from the same call, which is what stops the strip and the story
// from answering "was this criterion refused" two ways. `gates.ts` says why.
import { openArtifact, type OpenArtifact } from "./opening";

/**
 * How a record is opened, and where a refusal is said.
 *
 * **Handed in rather than reached for, and required.** This file builds data
 * for a strip and holds no Job id; the Job is the panel's, and the sentence a
 * failed open writes has to reach a toast the panel owns. Required because the
 * paths were already on screen and unopenable, and an optional handler is how
 * a surface quietly goes back to that.
 */
export type Opens = {
  jobId: string;
  /**
   * Ask the host to open a file. **Carried here rather than reached for**: a
   * screen says what an unopenable file means and the process with a
   * filesystem does the opening, and the two sit on different layers.
   */
  open: OpenArtifact;
  /** Say a sentence to the person. Called only where an open did not happen. */
  onSaid: (sentence: string) => void;
};

/**
 * One record, as a control that opens it.
 *
 * **The basename is the label and the whole path is the title.** A column of
 * these clipped from the right would all read `.armada/checks/01JOB/…`, which
 * is a run of rows saying nothing; the basename is the half a person came to
 * read, and it is the half they were hunting for — `implement.3.3.log`.
 *
 * **Ghost, because opening a file decides nothing.** Nothing about the Job
 * moves and nothing is spent, so it carries the same weight as the chapter act
 * beside it rather than the weight of an act on the work.
 */
export function Opening({ path, what, opens }: { path: string; what: Kept["what"]; opens: Opens }) {
  const kept: Kept = { kept: path, what };
  return (
    <Button
      variant="ghost"
      size="sm"
      title={path}
      onClick={(event) => {
        // The row sits inside a card the strip pins on a click. Without this
        // the open would also be a press on the stage behind it.
        event.stopPropagation();
        openKept(opens, kept);
      }}
    >
      {basename(path)}
    </Button>
  );
}

/**
 * Ask the host for one kept record, and say the sentence where it did not open.
 *
 * **One call, because two surfaces open the same three files now.** The strip
 * draws a control per record and a refusal in the Verdicts chapter cites the
 * brief it answers; a second `.then` written beside the second control is a
 * second answer to "what does a failed open say", which is exactly what
 * `opening.ts` exists to hold at one.
 */
export function openKept(opens: Opens, kept: Kept): void {
  void openArtifact(opens.open, opens.jobId, kept).then((because) => {
    if (because !== null) opens.onSaid(because);
  });
}

/** The last segment of a repository-relative path. The informative half. */
export function basename(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

/**
 * What a criterion's row says the panel came to.
 *
 * **Silent about the panel at one**, the convention `Judged.member` keeps: a
 * lone judge reads exactly as it did before panels were recorded, so no step
 * grows a count it did not have.
 *
 * **Exported, because the Verdicts grid says the same thing on the same row.**
 * The split on a closed criterion is this sentence's second half, and two
 * spellings of *refused by 2 of 3* on one screen is the drift `gates.ts` names.
 */
export function howThePanelWent(verdict: string, refused: number, members: number): string {
  // Not the registry's — `criterion_verdict_judge` has only `met` and
  // `not_met`. A live question is a Bridge-only reading of the same row.
  if (verdict === "asking") return "asking you";
  const verb = CRITERION_VERDICT_JUDGE[verdict]?.verb ?? verdict;
  if (members < 2) return verb;
  // A refusal from one of three is a close call and a refusal from all three is
  // not, and that difference is the reason the member number is on the wire.
  return refused === 0 ? `${verb} · ${members} judges` : `${verb} by ${refused} of ${members}`;
}

/**
 * One kept document, read once for both surfaces that draw it.
 *
 * **The markup is not shared and should not be.** The strip wants a
 * `PhaseStageRow` with a mark and a hue; the Produced chapter wants two
 * elements in a flex row. What drifts is the reading — the ordering, the
 * `what`, and the *attempt N* label — so that is what this carries.
 */
export type KeptRead = {
  /** The path, which is also the row key: one document per attempt. */
  path: string;
  /** The control that opens it, in the words a person was hunting for. */
  opening: ReactNode;
  /** Which run wrote it — `attempt 3`. */
  attempt: string;
};

/**
 * The documents this step kept, one per run, newest run first.
 *
 * **Per run, because a re-run is a different document.** A step worked three
 * times was judged on three, and a single row would make *the one the Judge
 * read* a guess on the one screen where that question is being asked. Empty on
 * a step that declares no deliverable, and on one whose Judge was never asked —
 * the bytes are copied where the call is built and nowhere else.
 *
 * **Newest run first, on both surfaces.** The wire orders them oldest first,
 * which is what a history wants; this is a person looking at why the last run
 * went the way it did, and the run they are reading about is the one at the
 * top. The Produced chapter reversed for the same reason and said so
 * separately, and two statements of one ordering is how a step retried twice
 * ends up listing its documents two ways. #321.
 */
export function keptOf(step: StepDetail, opens: Opens): KeptRead[] {
  return [...(step.deliverables ?? [])].reverse().map((kept) => ({
    path: kept.path,
    opening: <Opening path={kept.path} what="deliverable" opens={opens} />,
    attempt: `attempt ${kept.attempt}`,
  }));
}

