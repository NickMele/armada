// Journey 9's run sheet, on a real Job: start and stop a rehearsal, and read
// its output as it prints.
//
// Beside `connection.ts` rather than inside it, for `command.ts`'s reason: a
// POST under a Job and a socket that only follows one run's output are
// neither the socket, the runtime file nor the state machine.
//
// **A rehearsal, never a verdict.** Nothing here writes Evidence or moves a
// Job — `start_run` answers with the run underway and `stop_run` with its
// finished record, and what changed is only ever the sheet's own reading.

import WebSocket from "ws";

import type { NamedRun, RunFollowed, RunMessage, RunUnderway, StartRun } from "@armada/protocol";
import type { Outcome } from "@armada/protocol";
import { ask, route } from "./request";
import { HOST } from "./runtime-file";

/** What starting and stopping a run needs of the connection, and no more. */
export type RunBoard = {
  port: () => number | null;
  /** Follow this run's output from the instant it answers as underway. */
  follow: (port: number, jobId: string, runId: string) => void;
  /** Read the sheet again — its own `running` field is what moved. */
  refreshSheet: (port: number) => Promise<void>;
};

/** `start_run` and `stop_run`. **`undo_run` and `list_runs` are not here** — the
 * sheet does not yet draw a run's history or its changed files, so there is
 * nothing on this side of the wire to call them from. Reported. */
export class RunCommands {
  private readonly board: RunBoard;

  constructor(board: RunBoard) {
    this.board = board;
  }

  /**
   * Run one Check or Command in this Job's own worktree. **Answers at once,
   * with the run underway** — Fleet takes a snapshot first and the output
   * streams on `observe_run`, which this opens the moment the run exists.
   */
  async startRun(jobId: string, body: StartRun): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const answer = await ask(port, "POST", route(jobId, "start_run"), body);
    if (answer.ok !== true) return answer.outcome;
    const running = answer.body as RunUnderway;
    this.board.follow(port, jobId, running.id);
    await this.board.refreshSheet(port);
    return { ok: true };
  }

  /** End a run's process group. The log keeps what printed. */
  async stopRun(jobId: string, id: string): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const body: NamedRun = { id };
    const answer = await ask(port, "POST", route(jobId, "stop_run"), body);
    if (answer.ok !== true) return answer.outcome;
    await this.board.refreshSheet(port);
    return { ok: true };
  }
}

/**
 * One run's output, as a window is watching it. **`FollowSocket`'s shape, one
 * subject over** — a run's own id in place of a Check's `kept` — and one at a
 * time for the same reason: the sheet shows one run, and opening another
 * replaces it.
 */
export class RunSocket {
  private readonly publish: (followed: RunFollowed) => void;
  private socket: WebSocket | null = null;
  private held: RunFollowed = { state: "none" };

  constructor(publish: (followed: RunFollowed) => void) {
    this.publish = publish;
  }

  /** Which run is being read, or `null` for either to stop. */
  open(port: number | null, jobId: string | null, runId: string | null): void {
    const held = this.held;
    if (
      jobId !== null &&
      runId !== null &&
      held.state !== "none" &&
      held.state !== "failed" &&
      held.jobId === jobId &&
      held.runId === runId
    ) {
      return;
    }
    this.close();
    if (jobId === null || runId === null) {
      this.set({ state: "none" });
      return;
    }
    if (port === null) {
      this.set({ state: "failed", jobId, runId, detail: "Fleet is not connected." });
      return;
    }
    this.set({ state: "opening", jobId, runId });

    const path = `/jobs/${encodeURIComponent(jobId)}/runs/${encodeURIComponent(runId)}/observe`;
    const socket = new WebSocket(`ws://${HOST}:${port}${path}`);
    this.socket = socket;
    socket.on("message", (data: WebSocket.RawData) => this.arrived(jobId, runId, String(data)));
    socket.on("error", (cause: Error) => this.broke(jobId, runId, cause.message));
    socket.on("close", () => this.broke(jobId, runId, "the connection closed"));
  }

  close(): void {
    const socket = this.socket;
    this.socket = null;
    if (socket === null) return;
    socket.removeAllListeners();
    // One listener stays, `following.ts`'s reason: a socket still shaking
    // hands reports its abort as an `error` on the next tick, and one with
    // nothing listening is thrown by Node.
    socket.on("error", () => {});
    socket.close();
  }

  private set(followed: RunFollowed): void {
    this.held = followed;
    this.publish(followed);
  }

  private arrived(jobId: string, runId: string, text: string): void {
    let message: RunMessage;
    try {
      message = JSON.parse(text) as RunMessage;
    } catch {
      this.broke(jobId, runId, "Fleet sent a message this Bridge could not read.");
      return;
    }

    if (message.message === "opened") {
      this.set({
        state: "following",
        jobId,
        runId,
        name: message.name,
        path: message.path,
        fromLine: message.skipped + 1,
        lines: [],
      });
      return;
    }

    const held = this.held;
    if (held.state !== "following") return;

    if (message.message === "lines") {
      this.set({ ...held, lines: [...held.lines, ...message.lines] });
      return;
    }
    if (message.message === "missed") return;

    // `closed` carries why, and the socket is let go first so its own `close`
    // cannot overwrite the reason.
    this.close();
    this.set({ ...held, ended: message.because });
  }

  /** The socket went without a sentence. What had arrived stays arrived. */
  private broke(jobId: string, runId: string, detail: string): void {
    if (this.socket === null) return;
    this.socket.removeAllListeners();
    this.socket = null;
    const held = this.held;
    if (held.state === "following") {
      this.set({ ...held, ended: held.ended ?? "broke" });
      return;
    }
    this.set({ state: "failed", jobId, runId, detail });
  }
}
