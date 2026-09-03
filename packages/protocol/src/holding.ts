// What Fleet is holding disk for, and the test each one did not pass.
// `crates/ipc/src/holding.rs`.
//
// **The reason is the payload, not a label on it.** Not-provably-safe is one
// word for several situations a person answers differently — a branch the base
// cannot reach, files nobody committed, a job still moving, a job something
// else is waiting on — so each arm carries what its own decision needs. A count
// of commits without the branch they are on, or a claim of uncommitted work
// without the filenames, is a row that asks somebody to guess.
//
// **A piloted job's checkout is not on this wire.** Fleet drops it before the
// answer is built, so there is no arm below for it and nothing here could draw
// one by mistake. `#367` is the reason: a person is at an unrestricted toolset
// in that directory.
//
// **Nothing says how large a worktree is, deliberately.** Bytes are not the
// decision. Which commits go, whether anything else has them, and which
// uncommitted files exist nowhere but that checkout is.

/**
 * Every worktree Fleet is holding disk for, ordered by job id.
 *
 * **Complete, including the ones the sweep is about to take on its own.** A
 * list filtered to the held ones would be a list the next sweep changes, and a
 * person who came looking for a worktree that is not on it could not tell
 * "already given back" from "held and not said".
 */
export type WorktreesHeld = { worktrees: WorktreeHeld[] };

/** One job's worktree, and every test it failed. */
export type WorktreeHeld = {
  job_id: string;
  job_title: string;
  status: string;
  /**
   * When armada last moved anything on this job.
   *
   * **Not when the files in the checkout were last written**, and nothing on
   * this seam can be: the dirty reading answers names and not times. It is a
   * floor — a checkout whose job stopped four days ago has been sitting at
   * least that long — and it is here because it is read against `uncommitted`,
   * the one reason where reclaiming ends something.
   */
  last_moved_at: string;
  /** The checkout on disk — what a person goes and looks at. */
  path: string;
  /** The branch the job derived, named even where it is already gone. */
  branch: string;
  /**
   * **Empty is the whole of the safety claim.** Fleet's own sweep takes
   * exactly the empty ones, so a row with nothing here is a row nobody has to
   * decide about.
   */
  held: HeldReason[];
};

/**
 * One test a worktree did not pass.
 *
 * Discriminated on `why`, and every arm is matched rather than rendered — which
 * is what makes a reason added to this set a **major** protocol bump rather
 * than a minor one. `docs/practices/protocol.md` has the row.
 */
export type HeldReason =
  /**
   * The job is still moving, so it may still need its worktree. **Nothing may
   * reclaim this one** — Fleet refuses a status that is not terminal, so a
   * surface offering it would be offering a 409.
   */
  | { why: "not_terminal"; status: string }
  /**
   * The branch holds commits the base cannot reach.
   *
   * **Reclaiming does not destroy them.** There is no force on this seam, so
   * the checkout goes and the branch stays exactly where it is — which is why
   * the tip travels: it is what the work is reachable from afterwards.
   */
  | { why: "unmerged"; base: string; commits: number; tip: string }
  /** Nothing could say what the branch would be merged into, so it is kept. */
  | { why: "base_unanswered"; detail: string }
  /**
   * Files written and committed nowhere.
   *
   * **The one reason where reclaiming destroys something.** No branch carries
   * these, so removing the directory is the end of them — which is why they are
   * named file by file and why the confirmation reads them out.
   */
  | { why: "uncommitted"; files: string[] }
  /** Somebody locked the checkout, which is a person saying not yet. */
  | { why: "locked"; reason: string }
  /** A job that depends on this one has not finished. */
  | { why: "depended_on"; by: string[] }
  /** Version control would not say what is in the checkout. */
  | { why: "unreadable"; detail: string };

/** Whether Fleet will give this one back without being asked. */
export function provablySafe(held: WorktreeHeld): boolean {
  return held.held.length === 0;
}

/**
 * Whether a person may choose this one.
 *
 * **A job that is not terminal is drawn and not offered.** Fleet refuses the
 * act with a 409 while a drone might still be writing, so a checkbox on that
 * row would be a control whose only outcome is a refusal — and #385's own table
 * says what a person decides about one: leave it alone.
 */
export function reclaimable(held: WorktreeHeld): boolean {
  return !held.held.some((reason) => reason.why === "not_terminal");
}
