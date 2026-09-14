// Fleet's four limits: drones at once, the memory and disk Fleet keeps free
// before starting another, and how many of a job's checks run at once.
//
// Beside `command.ts` rather than inside it, `clearing.ts`'s reason: it named
// no Job and had reached the length the gate warns about, and this is a seam
// rather than a cut made for the number — `JobCommands` delegates here in a
// line, the way `settleWork` delegates to `decide`.
//
// It holds no connection. The `Board` it is handed is what an act needs, so
// nothing here can drift from what the socket believes.

import type { FleetLimits, Outcome, SaveLimits } from "@armada/protocol";
import { ask } from "./request";
// Type-only, and therefore not a cycle at runtime: `Board` is what an act
// needs of the connection, and it is declared where the acts are.
import type { Board } from "./command";

/**
 * One save at a time, out. **A flag rather than a set**: there is no Job to
 * key it on, and the panel sending this is one screen whose three rows are
 * one act's worth of controls — a second guard would only ever refuse a press
 * the panel never sends.
 */
export class Limits {
  private readonly board: Board;
  private saving = false;

  constructor(board: Board) {
    this.board = board;
  }

  /**
   * Change one or more of the four. **Fleet-wide, and never a Job's act.**
   * Applies the next time a Job is ready to start; nothing running stops. The
   * status bar's count and its hold both read `/capacity`, so a save re-reads
   * it rather than guessing what changed.
   */
  async save(values: SaveLimits): Promise<Outcome> {
    if (this.saving) return { ok: false, why: "already_setting" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    this.saving = true;
    try {
      const answer = await ask(port, "POST", "/limits/save", values);
      if (answer.ok !== true) return answer.outcome;
      this.board.publish({ limits: answer.body as FleetLimits });
      await this.board.rereadCapacity(port);
      return { ok: true };
    } finally {
      this.saving = false;
    }
  }
}
