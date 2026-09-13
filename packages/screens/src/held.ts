// What the held list is divided into, and what a confirmation has to say about
// what is chosen.
//
// **Its own file because it is arithmetic, and arithmetic is unit-tested.** A
// `play` that computed rather than read would be a unit test paying a browser's
// price — `docs/practices/react.md` is explicit — and every case below is a
// sentence somebody reads immediately before destroying something.

import type { HeldReason, JobSummary, WorktreeHeld } from "@armada/protocol";
import { provablySafe, reclaimable } from "@armada/protocol";
import type { RowChoice } from "@armada/components";
import { instant } from "./duration";

export type { RowChoice } from "@armada/components";

/** Nothing chosen for a row — every act unticked. */
export const NO_CHOICE: RowChoice = { removeCheckout: false, deleteBranch: false, forget: false };

/** What is currently chosen for `jobId`, or `NO_CHOICE` where nothing has been touched yet. */
export function choiceOf(choices: Readonly<Record<string, RowChoice>>, jobId: string): RowChoice {
  return choices[jobId] ?? NO_CHOICE;
}

/** Whether any of a row's three acts is chosen. */
export function anyChosen(choice: RowChoice): boolean {
  return choice.removeCheckout || choice.deleteBranch || choice.forget;
}

/** The `unmerged` reason on a row, where it carries one — the source of the tip a branch delete sends. */
export function unmergedOf(held: WorktreeHeld): Extract<HeldReason, { why: "unmerged" }> | null {
  return held.held.find((reason): reason is Extract<HeldReason, { why: "unmerged" }> => reason.why === "unmerged") ?? null;
}

/**
 * Which of a row's three acts it offers right now.
 *
 * **Forget reads `choice`, the other two do not.** It is the one act gated on
 * the other two: offering it only once the checkout and the branch are
 * already gone or chosen in this same act is what keeps a forgotten record
 * from orphaning disk `worktrees_held` no longer walks to.
 */
export function offeredOn(held: WorktreeHeld, choice: RowChoice): RowChoice {
  const removeCheckout = held.on_disk;
  const deleteBranch = unmergedOf(held) !== null;
  const checkoutSettled = !removeCheckout || choice.removeCheckout;
  const branchSettled = !deleteBranch || choice.deleteBranch;
  return { removeCheckout, deleteBranch, forget: checkoutSettled && branchSettled };
}

/** Rows carrying at least one chosen act, in fleet's own order. */
export function chosenRows(
  rows: readonly WorktreeHeld[],
  choices: Readonly<Record<string, RowChoice>>,
): WorktreeHeld[] {
  return rows.filter((row) => anyChosen(choiceOf(choices, row.job_id)));
}

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
 * A `depended_on` reason, with its `by` read against the handle a person
 * would recognise rather than the id fleet named it by.
 *
 * **The handle where Bridge holds one, the id otherwise.** The job that is
 * still holding this worktree open is, ordinarily, still on the board — but a
 * lineage fold or a sweep between the two reads can leave it named and gone,
 * and the id is still a fact worth showing rather than nothing.
 */
export function namedByHandle(held: WorktreeHeld, jobs: readonly JobSummary[]): WorktreeHeld {
  return {
    ...held,
    held: held.held.map((reason) =>
      reason.why === "depended_on"
        ? { ...reason, by: reason.by.map((jobId) => jobs.find((job) => job.id === jobId)?.handle ?? jobId) }
        : reason,
    ),
  };
}

/** One kept or one deleted branch, on the confirmation. */
type BranchLine = { jobId: string; title: string; branch: string; commits: number; tip: string };

/**
 * What the chosen acts end, and what they leave standing.
 *
 * **The confirmation is built from this and from nothing else.** Bytes are not
 * on it: which commits go, whether anything else has them, which files exist
 * nowhere but a checkout about to be removed, and which records are forgotten.
 */
export type Planned = {
  /** How many checkouts go. Only rows with `removeCheckout` chosen. */
  checkouts: number;
  /** Files written and committed nowhere, under a checkout being removed. */
  destroying: { jobId: string; title: string; files: string[]; lastMovedAt: string }[];
  /** Unmerged branches left standing — not chosen for deletion. */
  keeping: BranchLine[];
  /** Unmerged branches chosen for deletion, with the tip a person confirmed. */
  deletingBranches: BranchLine[];
  /** Records chosen to be forgotten. There is no undo. */
  forgetting: { jobId: string; title: string }[];
};

