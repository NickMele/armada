// Clearing up after a Job that has ended: its record, and its disk.
//
// Beside `command.ts` rather than inside it, for the reason `proposing.ts` and
// `review.ts` are: that file is the acts that carry a Job forward, each of them
// one POST answering with a row to fold, and neither of these is that. It had
// reached the length the gate warns about, and this is a seam rather than a cut
// made for the number — `JobCommands` delegates here in a line, the way
// `settleWork` delegates to `decide`.
//
// # The two acts are two, and stay two
//
// `crates/ipc/operations.toml` splits them and argues the split on its
// `forget_job` row: one call with two unrelated things to fail at is worse than
// two calls, and a person clearing a Board should not also have to think about
// a directory. So neither one's outcome depends on the other and a caller
// wanting both sends both — which is why they belong in one file and not in one
// function.
//
// **Neither goes through `act`.** That helper folds a `JobSummary` and re-reads
// the open Job. A forget leaves no row to fold; a reclaim leaves the row
// untouched and answers with a receipt instead.
//
// It holds no connection. The `Board` it is handed is what any act needs, so
// nothing here can drift from what the socket believes.

import type { ClearOutcome, Outcome, WorktreeReclaimed } from "@armada/protocol";
import { ask, route } from "./request";
// Type-only, and therefore not a cycle at runtime: `Board` is what an act needs
// of the connection, and it is declared where the acts are.
import type { Board } from "./command";

/**
 * The record and the disk, and which of each is in flight.
 *
 * **Two sets, not one.** The two acts are halves of one clearing-up and a
 * person may well send both on the same Job, so a single set would refuse the
 * second press for being the first.
 */
export class Clearing {
  private readonly board: Board;
  private readonly forgetting = new Set<string>();
  private readonly reclaiming = new Set<string>();

  constructor(board: Board) {
    this.board = board;
  }

  /**
   * Clear every terminal Job at once. **One `forget_job` per id, sent in
   * turn** — there is no bulk route on the wire, and each Job is forgotten
   * independently, so a status that moved between the press and this call
   * (or an id already gone) does not stop the rest.
   *
   * **The caller decides which ids are terminal.** This sends exactly what it
   * is given; Fleet's 409 is the safety net, not the gate.
   */
  async clearTerminal(jobIds: readonly string[]): Promise<ClearOutcome> {
    const cleared: string[] = [];
    const failed: { jobId: string; outcome: Outcome }[] = [];
    for (const jobId of jobIds) {
      const outcome = await this.forget(jobId);
      if (outcome.ok) cleared.push(jobId);
      else failed.push({ jobId, outcome });
    }
    return { cleared, failed };
  }

  /**
   * Give one terminal Job's worktree and branch back, while Fleet is running.
   *
   * **The Job is untouched.** What was reclaimed is disk and not the record, so
   * the row stays exactly where it is on the board and the answer is a receipt
   * rather than a Job. `clearTerminal` above is the other half and takes the
   * row.
   *
   * **A kept branch comes back as a success.** Fleet always runs this with the
   * safe setting and there is no force on this seam, so a branch holding
   * commits the base cannot reach survives and says so — which is a true answer
   * to the act, not a refusal of it. The caller reads `reclaimed.branch` to
   * find out which half happened.
   */
  async reclaim(jobId: string): Promise<Outcome> {
    if (this.reclaiming.has(jobId)) return { ok: false, why: "already_reclaiming" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };

    this.reclaiming.add(jobId);
    try {
      const answer = await ask(port, "POST", route(jobId, "reclaim_worktree"));
      if (answer.ok !== true) return answer.outcome;
      return { ok: true, reclaimed: answer.body as WorktreeReclaimed };
    } finally {
      this.reclaiming.delete(jobId);
    }
  }

  /**
   * Delete one terminal Job's whole record.
   *
   * **Private, because nothing sends one.** Clearing a board is a set and the
   * bulk shape above is the only caller — a single-id entry point would be a
   * second way in to the one act on this seam that cannot be undone.
   *
   * `board.forget` is what actually removes the row; `job.forgotten` on the
   * stream does the same thing for a window that did not make the call itself.
   */
  private async forget(jobId: string): Promise<Outcome> {
    if (this.forgetting.has(jobId)) return { ok: false, why: "already_forgetting" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };

    this.forgetting.add(jobId);
    try {
      const answer = await ask(port, "POST", route(jobId, "forget_job"));
      if (answer.ok !== true) return answer.outcome;
      this.board.forget(jobId);
      return { ok: true };
    } finally {
      this.forgetting.delete(jobId);
    }
  }
}
