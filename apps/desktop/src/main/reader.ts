// One per-Job read, and the rule that keeps one Job's answer off another Job's
// panel.
//
// Four reads were the same twenty lines: hold the id a surface asked for, GET
// the route under it, **drop the answer if the id moved while it was in
// flight**, publish read-or-failed. That third clause is a correctness rule —
// it is what stops a read for one Job painting the Job that replaced it — and
// four copies of it meant a fifth read would be written by copying one of them.
// Written once here, it is the one place to look for it.
//
// Beside `connection.ts` rather than inside it, for the reason `command.ts`,
// `request.ts` and `review.ts` are: the connection is a socket, a runtime file
// and a state machine, and a GET under a Job is none of those. No port is held,
// only passed in, so nothing here can drift from what the socket believes.

import type { JobRead } from "../shared/bridge";
import { ask } from "./request";

/** What one read is: where to send, what to keep, and where the state goes. */
export type Reads<Read> = {
  /** The route under one Job. The id is a path segment, so it is encoded here. */
  route: (jobId: string) => string;
  /** What of the answered body the `read` state carries. */
  keeps: (body: unknown) => Read;
  /** Where each state goes, `none` included. Nothing else publishes this read. */
  publish: (state: JobRead<Read>) => void;
  /**
   * Whether a failed read keeps the last good answer for the same Job.
   *
   * **The open Job's detail alone.** It is re-read on every event naming its
   * Job, so one timed-out read would blank a rail mid-run; the other three are
   * read once for a Job that has stopped, where a failure is all there is to
   * say. A first read that fails has nothing to keep and says so either way.
   */
  keepsLastGood?: boolean;
};

/**
 * One route under one Job, read and re-read for as long as a surface wants it.
 *
 * **The id is held here rather than passed to each read**, because an answer
 * has to be checked against the id that is current when it lands and not the
 * one that was current when it was asked for.
 */
export class JobReader<Read> {
  private readonly reads: Reads<Read>;
  /** The Job this read is for. `null` is no read. */
  private open: string | null = null;
  /** The last state published, which is what a kept answer is kept from. */
  private last: JobRead<Read> = { state: "none" };

  constructor(reads: Reads<Read>) {
    this.reads = reads;
  }

  /** The Job this read is for, or `null`. */
  get jobId(): string | null {
    return this.open;
  }

  /** Read one Job, or `null` to stop. Nothing connected is a failure to draw. */
  async want(port: number | null, jobId: string | null): Promise<void> {
    this.open = jobId;
    if (jobId === null) {
      this.say({ state: "none" });
      return;
    }
    this.say({ state: "reading", jobId });
    if (port === null) {
      this.say({ state: "failed", jobId, outcome: { ok: false, why: "not_connected" } });
      return;
    }
    await this.again(port);
  }

  /** Read again, for whatever Job is held. Nothing held is no read. */
  async again(port: number): Promise<void> {
    const jobId = this.open;
    if (jobId === null) return;
    const answer = await ask(port, "GET", this.reads.route(jobId));
    // **The id moved while the request was in flight.** Nobody has this
    // answer's Job open, so publishing it would paint the Job that replaced it.
    if (this.open !== jobId) return;
    if (answer.ok !== true) {
      const held = this.last;
      if (this.reads.keepsLastGood === true && held.state === "read" && held.jobId === jobId) {
        return;
      }
      this.say({ state: "failed", jobId, outcome: answer.outcome });
      return;
    }
    this.say({ state: "read", jobId, ...this.reads.keeps(answer.body) });
  }

  /** The read ends with the window. Nothing is published: the surface is gone. */
  close(): void {
    this.open = null;
  }

  private say(state: JobRead<Read>): void {
    this.last = state;
    this.reads.publish(state);
  }
}