/** Read what is chosen, over rows that carry at least one chosen act. */
export function planned(rows: readonly WorktreeHeld[], choices: Readonly<Record<string, RowChoice>>): Planned {
  const destroying: Planned["destroying"] = [];
  const keeping: Planned["keeping"] = [];
  const deletingBranches: Planned["deletingBranches"] = [];
  const forgetting: Planned["forgetting"] = [];
  let checkouts = 0;

  for (const held of rows) {
    const choice = choiceOf(choices, held.job_id);
    if (choice.removeCheckout) {
      checkouts += 1;
      for (const reason of held.held) {
        if (reason.why === "uncommitted" && reason.files.length > 0) {
          destroying.push({
            jobId: held.job_id,
            title: held.job_title,
            files: reason.files,
            lastMovedAt: held.last_moved_at,
          });
        }
      }
    }
    const unmerged = unmergedOf(held);
    if (unmerged !== null) {
      const line: BranchLine = {
        jobId: held.job_id,
        title: held.job_title,
        branch: held.branch,
        commits: unmerged.commits,
        tip: unmerged.tip,
      };
      if (choice.deleteBranch) deletingBranches.push(line);
      else keeping.push(line);
    }
    if (choice.forget) forgetting.push({ jobId: held.job_id, title: held.job_title });
  }
  return { checkouts, destroying, keeping, deletingBranches, forgetting };
}

/**
 * What the confirmation is called.
 *
 * **It names the act and the count, never "are you sure".** The design
 * system's rule for a confirmation is that it states what happens and what
 * survives, and a title that asks a person to be sure states neither.
 */
export function confirmTitle(rows: number): string {
  return rows === 1 ? "Clean up this row?" : `Clean up ${rows} rows?`;
}

/**
 * The opening line of the confirmation: what happens, over every chosen act.
 *
 * **Only the acts actually chosen get a clause.** A set that only forgets
 * records reading "0 checkouts are removed" would say more than it means.
 */
export function confirmOpening(plan: Planned): string {
  const said: string[] = [];
  if (plan.checkouts > 0) {
    said.push(plan.checkouts === 1 ? "One checkout is removed." : `${plan.checkouts} checkouts are removed.`);
  }
  if (plan.deletingBranches.length > 0) {
    said.push(
      plan.deletingBranches.length === 1
        ? "One branch is deleted."
        : `${plan.deletingBranches.length} branches are deleted.`,
    );
  }
  if (plan.forgetting.length > 0) {
    said.push(
      plan.forgetting.length === 1
        ? "One job's record is forgotten, and there is no undo."
        : `${plan.forgetting.length} jobs' records are forgotten, and there is no undo.`,
    );
  }
  return said.length === 0 ? "Nothing is chosen, so nothing happens." : said.join(" ");
}

/**
 * The sentence for a set where nothing is destroyed, deleted or forgotten.
 *
 * **Said rather than left as an absence.** A confirmation that lists nothing
 * reads as one that failed to say what it costs, and this is the ordinary
 * case for a checkout on its own: reclaiming never forces a branch.
 */
export const NOTHING_IS_LOST =
  "Nothing is lost. Every commit stays on the branch it is on, and a branch the base cannot reach is kept where it is.";

/**
 * How many files removing the chosen checkouts would end.
 *
 * A total for the one sentence that needs one — the warning above the list —
 * while the files themselves stay grouped under the job that wrote them.
 */
export function filesDestroyed(plan: Planned): number {
  return plan.destroying.reduce((total, one) => total + one.files.length, 0);
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

/**
 * How long a checkout has been sitting, in the coarsest true unit.
 *
 * **Its own formatter and not `lasting`.** That one writes a run — `4m 09s`,
 * `2h 13m` — because a job in flight is read to the second. This is an age, and
 * `97h 12m` is a number nobody converts in their head at the moment they are
 * deciding whether four days of untouched work is worth opening the directory
 * for. Two quantities, two formatters, and neither pretends to be the other.
 *
 * **It rounds down and never up.** `last_moved_at` is already a floor — armada
 * last moved the job then, and the files were written at or before it — so
 * rounding up would turn a floor into a claim.
 *
 * `null` where the stamp will not parse or is in the future, which is the same
 * convention `instant` sets: a checkout whose date is unreadable says nothing
 * rather than showing an age measured from zero.
 */
export function sitting(at: string, now: number): string | null {
  const moved = instant(at);
  if (moved === null || moved > now) return null;
  const minutes = Math.floor((now - moved) / 60_000);
  if (minutes < 1) return "under a minute";
  if (minutes < 60) return minutes === 1 ? "1 minute" : `${minutes} minutes`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return hours === 1 ? "1 hour" : `${hours} hours`;
  const days = Math.floor(hours / 24);
  return days === 1 ? "1 day" : `${days} days`;
}
