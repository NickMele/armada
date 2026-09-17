// What the Drone is doing right now, for the live phase's header. #1196.
//
// **The call in flight, not the last row of the log.** A person watching a
// running Job could tell it apart from a sleeping one only by reading the
// stream, which is the defect this answers: the header says the thing itself.
//
// **A call with no answer is the whole definition.** Fleet sends `called` and
// `answered` as two turns with the tool running in the gap, so a call nobody
// has answered and nobody refused is, by construction, the one still open.
import { toolFamily } from "@armada/components";
import type { Turn } from "@armada/protocol";

import { instant, lasting } from "./duration";
import { editOf, inside, isEcho } from "./story";

/** What the Drone is doing, as the header draws it. */
export type Doing = {
  /**
   * The verb, hued. Absent between calls, where the Drone's own sentence is
   * what there is to say and no verb would be true of it.
   */
  verb?: string;
  /** What it is doing it to: a path, a command, or the sentence itself. */
  detail?: string;
  /** How long the call has been open — `3s`. Absent on a sentence. */
  took?: string;
  /** Whether `detail` is machine-derived. A path is; a sentence is not. */
  mono?: boolean;
};

/**
 * The verb for a tool.
 *
 * **Named per tool rather than per family**, because the three families are a
 * colour and these are words a person reads: *Looking* is true of `Read`,
 * `Grep` and `Glob` alike and says less than any of them. A tool with no entry
 * falls back to its own name, which is honest and is what the log row already
 * shows.
 */
const VERB: Record<string, string> = {
  Read: "Reading",
  Grep: "Searching",
  Glob: "Listing",
  Edit: "Editing",
  MultiEdit: "Editing",
  Write: "Writing",
  NotebookEdit: "Editing",
  Bash: "Running",
};


/**
 * What the Drone is doing on this step, or nothing where it has done nothing
 * this attempt.
 *
 * `now` is the clock the panel already ticks on, so the open call's figure
 * moves with everything else on the screen rather than on a timer of its own.
 */
export function doingNow(
  turns: readonly Turn[],
  stepId: string | undefined,
  now: number,
): Doing | undefined {
  const mine = turns
    .filter((turn) => stepId === undefined || turn.step === undefined || turn.step === stepId)
    .filter((turn) => !isEcho(turn));

  const settled = new Set<string>();
  for (const turn of mine) {
    if (turn.saw.event === "answered" || turn.saw.event === "refused") settled.add(turn.saw.call);
  }

  for (let at = mine.length - 1; at >= 0; at -= 1) {
    const turn = mine[at] as Turn;
    const saw = turn.saw;
    if (saw.event === "called" && !settled.has(saw.call)) {
      const opened = instant(turn.ts);
      // The worktree prefix off, as every log row already drops it: forty
      // characters of machine before the file would fill the line.
      const detail =
        saw.detail === ""
          ? undefined
          : inside(
              toolFamily(saw.tool) === "changing"
                ? editOf(saw.detail, saw.truncated).path
                : saw.detail,
            );
      return {
        verb: VERB[saw.tool] ?? saw.tool,
        ...(detail === undefined ? {} : { detail, mono: true }),
        ...(opened === null ? {} : { took: lasting(Math.max(0, now - opened)) }),
      };
    }
    // Between calls the Drone's last sentence is what it is doing. Walked from
    // the newest end, so whichever of the two happened last is what is drawn.
    if (saw.event === "said") return { detail: saw.text };
  }
  return undefined;
}
