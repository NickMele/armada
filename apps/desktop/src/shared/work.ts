// The material a person reads before deciding, as TypeScript sees it.
// `crates/ipc/src/work.rs`.
//
// Its own file for the reason the Rust side gives it one: these are two reads
// and neither is a field on `JobDetail`. A detail is fetched on every open of a
// Job to draw a summary, and `adapter-traits`' `WorkProduct` splits the file
// list from the patch because the bytes are large and most steps ask no
// semantic question. **This is where the expensive half is finally spent** — on
// a person deciding whether to take the work, which is the case the bytes are
// for — so it is asked for on that act and nothing else pays for it.
//
// Evidence is split from the diff for the same reason one step down: a surface
// wanting only the claims would otherwise fetch a megabyte to read four lines.
//
// **Absent, never present-and-empty.** `JobDiff.work` is absent where there was
// no worktree to read; an empty `files` is a Drone that changed nothing, which
// is a real and different answer. Collapsing the two would tell somebody a
// Drone wrote nothing when what is true is that it never had a worktree.

import type { ChangedFile } from "./protocol";

/** What one Job's worktree holds against the branch it was cut from. */
export type JobDiff = {
  job_id: string;
  /**
   * The reading. **Absent where there was no worktree to read**: a Job still at
   * the approval gate, one never dispatched, or one whose worktree has been
   * reclaimed. Not the same as a reading that found nothing.
   */
  work?: Work;
};

/** One reading of a worktree: which files moved, and what moved inside them. */
export type Work = {
  /**
   * Every file changed since the branch was cut, in the order the reading found
   * them. **Empty is a real answer** — the worktree opened and holds no change.
   *
   * The same `ChangedFile` a `job.files_changed` event carries, so a review
   * screen and a live footprint are one vocabulary rather than two.
   */
  files: ChangedFile[];
  /**
   * Whether a step has declared a plan for `outside_plan` to mean anything.
   * **False is "there is no plan", not "nothing drifted"**, and it is false on
   * every Job whose Drone is no longer holding the pen.
   */
  plan_declared: boolean;
  /**
   * The unified diff, as the repository rendered it. **Absent where there is
   * nothing in it**, which `files` being empty says in the same breath.
   */
  patch?: string;
};

/** Every claim a Job's Drones have submitted, step by step. */
export type JobEvidence = {
  job_id: string;
  /**
   * One entry per step that has submitted evidence, in step order. **A step
   * that submitted none is absent, not blank**, and empty is a real answer —
   * no step has submitted anything.
   */
  steps: Submitted[];
};

/**
 * What one step's Drone claimed, and what it offered as the demonstration.
 *
 * **There is no `source` field, here or in the record.** A Drone marking its
 * own evidence human-attested has to be impossible on both sides of the write.
 */
export type Submitted = {
  step_id: string;
  /**
   * What the step's workflow asked the work product to be. Recorded by Fleet
   * from the frozen step, never named by the Drone.
   *
   * Left as `string` like every other closed set: `enum-verbs.toml` carries no
   * `evidence_type` rows, so there is no verb, glyph or hue for one and the
   * wire's spelling renders. Reported.
   */
  evidence_type: string;
  /** What the work now does, as an observable. */
  claimed: string;
  /** The artifact demonstrating it. */
  shown_by: string;
  /**
   * Everything the claim does not assert. **Absent where the submission drew no
   * boundary**, which the record calls legitimately empty — an empty string
   * here would read as a limit somebody forgot to write.
   */
  not_claimed?: string;
};

/**
 * What a person says when they send the work back. The body of
 * `request_changes`.
 *
 * A type of its own rather than `Redirection`, which is structurally the same
 * string: a redirect steers a Drone whose step *stopped* and this answers a
 * gate the Drone is *waiting at*. Two acts with one body would be one route
 * that means whichever the caller had in mind.
 *
 * **The one string on this route Fleet does not assemble.** The reviewer read
 * the diff and the evidence, and what they want changed is not derivable from
 * either. Blank is refused before it is sent, matching the 422 Fleet gives it.
 */
export type ChangesRequested = {
  note: string;
};
