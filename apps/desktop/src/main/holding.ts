// What Fleet is holding disk for, read while somebody is deciding about it.
//
// Beside `connection.ts` rather than inside it, for `reports.ts`'s reason: the
// connection is a socket, a runtime file and a state machine, and a GET is none
// of those. No port is held, only passed in, so nothing here can drift from
// what the socket believes.
//
// # It is not a `JobReader`, and that is the whole shape of it
//
// Every read in `reader.ts` is scoped to a job, and the rule that file exists
// for is that an answer whose job moved while it was in flight is dropped
// rather than painted onto the job that replaced it. There is no id here for
// that rule to check: what is being decided is which of a set to give back, and
// that is a question about the set. What survives of the rule is the half that
// still applies — an answer landing after the surface closed is dropped.
//
// # Re-read after a reclaim, never folded
//
// Reclaiming changes what is held, and the answer to `reclaim_worktree` says
// only what happened to the one job. Folding that receipt into this list would
// be Bridge deciding a worktree is gone on its own reading; asking again is
// Fleet answering. The list is short, the read is cheap, and the correction is
// the point — a row that fails to go is a row that has to stay.
//
// # This is `reports.ts` again, and knowingly
//
// Two reads on this seam are scoped to nothing and held while a surface is
// open, and they move through the same four states. They are two classes rather
// than one generic because a third does not exist yet and the shape a generic
// would need is uglier than the duplication; the day there is a third, they
// collapse.

import type { HeldWorktrees, WorktreesHeld } from "@armada/protocol";
import { ask } from "./request";

/** One read of `GET /worktrees`, held for as long as a surface wants it. */
export class HeldReader {
  private readonly publish: (held: HeldWorktrees) => void;
  /** Whether anybody is deciding. The only scope this read has. */
  private open = false;

  constructor(publish: (held: HeldWorktrees) => void) {
    this.publish = publish;
  }

  /** Read them, or drop what was read. Nothing connected is a failure to draw. */
  async want(port: number | null, want: boolean): Promise<void> {
    this.open = want;
    if (!want) {
      this.publish({ state: "none" });
      return;
    }
    if (port === null) {
      this.publish({ state: "failed", outcome: { ok: false, why: "not_connected" } });
      return;
    }
    await this.again(port);
  }

  /** Read again, where a surface has one open. Nothing open is no read. */
  async again(port: number): Promise<void> {
    if (!this.open) return;
    this.publish({ state: "reading" });
    const answer = await ask(port, "GET", "/worktrees");
    // The surface closed while this was in flight, so nobody is reading the
    // answer and publishing it would draw a list onto a screen that is gone.
    if (!this.open) return;
    this.publish(
      answer.ok === true
        ? { state: "read", held: answer.body as WorktreesHeld }
        : { state: "failed", outcome: answer.outcome },
    );
  }

  /** The read ends with the window. Nothing is published: the surface is gone. */
  close(): void {
    this.open = false;
  }
}
