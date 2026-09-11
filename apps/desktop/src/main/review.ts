// The two reads a review is made of, and the three acts that end one.
//
// Beside `connection.ts` rather than inside it, for the reason `observe.ts` and
// `request.ts` are: the connection is a socket, a runtime file and a state
// machine, and this is neither. It holds which Job each read is for and
// publishes what came back; it opens nothing and it never talks to a Drone.
//
// # Why the two reads are two
//
// `crates/ipc/src/work.rs` splits them, and the split is the whole reason the
// patch is affordable. `get_job` is fetched on every open of a Job to draw a
// summary. The evidence is four lines a step. The patch is the expensive half —
// `crates/adapter-traits/src/work_product.rs:110` separates it from the file
// list because the bytes are large and most steps ask no semantic question —
// and **a person reading a diff to decide whether to take the work is the one
// case it is for.** So it is read when that surface asks, not when a Job opens.
//
// # Neither read is refreshed by an event
//
// A history grows while a Job runs and is re-read when a move arrives. These do
// not: a Job sitting at `awaiting_review` has a Drone suspended and a worktree
// nobody is writing to, and all three decisions move the Job off the status
// that made the surface reachable. Re-reading a megabyte on every event would
// spend the bytes the split exists to save.

import type { Diff, Evidence, Remarks } from "@armada/protocol";
import type { JobDiff, JobEvidence, JobRemarks, Submitted, Work } from "@armada/protocol";
import { JobReader } from "./reader";
import { ask, type Answer } from "./request";

/**
 * What each act is called on the route table. `crates/api/src/routes.rs`.
 *
 * **`merge` is the fourth, and the only one that writes into a repository
 * Armada does not own.** It is `approve_review` with that write in front of it,
 * and Fleet performing it is what makes the repository's after-merge checks
 * run — a person who merges on the forge instead waits for a sweep that asks
 * about one pull request at a time.
 */
export type Decision = "approve_review" | "request_changes" | "reject" | "merge";

/**
 * One Job's claims and one Job's diff, each read when a surface asks for it.
 *
 * **Two ids, not one.** A surface may want the claims without the patch — the
 * record of a finished Job does — and folding them into one field would make
 * that surface pay for bytes it does not draw.
 */
export class ReviewMaterial {
  /**
   * `GET /jobs/:job_id/evidence`, published whole.
   *
   * **An empty list is `read`, not `none`.** No step has submitted anything is
   * a fact about the Job; nobody asked is a fact about the window. A surface
   * that could not tell them apart would say a Drone reported nothing when what
   * is true is that nothing was read.
   */
  private readonly claims: JobReader<{ steps: Submitted[] }>;
  /**
   * `GET /jobs/:job_id/diff`, published whole.
   *
   * **`work` stays optional all the way through.** Absent is a Job with no
   * worktree to read; present with an empty `files` is a Drone that changed
   * nothing, which is what fails a `diff_nonempty` check. Filling in an empty
   * reading here would erase the difference before any surface saw it.
   */
  private readonly patch: JobReader<{ work?: Work }>;
  /**
   * `GET /jobs/:job_id/remarks`, published whole.
   *
   * **The one read on this class that costs a forge**, which is why it is here
   * and not on the reads `takeAgain` refreshes on every occasion: no occasion
   * of `takeAgain`'s takes it. What re-takes it is a person pressing Refresh,
   * a connection coming back to a reading that failed, `job.remarks_changed`
   * (`#661`) — Fleet's own sweep finding the pull request's comments moved —
   * or the panel's own 20 s timer while this Job's comments are the ones on
   * screen (`#667`, `remarks-poll.ts`). The last two both answer through
   * {@link ReviewMaterial.remarksChanged} rather than through `takeAgain`'s
   * occasions, which is what lets them share the one re-fetch.
   *
   * **An empty list on `read` is a pull request nobody has commented on.** A
   * forge that would not answer is a refusal, so it lands as `failed` — the two
   * are different sentences and this shape keeps them apart.
   */
  private readonly conversation: JobReader<{ review: JobRemarks }>;

  constructor(
    publish: (change: { evidence?: Evidence; diff?: Diff; remarks?: Remarks }) => void,
  ) {
    this.claims = new JobReader<{ steps: Submitted[] }>({
      route: (jobId) => `/jobs/${encodeURIComponent(jobId)}/evidence`,
      keeps: (body) => ({ steps: (body as JobEvidence).steps }),
      publish: (evidence) => publish({ evidence }),
    });
    this.patch = new JobReader<{ work?: Work }>({
      route: (jobId) => `/jobs/${encodeURIComponent(jobId)}/diff`,
      keeps: (body) => ({ work: (body as JobDiff).work }),
      publish: (diff) => publish({ diff }),
    });
    this.conversation = new JobReader<{ review: JobRemarks }>({
      route: (jobId) => `/jobs/${encodeURIComponent(jobId)}/remarks`,
      keeps: (body) => ({ review: body as JobRemarks }),
      publish: (remarks) => publish({ remarks }),
    });
  }

