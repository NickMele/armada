// What the held list is divided into, and what a confirmation has to say about
// what is chosen.
//
// **Its own file because it is arithmetic, and arithmetic is unit-tested.** A
// `play` that computed rather than read would be a unit test paying a browser's
// price — `docs/practices/react.md` is explicit — and every case below is a
// sentence somebody reads immediately before destroying something.

import type { HeldReason, WorktreeHeld } from "@armada/protocol";
import { provablySafe, reclaimable } from "@armada/protocol";

/**
 * The list, split by what a person can do about each row.
 *
 * **Three groups and not two.** A worktree fleet will take back on its own, one
 * that is waiting on a decision, and one nothing may act on yet are three
 * different things to say — and a surface with two groups would have to file the
 * running job under either "decide about this" or "already handled", both of
 * which are wrong.
 */
export type Divided = {
  /** Held, and a person may choose them. What this surface exists for. */
  deciding: WorktreeHeld[];
  /** Held, and nothing may act on them yet — the job has not ended. */
  waiting: WorktreeHeld[];
  /** Nothing is holding them. Fleet gives these back on its own sweep. */
  automatic: WorktreeHeld[];
};

/** Divide the answer, keeping fleet's order inside each group. */
export function divided(worktrees: readonly WorktreeHeld[]): Divided {
  const deciding: WorktreeHeld[] = [];
  const waiting: WorktreeHeld[] = [];
  const automatic: WorktreeHeld[] = [];
  for (const held of worktrees) {
    if (provablySafe(held)) automatic.push(held);
    else if (reclaimable(held)) deciding.push(held);
    else waiting.push(held);
  }
  return { deciding, waiting, automatic };
}

/**
 * What reclaiming the chosen worktrees ends, and what it leaves standing.
 *
 * **The confirmation is built from this and from nothing else.** Bytes are not
 * on it: what a person is deciding is which commits go, whether anything else
 * has them, and which files exist nowhere but a checkout that is about to be
 * removed.
 */
export type Losing = {
  /** How many checkouts go. Always the count of what was chosen. */
  checkouts: number;
  /**
   * Files written and committed nowhere, by the job they are under.
   *
   * **The only thing here that cannot be got back.** No branch carries them, so
   * removing the directory is the end of them — which is why they are listed
   * rather than counted into a total.
   */
  destroying: { jobId: string; title: string; files: string[] }[];
  /**
   * Branches left standing, with the commits that kept them there.
   *
   * **Not a loss, and the confirmation says so in those words.** There is no
   * force on this seam, so a branch the base cannot reach survives its own
   * checkout — the commits stay, reachable from the tip.
   */
  keeping: { jobId: string; title: string; branch: string; commits: number; tip: string }[];
};

/** Read what is chosen, in the order it was drawn. */
export function losing(chosen: readonly WorktreeHeld[]): Losing {
  const destroying: Losing["destroying"] = [];
  const keeping: Losing["keeping"] = [];
  for (const held of chosen) {
    for (const reason of held.held) {
      if (reason.why === "uncommitted" && reason.files.length > 0) {
        destroying.push({ jobId: held.job_id, title: held.job_title, files: reason.files });
      }
      if (reason.why === "unmerged") {
        keeping.push({
          jobId: held.job_id,
          title: held.job_title,
          branch: held.branch,
          commits: reason.commits,
          tip: reason.tip,
        });
      }
    }
  }
  return { checkouts: chosen.length, destroying, keeping };
}

/**
 * What the confirmation is called.
 *
 * **It names the act and the count, never "are you sure".** The design system's
 * rule for a confirmation is that it states what happens and what survives, and
 * a title that asks a person to be sure states neither.
 */
export function confirmTitle(chosen: number): string {
  return chosen === 1 ? "Reclaim this worktree?" : `Reclaim ${chosen} worktrees?`;
}

/**
 * The first line of the confirmation: what happens, and what survives it.
 *
 * **The record survives and the sentence says so**, because it is the fact most
 * often assumed the other way — reclaiming takes disk and leaves the job on the
 * board with everything it recorded.
 */
export function confirmOpening(losing: Losing): string {
  const checkouts =
    losing.checkouts === 1 ? "One checkout is removed" : `${losing.checkouts} checkouts are removed`;
  return `${checkouts}. The jobs stay on the board with everything they recorded — this takes the disk and not the record.`;
}

/**
 * The sentence for a set where nothing is destroyed.
 *
 * **Said rather than left as an absence.** A confirmation that lists nothing
 * reads as a confirmation that failed to say what it costs, and this is the
 * ordinary case: with no force on the seam, most reclaims lose nothing at all.
 */
export const NOTHING_IS_LOST =
  "Nothing is lost. Every commit stays on the branch it is on, and a branch the base cannot reach is kept where it is.";

/**
 * How many files a reclaim would end, over every chosen worktree.
 *
 * A total for the one sentence that needs one — the warning above the list —
 * while the files themselves stay grouped under the job that wrote them.
 */
export function filesDestroyed(losing: Losing): number {
  return losing.destroying.reduce((total, one) => total + one.files.length, 0);
}

/**
 * Which reason is the one that decides a row's answer, where several apply.
 *
 * **Uncommitted wins, always.** It is the only reason on the list where the act
 * ends something, and a row summarised by its unmerged branch would tell a
 * person the safe half of a decision that also has an unsafe half.
 */
export function decides(held: WorktreeHeld): HeldReason | null {
  return (
    held.held.find((reason) => reason.why === "uncommitted") ??
    held.held.find((reason) => reason.why === "unreadable") ??
    held.held[0] ??
    null
  );
}
