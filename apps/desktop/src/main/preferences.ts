// A person's Bridge preferences: loaded when Fleet connects, saved on
// change, and published in `BridgeState`. `limits.ts`'s shape one table over
// — beside it for the same reason, and its own class for the same reason:
// what an act needs is `Board`, so nothing here can drift from what the
// socket believes.
//
// **While Fleet is unreachable, Bridge draws the shipped default and queues
// nothing.** There is no `saving` flag held across a lost connection for a
// press to resume — a save that could not reach Fleet is a save that did not
// happen, and the panel says so the way every other act here does.

import type { Outcome, Preferences, SavePreference } from "@armada/protocol";
import { ask } from "./request";
// Type-only, and therefore not a cycle at runtime: `Board` is what an act
// needs of the connection, and it is declared where the acts are.
import type { Board } from "./command";

/**
 * One save at a time, out. **A flag rather than a set**, `Limits`' reason:
 * there is no Job to key it on, and the panel sending this is one control.
 */
export class Preferring {
  private readonly board: Board;
  private saving = false;

  constructor(board: Board) {
    this.board = board;
  }

  /**
   * Save one preference by name. **Fleet-wide, and never a Job's act.**
   *
   * `save.value`'s type is fixed here at `boolean` because that is what the
   * one preference this build has is — a second preference of another shape
   * changes this signature rather than widening it on spec.
   */
  async save(save: SavePreference): Promise<Outcome> {
    if (this.saving) return { ok: false, why: "already_setting" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    this.saving = true;
    try {
      const answer = await ask(port, "POST", "/preferences/save", save);
      if (answer.ok !== true) return answer.outcome;
      this.board.publish({ preferences: answer.body as Preferences });
      return { ok: true };
    } finally {
      this.saving = false;
    }
  }
}
