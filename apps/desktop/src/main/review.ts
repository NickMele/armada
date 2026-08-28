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

import type { Diff, Evidence, Outcome } from "../shared/bridge";
import type { JobDiff, JobEvidence } from "../shared/work";
import { ask, type Answer } from "./request";

/** What each act is called on the route table. `crates/api/src/routes.rs`. */
export type Decision = "approve_review" | "request_changes" | "reject";

/**
 * One Job's claims and one Job's diff, each read when a surface asks for it.
 *
 * **Two ids, not one.** A surface may want the claims without the patch — the
 * record of a finished Job does — and folding them into one field would make
 * that surface pay for bytes it does not draw.
 */
export class ReviewMaterial {
  /** The Job whose claims are wanted. `null` is no read. */
  private claiming: string | null = null;
  /** The Job whose patch is wanted. `null` is no read. */
  private showing: string | null = null;
  private readonly publish: (change: { evidence?: Evidence; diff?: Diff }) => void;

  constructor(publish: (change: { evidence?: Evidence; diff?: Diff }) => void) {
    this.publish = publish;
  }

  /** Both reads end with the window. Neither is written onto the Job. */
  close(): void {
    this.claiming = null;
    this.showing = null;
  }

  /** Read what one Job's Drones claimed, or `null` to stop. */
  async evidence(port: number | null, jobId: string | null): Promise<void> {
    this.claiming = jobId;
    if (jobId === null) {
      this.publish({ evidence: { state: "none" } });
      return;
    }
    this.publish({ evidence: { state: "reading", jobId } });
    if (port === null) {
      this.publish({ evidence: failed(jobId) });
      return;
    }
    await this.readClaims(port);
  }

  /** Read one Job's worktree against its branch, or `null` to stop. */
  async diff(port: number | null, jobId: string | null): Promise<void> {
    this.showing = jobId;
    if (jobId === null) {
      this.publish({ diff: { state: "none" } });
      return;
    }
    this.publish({ diff: { state: "reading", jobId } });
    if (port === null) {
      this.publish({ diff: failed(jobId) });
      return;
    }
    await this.readWork(port);
  }

  /** Both again, for whichever is open. The bar's Refresh reaches this. */
  async reread(port: number): Promise<void> {
    await Promise.all([this.readClaims(port), this.readWork(port)]);
  }

  /**
   * `GET /jobs/:job_id/evidence`, published whole.
   *
   * **An empty list is `read`, not `none`.** No step has submitted anything is
   * a fact about the Job; nobody asked is a fact about the window. A surface
   * that could not tell them apart would say a Drone reported nothing when what
   * is true is that nothing was read.
   */
  private async readClaims(port: number): Promise<void> {
    const jobId = this.claiming;
    if (jobId === null) return;
    const answer = await ask(port, "GET", `/jobs/${encodeURIComponent(jobId)}/evidence`);
    // The open Job changed mid-read: nobody has this answer's Job open.
    if (this.claiming !== jobId) return;
    if (answer.ok !== true) {
      this.publish({ evidence: { state: "failed", jobId, outcome: answer.outcome } });
      return;
    }
    const read = answer.body as JobEvidence;
    this.publish({ evidence: { state: "read", jobId, steps: read.steps } });
  }

  /**
   * `GET /jobs/:job_id/diff`, published whole.
   *
   * **`work` stays optional all the way through.** Absent is a Job with no
   * worktree to read; present with an empty `files` is a Drone that changed
   * nothing, which is what fails a `diff_nonempty` check. Filling in an empty
   * reading here would erase the difference before any surface saw it.
   */
  private async readWork(port: number): Promise<void> {
    const jobId = this.showing;
    if (jobId === null) return;
    const answer = await ask(port, "GET", `/jobs/${encodeURIComponent(jobId)}/diff`);
    if (this.showing !== jobId) return;
    if (answer.ok !== true) {
      this.publish({ diff: { state: "failed", jobId, outcome: answer.outcome } });
      return;
    }
    const read = answer.body as JobDiff;
    this.publish({ diff: { state: "read", jobId, work: read.work } });
  }
}

/**
 * Send one decision on the work. **The route is the whole of the difference** —
 * two of the three carry no body, and the third carries the reviewer's own
 * words, which is the one string on this seam Fleet does not assemble.
 *
 * All three answer with the Job as it now stands, so the caller folds one row
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

/** Not connected, in the shape both reads publish it. */
function failed(jobId: string): { state: "failed"; jobId: string; outcome: Outcome } {
  return { state: "failed", jobId, outcome: { ok: false, why: "not_connected" } };
}
