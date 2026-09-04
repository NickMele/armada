// Which jobs are waiting on a person, the sentence that counts them, and the
// order a queue of them is read in.
//
// **The Board arithmetic main needs too, and only that.** Everything here is
// also drawn by `board.ts`, which re-exports it; nothing here is a second
// reading of anything.
//
// **Moved out of `board.ts` whole, and not rewritten.** The rule is unchanged;
// what changed is who can read it. Bridge's main process has to know the same
// set the Needs-you tab draws — it is what decides when to tell somebody away
// from the app — and main is a Node bundle with no React in it.
//
// **That is why `JOB_LIFECYCLE` is imported from the generated module rather
// than from `@armada/components`.** The barrel is every primitive and every
// composition; importing one constant through it puts `react`, `lucide-react`
// and a stylesheet chunk into the main bundle, which was measured rather than
// assumed — 86 kB and no React became 95 kB, a CSS chunk and three `require`s
// the main process has no use for. The generated file is data, so reaching it
// by name costs the map and nothing else.
//
// Nothing else in this module may import from `@armada/components`. A single
// import of `plural` here would undo the paragraph above, silently, and the
// only thing that would say so is the size of a bundle nobody reads.

import { JOB_LIFECYCLE } from "@armada/components/src/generated/vocabulary";
import type { JobSummary } from "@armada/protocol";
import { instant } from "./duration";

/**
 * The five tabs, in the order they are drawn and keyed.
 *
 * **Here rather than in `board.ts`, where the strip that draws them is.** A
 * type import is erased at runtime and still typechecked, so leaving it there
 * would pull the whole component barrel into main's project — the same cost
 * this file exists to avoid, arriving through the type system instead of
 * through the bundle. `board.ts` re-exports both, and the tab strip is still
 * the only thing that draws them.
 */
export type BoardTab = "all" | "needs-you" | "running" | "queued" | "finished";

/** The four a job can be in. `all` is not a state and holds every job. */
export type StateTab = Exclude<BoardTab, "all">;

/**
 * Which state tab a job is in, or `null` where this build's registry has no
 * lifecycle row for its status.
 *
 * **`null` is a real answer and it is not a fifth tab.** Two things reach it: a
 * status this build's registry has never heard of, and one whose `terminal`,
 * `mode` and `who_is_acting` match none of the four rules. The first also has no
 * verb and no glyph, so the list already draws it as a named line beneath the
 * rows rather than as a row.
 *
 * Either way it counts under `All` and under no state tab, which makes the
 * counts stop summing — visibly, which is the point. A residual tab that
 * swallowed both would make a registry change look like nothing happened.
 */
export function tabOf(job: JobSummary): StateTab | null {
  const life = JOB_LIFECYCLE[job.status];
  if (life === undefined) return null;
  if (life.terminal) return "finished";
  // **The one rule here that is not a lifecycle row, and it is first for a
  // reason.** A drone waiting on an answer is on a `running` job whose
  // `who_is_acting` is `Drone`, so every rule below would put it under Running
  // — and a question on a job nobody has open would be invisible until somebody
  // opened it. The registry cannot say this: it is a fact about a live slot
  // rather than about a status, which is exactly why it rides on the row.
  //
  // It is above `terminal` in intent and below it in code because a terminal
  // job has no drone left to be waiting, so the two can never both be true.
  if (job.asking) return "needs-you";
  // Before the actor, and not after it: `piloted` is `Working` with a person
  // acting, and a job somebody has taken over is still moving.
  if (life.mode === "Working") return "running";
  if (life.whoIsActing === "Person") return "needs-you";
  if (life.whoIsActing === "Drone") return "queued";
  return null;
}

/** Whether a job is one of the ones waiting on a person. */
export function needsYou(job: JobSummary): boolean {
  return tabOf(job) === "needs-you";
}

/**
 * "Nothing needs you", or how many do.
 *
 * **One sentence with one owner.** The Board says it beside its controls and a
 * notification says it on a desktop with no Bridge on screen, and those two
 * saying it differently is how a person comes to distrust both — so the count
 * clause lives beside the rule that produced the count.
 */
export function needsYouClause(count: number): string {
  if (count === 0) return "Nothing needs you.";
  return count === 1 ? "1 job needs you." : `${count} jobs need you.`;
}

/**
 * Oldest first, with an unreadable date last and the id as the tiebreak.
 *
 * **Never first.** A corrupt date must not shove real work off a bounded
 * window, whether that window is the top of the Board or the three jobs a
 * notification has room to name.
 */
export function oldest(a: JobSummary, b: JobSummary): number {
  const left = instant(a.created_at);
  const right = instant(b.created_at);
  if (left === null) return right === null ? a.id.localeCompare(b.id) : 1;
  if (right === null) return -1;
  return left === right ? a.id.localeCompare(b.id) : left - right;
}
