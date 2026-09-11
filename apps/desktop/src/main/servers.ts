// Start and stop a server the Manifest declares — for a Job's worktree, or,
// with no Job, the main checkout. `docs/concepts/fleet.md`, *Servers*.
//
// Beside `connection.ts` rather than inside it, `rehearsal.ts`'s reason: a
// POST is none of the socket, the runtime file or the state machine. The
// list itself is read once per connection and kept current by folding
// `server.*` off `/events`, which is `connection.ts`'s own job — nothing
// here holds the list.

import { shell } from "electron";

import type { Followed, NamedServer, Outcome, StartServer } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { ask } from "./request";

export type ServerBoard = {
  port: () => number | null;
};

export class ServerCommands {
  private readonly board: ServerBoard;

  constructor(board: ServerBoard) {
    this.board = board;
  }

  /**
   * Start a declared server, for a Job's worktree or the main checkout.
   * **Answers at once, `starting`** — `server.serving` and `server.exited`
   * follow on `/events`, which is what keeps the list current without a
   * second read here.
   */
  async startServer(name: string, jobId?: string): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const body: StartServer = jobId === undefined ? { name } : { name, job_id: jobId };
    const answer = await ask(port, "POST", "/servers/start", body);
    return answer.ok === true ? { ok: true } : answer.outcome;
  }

  /** End a server's process group. Its log keeps what printed. */
  async stopServer(id: string): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const body: NamedServer = { id };
    const answer = await ask(port, "POST", "/servers/stop", body);
    return answer.ok === true ? { ok: true } : answer.outcome;
  }
}

/**
 * Open one link a server is offering, in whatever browses the web on this
 * machine. **No surface in Bridge navigates**, the design system's hard rule
 * — this is the OS opening it, never the window.
 *
 * **The address is checked against what main published, `forge.ts`'s rule
 * for a pull request's.** A server's link carries no id of its own to look
 * up by, so what closes the gap here is checking the value the renderer sent
 * against the links main is already holding for that server, rather than
 * trusting a string a click handler composed.
 *
 * **`http:` is allowed, where `forge.ts` refuses it.** A server's link is a
 * local address Fleet resolved from a declared port — never a forge — so the
 * scheme test admits both loopback schemes and refuses everything else.
 */
export async function openServerLink(
  state: BridgeState,
  serverId: string,
  url: string,
): Promise<Followed> {
  const server = state.servers.servers.find((row) => row.id === serverId);
  const known = server?.links.some((link) => link.url === url) ?? false;
  if (!known) return { ok: false, why: "no_address" };
  if (!addressable(url)) return { ok: false, why: "not_addressable", address: url };

  try {
    await shell.openExternal(url);
    return { ok: true };
  } catch (error) {
    return { ok: false, why: "refused", address: url, detail: String(error) };
  }
}

function addressable(address: string): boolean {
  try {
    const scheme = new URL(address).protocol;
    return scheme === "http:" || scheme === "https:";
  } catch {
    return false;
  }
}
