// What branches a repository has. Draft, for `crates/ipc/src/refs.rs`.
//
// Source of truth today: nothing lists them. `ManifestDeclared.base` and
// `WorktreeHeld.branch` are the only branch names Bridge ever sees, so the
// derivation below is a floor and never the repository's own list — which is
// why the field reading it still takes a name typed by hand.
//
// The composer holds neither read, so Bridge on a real Fleet answers `null`
// and both fields draw as the plain ones they were. That is `landing.ts`'s
// gap one field over, and it closes with the same read.

import type { ManifestDeclared, WorktreeHeld } from "@armada/protocol";

/** One branch a dispatch may start from or land in. */
export type BranchView = {
  name: string;
  /**
   * The Manifest's base: where a worktree is cut from, and what both fields
   * open on. One at most — a repository declares one base or none.
   */
  base: boolean;
  /**
   * The Job whose worktree is on it. Absent is the base, and absent is a
   * branch no Job of this repository holds.
   */
  job?: string;
};

/**
 * Nobody listed them, versus a list that found nothing.
 *
 * **`null` and `[]` are different sentences**, the rule `peers.ts` states:
 * `null` is nothing having asked, which is Bridge on a real Fleet today, and
 * `[]` is a repository declaring no base and holding no worktree. A field
 * drawing them alike offers an empty picker over a repository nobody read.
 */
export type BranchesAnswer = readonly BranchView[] | null;

/**
 * The branches Armada has met in this repository, base first.
 *
 * **Met-order, not sort-order.** The base is what a dispatch opens on, so it
 * leads; the worktrees follow in the order the read gave them. Sorting the run
 * would put `armada/2-…` above the base on any repository based on `main`.
 *
 * Neither read given is `null`, which is nobody having looked.
 */
export function branchesOf(
  manifest?: ManifestDeclared,
  held?: readonly WorktreeHeld[],
): BranchesAnswer {
  if (manifest === undefined && held === undefined) return null;
  const branches: BranchView[] = [];
  const seen = new Set<string>();
  const base = manifest?.base;
  if (base !== undefined && base !== "") {
    branches.push({ name: base, base: true });
    seen.add(base);
  }
  for (const worktree of held ?? []) {
    // A Job cut straight onto the base is one row and keeps the base's mark,
    // rather than appearing again under the Job that is sitting on it.
    if (worktree.branch === "" || seen.has(worktree.branch)) continue;
    seen.add(worktree.branch);
    branches.push({ name: worktree.branch, base: false, job: worktree.job_title });
  }
  return branches;
}

/**
 * The branch both fields open on, or `null` where no base is named.
 *
 * Read off the list rather than off the Manifest a second time, so a surface
 * cannot default to a branch the picker does not offer.
 */
export function baseBranch(branches: BranchesAnswer): string | null {
  return branches?.find((one) => one.base)?.name ?? null;
}
