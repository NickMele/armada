// Repository-wide always-allow rules — Fleet's own table since protocol
// 13.5, read for the Manifest surface and removable there. #836.
//
// Beside `editing.ts` rather than inside `checkout-runs.ts`: neither a run
// nor the file is involved, so `port()` alone is what a route under
// `/manifest` needs here, `editing.ts`'s own shape.

import type { RemoveRepositoryAllowedCommand, RepositoryAllowedCommands } from "@armada/protocol";
import type { RepositoryAllowedCommandsRead } from "@armada/screens/src/manifest-allows";

import { ask } from "./request";

/** `get_repository_allowed_commands` and `remove_repository_allowed_command`. */
export class RepositoryAllowsCommands {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Every rule a person always-allowed for this repository, oldest first. */
  async list(): Promise<RepositoryAllowedCommandsRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "GET", "/manifest/allowed_commands");
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
    const body: RemoveRepositoryAllowedCommand = { run };
    const answer = await ask(port, "POST", "/manifest/allowed_commands/remove", body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, commands: answer.body as RepositoryAllowedCommands };
  }
}
