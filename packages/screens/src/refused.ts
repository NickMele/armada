// What a stopped Job reached for and was refused, as the band draws it.
//
// **The evidence for the trigger, and nothing on screen carried it.**
// `blocked_by_policy` named a policy and no surface named what the policy
// stopped, so a person told to unblock a Job had nothing saying what to unblock
// it from. `stopped_by`, `recourse` and `worktree_on_disk` all render in the
// band above a stopped step's story; this is read from the same `stuck` and
// drawn beside them.
//
// **The rows are the wire's, and the sentences around them are here.**
// `recovery.ts` is the file that says what a stopped Job's screen reads, and
// the words live beside the reading there for the reason they do here: a
// heading and a size note that disagreed with the rows under them would be two
// answers to one question.
//
// **The list did not finish its own job.** It named every refusal and said
// nothing about whether a restart met them again, beside a screen whose only
// control was `Restart step` — so the rows read as the diagnosis and the button
// read as the cure. It is not one: `crates/adapters/src/harness.rs` renders
// `--allowedTools` at spawn, so a fresh Drone on the same step carries the same
// toolset. `againOf` is that fact, said where the rows are.
//
// **It is read on every trigger and not only `blocked_by_policy`.** Fleet
// gathers refusals for every stopped Job, because a Drone denied the command it
// needed escalates as `stalled` or `silent` just as often — evidence collected
// only where the classification already named a policy would be missing from
// exactly the Jobs the classification got wrong.

import type { Refused } from "@armada/components";
import type { JobDetail as JobWhole, Refusal } from "@armada/protocol";
import { offeredOf } from "./copy";
import { sizeOf } from "./story";

/** The rows, and the sentences under them. */
export type Refusals = {
  /** In the order Fleet gathered them, oldest first. Never empty. */
  refused: Refused[];
  /** That the list is shorter than what happened, where it is. */
  note?: string;
  /**
   * What a restart meets, and what would change it. **Always present where
   * there are rows**, because the fact it states is true of every refusal.
   */
  again: string;
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
    ...answersOf(one),
  }));
  return {
    refused,
    again: againOf(stuck.refused),
    ...(shortOf(refused.length, stuck.refusals) ?? {}),
  };
}

/**
 * What a restart meets. **A mechanism and not a tip** — it says how a toolset
 * is built and what has to change for it to differ, and tells nobody to do
 * anything.
 *
 * **Two sentences where a `Bash` row is in the list, one where none is.** Every
 * `Bash` grant is one declared command, since `command_rule` in
 * `crates/adapters/src/harness.rs` renders each one as `Bash(<run>:*)` and
 * there is no other spelling of one — so a refused command is always a command
 * the Manifest does not declare to this step, and the file is worth naming. A
 * refused `WebFetch` is a capability Armada grants nowhere, and naming a
 * Manifest there sends a person to edit a file that cannot help them.
 *
 * **The declared commands are not matched against the refused ones**, and that
 * is a decision rather than an omission. Nothing on this seam carries the
 * toolset a Drone was spawned with — `Stuck` holds the refusals and no grants —
 * so naming *which* row is undeclared would take a second read, and a match
 * wrong on one row is a false claim about a command in front of somebody. What
 * is said instead is true of every row without matching anything.
 *
 * **Where a row offers an answer, that is the way out, and the sentence says
 * so** instead of sending a person to edit armada.yml by hand. Fleet offers
 * answers only on a Job stopped at `blocked_by_policy`, so the two older
 * sentences stand everywhere else — including every Job an older Fleet serves.
 */
function againOf(refused: readonly Refusal[]): string {
  if (refused.some((one) => offeredOf(one.offers ?? []).length > 0)) return ALLOWED_FROM_ITS_ROW;
  const ran = refused.some((one) => one.tool === BASH);
  return ran ? DECLARED_COMMANDS : FIXED_AT_SPAWN;
}

/**
 * What a row's answers do, said once for the list. Both reaches are named,
 * because the two allows differ by exactly that and the controls say it in
 * three words each.
 */
const ALLOWED_FROM_ITS_ROW =
  "A command can be allowed from its row, for this job or for every job in the repository.";

/**
 * What a person may answer about one row, and why nothing where nothing.
 *
 * **The row's own call rides with its answers**, because an answer names the
 * call it is about, and a press sent under another row's would allow a
 * different command. A row Fleet offered nothing on carries neither: an older
 * Fleet sends no list, and a Job stopped for anything but policy an empty one.
 *
 * **`withheld` keeps Fleet's case.** Fleet uses the same clause mid-sentence,
 * so it arrives lower-case and unstopped; on a line of its own it takes a full
 * stop and not a capital, because its first word is often a file name.
 */
function answersOf(one: Refusal): Pick<Refused, "call" | "answers" | "withheld"> {
  const offered = offeredOf(one.offers ?? []);
  return {
    ...(offered.length === 0
      ? {}
      : { call: one.call, answers: offered.map(({ offer, label }) => ({ answer: offer, label })) }),
    ...(one.withheld === undefined || one.withheld === ""
      ? {}
      : { withheld: /[.!?]$/.test(one.withheld) ? one.withheld : `${one.withheld}.` }),
  };
}

/**
 * The harness's own spelling for running a command, which is the wire's: `tool`
 * arrives as the transcript recorded it. **Compared and never rendered from
 * here** — the row draws the wire's string, so an unknown spelling draws itself
 * and only loses the second sentence.
 */
const BASH = "Bash";

/** True of every refusal, and the reason a restart is not a fix on its own. */
const FIXED_AT_SPAWN =
  "A drone keeps the tools it started with, so a restart would be blocked the same way.";

/**
 * What declares a command, where a command was refused. **No path**: Fleet may
 * hold more than one repository and the one reading Bridge holds is Fleet-wide,
 * so a path drawn here could name a different repository's file from this Job's.
 *
 * **`destructive` is in the sentence and is not a detail.** A command declared
 * and marked destructive is withheld from every Drone —
 * `crates/fleet/src/spawning.rs` — so a sentence naming only the section sends
 * a person to add an entry they already have.
 */
const DECLARED_COMMANDS =
  "A drone can only run commands the repository's armada.yml declares. To allow one, add it " +
  "under commands and do not mark it destructive.";

/**
 * One command as the wire carries it: a line, whether that line is all of it,
 * and how long it was. **A refused row and a command a drone is waiting on**
 * both carry exactly this, and both say how much was cut in one grammar.
 */
type Cut = { detail: string; truncated: boolean; length?: number };

/**
 * How much of one command is on the row, where it is not all of it.
 *
 * **The same sentence the transcript's own cut arguments carry**, through
 * the same function — *showing 200 of 14,320 characters*. A command cut
 * silently is one pasted into an allowlist believing it whole.
 *
 * **A row Fleet never sized says it was cut and claims no total** —
 * `sizeOf` refuses a one-number sentence, since `200 characters shown`
 * reads as the whole of it; `cut at` cannot read that way, and an
 * unstated total is honest where nothing recorded one. The count is code
 * points, not UTF-16 — `length` on the wire is Rust's `chars().count()`,
 * disagreeing on any astral character, where a shown count larger than
 * the total would read as nonsense.
 */
export function shownOf(one: Cut): { size: string } | undefined {
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
