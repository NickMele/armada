// One per-Job read, and the rule that keeps one Job's answer off another Job's
// panel.
//
// Four reads were the same twenty lines: hold the id a surface asked for, GET
// the route under it, **drop the answer if it is not the one to publish**,
// publish read-or-failed. That third clause is a correctness rule and four
// copies of it meant a fifth read would be written by copying one of them.
// Written once here, it is the one place to look for it.
//
// **It is two rules and was one.** An answer is dropped where the id moved
// under it, which stops one Job's read painting another; and where a newer read
// of the same Job was begun after it, which stops the slower of two answers
// being the one a surface keeps. The second was missing, and the events that
// collide are ordinary — a step boundary publishes two about twenty
// milliseconds apart.
//
// Beside `connection.ts` rather than inside it, for the reason `command.ts`,
// `request.ts` and `review.ts` are: the connection is a socket, a runtime file
// and a state machine, and a GET under a Job is none of those. No port is held,
// only passed in, so nothing here can drift from what the socket believes.

import type { JobRead } from "@armada/protocol";
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
   * **The two reads an event re-takes: the open Job's detail and what it holds
   * on this machine.** Both are re-read on every event naming their Job, so one
   * timed-out read would blank a rail or a panel mid-run. The rest are read
   * when a surface asks and not again, for a Job that has stopped, where a
   * failure is all there is to say. A first read that fails has nothing to keep
   * and says so either way. Resources joined the detail in #438.
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
  /**
   * How many reads this reader has begun. **The newest is the only one whose
   * answer may be published.**
   *
   * A counter and not a flag, because the question is not whether a read is in
   * flight — it is which of the ones in flight was asked for last. Two reads of
   * one Job overlap freely and their answers may land in either order, so
   * without this the screen keeps whichever was slowest rather than whichever
   * was newest. It never resets: a reader that wrapped would let an old answer
   * match a new number, and this counts events on one window.
   */
  private asked = 0;

  constructor(reads: Reads<Read>) {
    this.reads = reads;
  }

  /** The Job this read is for, or `null`. */
  get jobId(): string | null {
    return this.open;
  }

  /**
   * Whether what a surface is showing for this read is a failure.
   *
   * **What the screen says, not what the last attempt did.** A read carrying
   * `keepsLastGood` answers `false` where an attempt failed and the previous
   * answer is still up, because that panel has a reading and does not need
   * repairing. This is asked on a reconnection, of the reads nothing else takes
   * again, so that only the ones actually stuck are re-read. #472.
   */
  get failing(): boolean {
    return this.last.state === "failed";
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

  /**
   * Read again, for whatever Job is held. Nothing held is no read.
   *
   * **Two answers to two questions that look like one.** The id moving is one
   * failure and a second read of the same Job overtaking the first is another,
   * and the id check alone catches only the first — which is what let an
   * eighteen-minute-old spend survive the event that should have corrected it,
   * on Job `01M21BKVPW002DC0ATD1X9T0VF`.
   *
   * **Every caller is `void`-ed and there are seven of them.** A step boundary
   * publishes a Drone exiting and a step advancing about twenty milliseconds
   * apart, so the two overlap by construction rather than under load — and
   * nothing here can await the other's answer, because they are separate
   * events arriving on a socket.
   */
  async again(port: number): Promise<void> {
    const jobId = this.open;
    if (jobId === null) return;
    this.asked += 1;
    const asked = this.asked;
    const answer = await ask(port, "GET", this.reads.route(jobId));
    // **The id moved while the request was in flight.** Nobody has this
    // answer's Job open, so publishing it would paint the Job that replaced it.
    if (this.open !== jobId) return;
    // **A newer read was begun while this one was in flight.** Its answer is
    // the one a surface should keep, whichever of the two the network returns
    // first — so this one is dropped rather than raced, including its failure:
    // a stale timeout must not blank a panel the newer read is about to fill.
    if (this.asked !== asked) return;
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