  /** Both reads end with the window. Neither is written onto the Job. */
  close(): void {
    this.claims.close();
    this.patch.close();
    this.conversation.close();
  }

  /** Read what one Job's Drones claimed, or `null` to stop. */
  async evidence(port: number | null, jobId: string | null): Promise<void> {
    await this.claims.want(port, jobId);
  }

  /** Read one Job's worktree against its branch, or `null` to stop. */
  async diff(port: number | null, jobId: string | null): Promise<void> {
    await this.patch.want(port, jobId);
  }

  /** What people wrote on one Job's pull request, or `null` to stop. */
  async remarks(port: number | null, jobId: string | null): Promise<void> {
    await this.conversation.want(port, jobId);
  }

  /**
   * `job.remarks_changed` arrived for this Job, or its panel's own 20 s timer
   * ticked (`remarks-poll.ts`, `#667`): ask again, so a person looking at the
   * comments sees the new one without reopening the Job.
   *
   * **A no-op for every Job but the one held.** Nobody has read this Job's
   * comments unless `Decide` opened on a pull request and called
   * {@link remarks} — so a Job this reader holds no id for has nobody to
   * refresh, whichever of the two callers this is.
   *
   * **`again`, not `want`.** `want` republishes a transient `reading` state
   * before the answer lands; a person part-way through ticking comments does
   * not need the block to blank and redraw around them, only the list under
   * it to gain the new one.
   */
  async remarksChanged(port: number, jobId: string): Promise<void> {
    if (this.conversation.jobId === jobId) await this.conversation.again(port);
  }

  /**
   * All three again, for whichever is open. The bar's Refresh reaches this.
   *
   * **The conversation is in it, and it is the one that costs a process.** A
   * person pressing Refresh on a pull request they are deciding about is asking
   * exactly this question — has anybody said anything since — and nothing else
   * in Bridge will ever ask it again.
   */
  async reread(port: number): Promise<void> {
    await Promise.all([
      this.claims.again(port),
      this.patch.again(port),
      this.conversation.again(port),
    ]);
  }

  /**
   * Whichever of the two is showing a failure, read again. A reconnection.
   *
   * **Not `reread`, and the difference is the megabyte.** These two are the
   * reads no event refreshes, for the reason at the top of this file — so a
   * good reading of either cannot have gone stale while the socket was down,
   * and taking it again would spend the bytes the split exists to save on every
   * reconnection. A *failed* reading is different: it is a surface a person
   * opened while Fleet was not answering, and nothing else in Bridge will ever
   * take it again. #472.
   */
  async repair(port: number): Promise<void> {
    await Promise.all([
      this.claims.failing ? this.claims.again(port) : undefined,
      this.patch.failing ? this.patch.again(port) : undefined,
      this.conversation.failing ? this.conversation.again(port) : undefined,
    ]);
  }
}

/**
 * Hand the comments a person picked to a Drone.
 *
 * **A fifth act at the same gate, and not a fifth `Decision`.** The four above
 * differ in what happens to the Job and each carries at most a note; this one
 * carries a set of handles off a forge, and folding it into `decide` would make
 * that function's body mean two shapes.
 *
 * It answers with the Job as it now stands, like the four, so the caller folds
 * one row rather than re-reading the board.
 */
export function takeUp(port: number, jobId: string, remarks: string[]): Promise<Answer> {
  return ask(port, "POST", `/jobs/${encodeURIComponent(jobId)}/take_up_remarks`, { remarks });
}

/**
 * Send one decision on the work. **The route is the whole of the difference** —
 * three of the four carry no body, and the other carries the reviewer's own
 * words, which is the one string on this seam Fleet does not assemble.
 *
 * All four answer with the Job as it now stands, so the caller folds one row
 * rather than re-reading the board.
 */
export function decide(
  port: number,
  jobId: string,
  decision: Decision,
  note?: string,
): Promise<Answer> {
  const path = `/jobs/${encodeURIComponent(jobId)}/${decision}`;
  return note === undefined
    ? ask(port, "POST", path)
    : ask(port, "POST", path, { note });
}
