// Filing that a Job failed in error, with the reason.
//
// Beside `command.ts` rather than inside it, `clearing.ts`'s reason: it had
// reached the length the gate warns about, and this act is not the shape
// `act` holds — it changes no status, no step and no drone, so folding a Job
// or re-reading the open one would be Bridge redrawing a board on the
// strength of somebody having written a sentence.
//
// It holds no connection. The `Board` it is handed is what an act needs, so
// nothing here can drift from what the socket believes.

import type { FileReport, Outcome, Report } from "@armada/protocol";
import { ask, route } from "./request";
// Type-only, and therefore not a cycle at runtime: `Board` is what an act
// needs of the connection, and it is declared where the acts are.
import type { Board } from "./command";

/** Which Jobs have a report being filed. Its own set: filing is not an act
 *  on the Job, so it is in flight beside any act that is. */
export class Reporting {
  private readonly board: Board;
  private readonly filing = new Set<string>();

  constructor(board: Board) {
    this.board = board;
  }

  /**
   * File this Job's record with the reason it failed in error.
   *
   * Blank is refused before the request is sent, matching the 422 Fleet would
   * give it. What comes back is the report, which the caller shows: Armada
   * files nothing in the issue tracker, so the rendered record is what a
   * person takes to file one there themselves.
   */
  async file(jobId: string, filing: FileReport): Promise<Outcome> {
    if (filing.said.trim() === "") return { ok: false, why: "empty_report" };
    if (this.filing.has(jobId)) return { ok: false, why: "already_reporting" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };

    this.filing.add(jobId);
    try {
      const answer = await ask(port, "POST", route(jobId, "report"), filing);
      if (answer.ok !== true) return answer.outcome;
      const report = answer.body as Report;
      // A body that is not a report is still a filing that happened — Fleet
      // answered 201 — so the outcome is a success carrying nothing rather
      // than a refusal that would send somebody to file it a second time.
      return typeof report?.record === "string" ? { ok: true, report } : { ok: true };
    } finally {
      this.filing.delete(jobId);
    }
  }
}
