// One Job's work, reviewed, and the two collection-wide reads beside it.
//
// Pulled out of `connection.ts` so that a route added here is a line in this
// file rather than another few lines on the file the gate measures. Every
// method is the same body it always was, minus the `this.connected()` port
// lookup — `port` arrives as a function so this file never learns how a port
// is found, only that one might not be.

import type { CallRead, CheckOutputRead, FrameRead } from "@armada/protocol";
import type { HeldReader } from "./holding";
import { callArgumentsOf, checkOutputOf, frameOf } from "./request";
import type { ReportsReader } from "./reports";
import type { ReviewMaterial } from "./review";

export type JobReadsWiring = {
  port: () => number | null;
  material: ReviewMaterial;
  reports: ReportsReader;
  held: HeldReader;
};

export class JobReads {
  private readonly wiring: JobReadsWiring;

  constructor(wiring: JobReadsWiring) {
    this.wiring = wiring;
  }

  // ------------------------------------------------- one Job's work, reviewed
  // The three reads. What each one is and why it is its own entry is in
  // `review.ts`; this holds the port the reads are made over, which is the
  // only part that belongs to the connection.

  /** What one Job's Drones claimed. The cheap half of the pair. */
  async readEvidence(jobId: string | null): Promise<void> {
    await this.wiring.material.evidence(this.wiring.port(), jobId);
  }

  /**
   * One Job's worktree against its branch. **The expensive half, and the only
   * place the patch bytes are spent** — called by the surface that draws a diff
   * rather than by opening a Job, which is the separation
   * `crates/adapter-traits/src/work_product.rs` records.
   */
  async readDiff(jobId: string | null): Promise<void> {
    await this.wiring.material.diff(this.wiring.port(), jobId);
  }

  /**
   * What people wrote on one Job's pull request. **The one read here that costs
   * a process on the machine Fleet is on and a network beyond it**, which is
   * why it is opened by the surface a person decides on and by nothing else.
   */
  async readRemarks(jobId: string | null): Promise<void> {
    await this.wiring.material.remarks(this.wiring.port(), jobId);
  }

  /**
   * One recorded call's arguments — the rest of a row the socket cut.
   *
   * **It answers the caller and publishes nothing.** Every read above is held
   * because the thing it draws moves; a recorded argument is finished, and it
   * is one reader's gesture on one row rather than state the window renders
   * from. Nothing connected is the caller's to say, so it comes back as the
   * refusal every other operation here uses rather than as silence.
   */
  async readCall(jobId: string, callId: string): Promise<CallRead> {
    const port = this.wiring.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    return await callArgumentsOf(port, jobId, callId);
  }

  /**
   * One Check's own output, for the person who opened that Check.
   *
   * **`readCall`'s shape, for `readCall`'s reasons.** A recorded output does
   * not move, so nothing here is held or republished; `kept` is the row's own
   * file name and Fleet resolves it against its record, so this passes it
   * through and composes nothing.
   */
  async readCheckOutput(jobId: string, kept: string): Promise<CheckOutputRead> {
    const port = this.wiring.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    return await checkOutputOf(port, jobId, kept);
  }

  /**
   * One frame a step's harness produced, for the person who opened it.
   *
   * **`readCheckOutput`'s shape, and the bytes stop here.** What crosses to the
   * renderer is an array and a media type; the renderer makes a `Blob` of it
   * and never learns Fleet's port. That is the rule every read on this seam
   * follows — main talks to Fleet, the renderer talks to main — and a frame is
   * fetched through it rather than put in an `img` tag pointed at a port, so
   * the one surface that draws a file is not also the one that opens a socket.
   */
  async readFrame(jobId: string, kept: string): Promise<FrameRead> {
    const port = this.wiring.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    return await frameOf(port, jobId, kept);
  }

  // ----------------------------------------------- every report, and the counts
  /**
   * Read every filed report, or drop what was read. **The one read here that no
   * Job scopes** — a report is about a Job and does not belong to one, so a
   * listing reached through a Job would lose the ones that outlived theirs.
   * `reports.ts` holds the read; this holds the port it is made over.
   */
  async readReports(want: boolean): Promise<void> {
    await this.wiring.reports.want(this.wiring.port(), want);
  }

  // ------------------------------------------- what Fleet is holding disk for
  /**
   * Read what Fleet is holding, or drop it. **The second read here no Job
   * scopes**, and for a different reason from the reports: this one is a
   * question about the set — which of these to give back — which no per-Job
   * field could be asked.
   */
  async readHeld(want: boolean): Promise<void> {
    await this.wiring.held.want(this.wiring.port(), want);
  }

  /**
   * Read it again, after something that changes what is held.
   *
   * **Nothing folds a reclaim's receipt into the list.** That answer says what
   * happened to one Job; whether the row is gone is Fleet's reading, and a row
   * whose checkout would not go has to stay. A no-op where nobody has the
   * surface open.
   */
  async rereadHeld(): Promise<void> {
    const port = this.wiring.port();
    if (port !== null) await this.wiring.held.again(port);
  }
}
