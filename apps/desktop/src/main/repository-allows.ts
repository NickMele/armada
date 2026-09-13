// Repository-wide always-allow rules — Fleet's own table since protocol
// 13.5, read for the Manifest surface and removable there. #836.
//
// Beside `editing.ts` rather than inside `checkout-runs.ts`: neither a run
// nor the file is involved, so `port()` and the pick are what a route under
// `/manifest` needs here, `editing.ts`'s own shape.

import type { RemoveRepositoryAllowedCommand, RepositoryAllowedCommands } from "@armada/protocol";
import type { RepositoryAllowedCommandsRead } from "@armada/screens/src/manifest-allows";

import type { Picked } from "./picked";
import { ask, NOT_SET_UP } from "./request";

/** `get_repository_allowed_commands` and `remove_repository_allowed_command`. */
export class RepositoryAllowsCommands {
  private readonly port: () => number | null;
  private readonly picked: Picked;

  constructor(port: () => number | null, picked: Picked) {
    this.port = port;
    this.picked = picked;
  }

  /** Every rule a person always-allowed for this repository, oldest first. */
  async list(): Promise<RepositoryAllowedCommandsRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.picked.manifest("/manifest/allowed_commands");
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, commands: answer.body as RepositoryAllowedCommands };
  }

  /**
   * Take back one rule. Every job against this repository stops being
   * granted it from the next spawn on.
   */
  async remove(run: string): Promise<RepositoryAllowedCommandsRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.picked.manifest("/manifest/allowed_commands/remove");
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const body: RemoveRepositoryAllowedCommand = { run };
    const answer = await ask(port, "POST", path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, commands: answer.body as RepositoryAllowedCommands };
  }
}
