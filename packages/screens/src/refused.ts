// What a stopped Job reached for and was refused, as the band draws it.
//
// **The evidence for the trigger, and nothing on screen carried it.**
// `blocked_by_policy` named a policy and no surface named what the policy
// stopped, so a person told to unblock a Job had nothing saying what to unblock
// it from. `stopped_by`, `recourse` and `worktree_on_disk` all render in the
// band above a stopped step's story; this is read from the same `stuck` and
// drawn beside them.
//
// **The rows are the wire's, and the two sentences are here.** `recovery.ts` is
// the file that says what a stopped Job's screen reads, and the words live
// beside the reading there for the reason they do here: a heading and a size
// note that disagreed with the rows under them would be two answers to one
// question.
//
// **It is read on every trigger and not only `blocked_by_policy`.** Fleet
// gathers refusals for every stopped Job, because a Drone denied the command it
// needed escalates as `stalled` or `silent` just as often — evidence collected
// only where the classification already named a policy would be missing from
// exactly the Jobs the classification got wrong.

import type { Refused } from "@armada/components";
import type { JobDetail as JobWhole, Refusal } from "@armada/protocol";
import { sizeOf } from "./story";

/** The rows and the two sentences over and under them. */
export type Refusals = {
  /** In the order Fleet gathered them, oldest first. Never empty. */
  refused: Refused[];
  /** What the rows are, said once over them. */
  said: string;
  /** That the list is shorter than what happened, where it is. */
  note?: string;
};

/**
 * What this Job was refused, or `undefined` where it was refused nothing.
 *
 * **Nothing is the common case and draws nothing at all.** Most stopped Jobs
 * were refused no call, and a section with a heading over an empty list on
 * every one of them would say a policy was involved where none was.
 *
 * **A Fleet behind this Bridge cannot reach here**, since `connects()` admits
 * `same` and `fleet_ahead` and nothing else — so an absent `refused` is a Job
 * Fleet classified before the field existed, and it reads as nothing refused
 * rather than as a gap.
 */
export function refusedIn(whole: JobWhole | null): Refusals | undefined {
  const stuck = whole?.stuck;
  if (stuck === undefined || stuck.refused.length === 0) return undefined;
  const refused = stuck.refused.map((one) => ({
    tool: one.tool,
    detail: one.detail,
    ...shownOf(one),
    // Absent rather than empty, because the row draws nothing for an absent
    // reason and `because` is empty on almost every refusal the harness sends.
    ...(one.because === "" ? {} : { because: one.because }),
  }));
  return { refused, said: SAID, ...(shortOf(refused.length, stuck.refusals) ?? {}) };
}

/**
 * What the rows are. **The subject is the Job and not the Drone**, which is the
 * error contract's rule and is also the truthful one here: Fleet reads every
 * transcript a Job's log names, so several attempts' Drones fold into one list.
 */
const SAID = "What this job reached for and was refused:";

/**
 * How much of one command is on the row, where it is not all of it.
 *
 * **The same sentence the transcript's own cut arguments carry**, through the
 * same function rather than through a second one written to look like it —
 * *showing 200 of 14,320 characters*. A command cut without saying so is one a
 * person pastes into an allowlist believing it is whole, which is the failure
 * this rides one level down from.
 *
 * **A row Fleet never sized says it was cut and claims no total.** `sizeOf`
 * refuses a one-number sentence for the reason it gives: `200 characters shown`
 * reads as the whole of it. Leading with `cut at` cannot be read that way, and
 * an unstated total is the honest answer where nothing recorded one.
 *
 * The count is code points and not UTF-16 units, because `length` on the wire
 * is Rust's `chars().count()` — the two disagree on any command carrying an
 * astral character, and a shown count larger than the total would read as
 * nonsense.
 */
function shownOf(one: Refusal): { size: string } | undefined {
  if (!one.truncated) return undefined;
  const shown = [...one.detail].length;
  return { size: sizeOf(shown, one.length) ?? `cut at ${shown.toLocaleString()} characters` };
}

/**
 * That the list is shorter than what happened, in the words the transcript's
 * own cut arguments already use — *showing 200 of 14,320 characters*.
 *
 * **A size and never a warning.** A capped list read as the whole one is worse
 * than no list, and the count rides beside the rows on the wire for exactly
 * this. Absent where every refusal fits, which is the ordinary case.
 *
 * The two are compared rather than the count trusted on its own: Fleet raises a
 * total below what it kept, so a note here can never claim fewer refusals than
 * there are rows under it.
 */
function shortOf(shown: number, inAll: number): { note: string } | undefined {
  if (inAll <= shown) return undefined;
  return {
    note: `showing ${shown.toLocaleString()} of ${inAll.toLocaleString()} refused calls`,
  };
}
