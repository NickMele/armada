// What giving one Job's worktree and branch back did. `crates/ipc/src/reclaimed.rs`.
//
// **Split out of `protocol.ts`, not authored apart from it.** That file is the
// one import for the wire vocabulary and still is — it re-exports every type
// here — but it reached the 900 lines the gate refuses, and the cut follows a
// seam `crates/ipc` already draws rather than one invented for the line count.
// `report.ts` and `events.ts` were cut the same way for the same reason.

/**
 * What giving one Job's worktree and branch back did, half by half.
 *
 * **The record survives this.** `reclaim_worktree` takes the disk and
 * `forget_job` takes the row; the Job is still on the board afterwards, which
 * is why nothing here is folded or removed.
 *
 * **Two halves, because half of it happening is a real outcome.** A branch
 * holding commits the base cannot reach is kept on purpose while its checkout
 * goes — this seam always runs with the safe setting and has no force — so a
 * single flag would have to lie about one of them.
 */
export type WorktreeReclaimed = {
  job_id: string;
  worktree: ReclaimedWorktree;
  branch: ReclaimedBranch;
};

/** What became of the checkout. */
export type ReclaimedWorktree = {
  /** Where it was. Named even when it was already gone: it is what a person checks by hand. */
  path: string;
  /** Whether the checkout is gone from disk. True where there was nothing there to begin with. */
  removed: boolean;
  /** Why it is still there, where it is — a lock message, or what version control said. */
  why?: string;
};

/**
 * What became of the branch the Job derived.
 *
 * **`unmerged_commits` is what tells a deliberate keep from a failure.** It is
 * set only where the branch was left standing because deleting it would
 * destroy work nobody has taken; anything else carrying a `why` is something
 * that did not work.
 */
export type ReclaimedBranch = {
  branch: string;
  deleted: boolean;
  /** The commit it pointed at. A deleted branch is recoverable from its SHA and nothing else. */
  tip?: string;
  why?: string;
  base?: string;
  unmerged_commits?: number;
};
