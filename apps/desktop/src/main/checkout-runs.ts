// The Manifest surface's half of Journey 9 — run one Check or Command in the
// main checkout, and read its output as it prints.
//
// **Beside `rehearsal.ts` rather than inside it, and it is the same subject
// one owner over.** Every read and every act here answers a route under
// `/manifest`, which resolves no Job: there is no id to hold, no id to check
// an answer against, and no worktree that can be gone. `JobReader` exists
// entirely to do the first two, so nothing here uses it.
//
// **A rehearsal, never a verdict.** Nothing here writes Evidence or moves a
// Job. A run ends with `checkout_run.finished`, which carries the record — and
// all that moves in this window is the sheet's own reading of it.

import WebSocket from "ws";

import type {
  CheckoutRunDiff,
  CheckoutRunFollowed,
  CheckoutRunList,
  CheckoutRunListRead,
  CheckoutRunMessage,
  CheckoutRunSheet,
  CheckoutRunSheetRead,
  CheckoutRunUnderway,
  CheckoutVerify,
  ManifestDrift,
  ManifestDriftRead,
  NamedRun,
  Outcome,
  RunOutput,
  RunOutputRead,
  StartCheckoutRun,
  StartCheckoutVerify,
} from "@armada/protocol";
import type { CheckoutRunDiffRead } from "@armada/protocol";
import type { Picked } from "./picked";
import { ask, NOT_SET_UP } from "./request";
import { HOST } from "./runtime-file";

/**
 * `GET /manifest/run_sheet`, read and re-read while a surface wants it.
 *
 * **`JobReader`'s second rule without its first.** There is no id for an
 * answer to be checked against, so what remains is the rule that a newer read
 * begun after an older one is the only one whose answer may be published —
 * which is not a refinement: `start_run` and `checkout_run.finished` re-read
 * this within milliseconds of each other by construction, and without the
 * counter the surface would keep whichever the network returned slowest.
 */
export class CheckoutSheetReader {
  private readonly publish: (read: CheckoutRunSheetRead) => void;
  private readonly picked: Picked;
  /** Whether a surface still wants it. `false` is no read. */
  private wanted = false;
  /** How many reads this reader has begun; the newest is the only one that publishes. */
  private asked = 0;

  constructor(publish: (read: CheckoutRunSheetRead) => void, picked: Picked) {
    this.publish = publish;
    this.picked = picked;
  }

  /** Whether anything is holding this read open. */
  get open(): boolean {
    return this.wanted;
  }

  /** Hold the read open, or let it go. Nothing connected is a failure to draw. */
  async want(port: number | null, want: boolean): Promise<void> {
    this.wanted = want;
    if (!want) {
      this.publish({ state: "none" });
      return;
    }
    this.publish({ state: "reading" });
    if (port === null) {
      this.publish({ state: "failed", outcome: { ok: false, why: "not_connected" } });
      return;
    }
    await this.again(port);
  }

  /** Read again, where a surface still wants it. */
  async again(port: number): Promise<void> {
    if (!this.wanted) return;
    this.asked += 1;
    const asked = this.asked;
    const path = this.picked.checkout("/manifest/run_sheet");
    if (path === null) {
      this.publish({ state: "failed", outcome: NOT_SET_UP });
      return;
    }
    const answer = await ask(port, "GET", path);
    // Nobody wants it any more, or a newer read was begun while this one was
    // in flight. Either way this answer is not the one to publish — including
    // its failure: a stale timeout must not blank a panel the newer read is
    // about to fill.
    if (!this.wanted || this.asked !== asked) return;
    if (answer.ok !== true) {
      this.publish({ state: "failed", outcome: answer.outcome });
      return;
    }
    this.publish({ state: "read", sheet: answer.body as CheckoutRunSheet });
  }

  /** The read ends with the window. Nothing is published: the surface is gone. */
  close(): void {
    this.wanted = false;
  }
}

/**
 * `GET /manifest/drift`, read on opening the Manifest surface and again when
 * Fleet re-reads the file. **`CheckoutSheetReader`'s rules, for its reasons**:
 * no id to check an answer against, and only the newest read publishes.
 */
export class DriftReader {
  private readonly publish: (read: ManifestDriftRead) => void;
  private readonly picked: Picked;
  private wanted = false;
  private asked = 0;

  constructor(publish: (read: ManifestDriftRead) => void, picked: Picked) {
    this.publish = publish;
    this.picked = picked;
  }

  get open(): boolean {
    return this.wanted;
  }

  async want(port: number | null, want: boolean): Promise<void> {
    this.wanted = want;
    if (!want) {
      this.publish({ state: "none" });
      return;
    }
    this.publish({ state: "reading" });
    if (port === null) {
      this.publish({ state: "failed", outcome: { ok: false, why: "not_connected" } });
      return;
    }
    await this.again(port);
  }

  async again(port: number): Promise<void> {
    if (!this.wanted) return;
    this.asked += 1;
    const asked = this.asked;
    const path = this.picked.manifest("/manifest/drift");
    if (path === null) {
      this.publish({ state: "failed", outcome: NOT_SET_UP });
      return;
    }
    const answer = await ask(port, "GET", path);
    if (!this.wanted || this.asked !== asked) return;
    if (answer.ok !== true) {
      this.publish({ state: "failed", outcome: answer.outcome });
      return;
    }
    this.publish({ state: "read", drift: answer.body as ManifestDrift });
  }

  close(): void {
    this.wanted = false;
  }
}

/** What starting and stopping a checkout run needs of the connection. */
export type CheckoutBoard = {
  port: () => number | null;
  /** Whose main checkout every run here is in. */
  picked: Picked;
  /** Follow this run's output from the instant it answers as underway. */
  follow: (port: number, runId: string) => void;
  /** Read the sheet again — its own `running` field is what moved. */
  refreshSheet: (port: number) => Promise<void>;
};

/** `start`, `stop`, `undo`, and the three reads the page's history draws from. */
export class CheckoutRunCommands {
  private readonly board: CheckoutBoard;

  constructor(board: CheckoutBoard) {
    this.board = board;
  }

  /**
   * Run one Check or Command in the main checkout, as it is on disk. `body.workspace` names the
   * directory whose own `armada.yml` declares it; absent is the root's.
   *
   * **Answers at once, with the run underway.** Fleet takes a snapshot first
   * and the output streams on `observe_checkout_run`, which this opens the
   * moment the run exists.
   */
  async startRun(body: StartCheckoutRun): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const path = this.board.picked.checkout("/manifest/start_run");
    if (path === null) return NOT_SET_UP;
    const answer = await ask(port, "POST", path, body);
    if (answer.ok !== true) return answer.outcome;
    const running = answer.body as CheckoutRunUnderway;
    this.board.follow(port, running.id);
    await this.board.refreshSheet(port);
    return { ok: true };
  }

  /**
   * Verify — setup and every Check once, one after another. **Answers once
   * the first step is out**, and follows it; each step after is followed as
   * the sheet's own `running` moves on `checkout_run.finished`. `workspace`
   * names a directory whose own `armada.yml` runs; absent is the root's.
   */
  async startVerify(workspace?: string): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const path = this.board.picked.checkout("/manifest/start_verify");
    if (path === null) return NOT_SET_UP;
    const body: StartCheckoutVerify | undefined = workspace === undefined ? undefined : { workspace };
    const answer = await ask(port, "POST", path, body);
    if (answer.ok !== true) return answer.outcome;
    const out = (answer.body as CheckoutVerify).steps.find((step) => step.state === "running");
    if (out?.state === "running") this.board.follow(port, out.run_id);
    await this.board.refreshSheet(port);
    return { ok: true };
  }

  /** End a run's process group. The log keeps what printed. */
  async stopRun(id: string): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const path = this.board.picked.checkout("/manifest/stop_run");
    if (path === null) return NOT_SET_UP;
    const body: NamedRun = { id };
    const answer = await ask(port, "POST", path, body);
    if (answer.ok !== true) return answer.outcome;
    await this.board.refreshSheet(port);
    return { ok: true };
  }

  /**
   * Put back the files one run changed, from the snapshot taken just before
   * it. **This tree holds a person's own uncommitted work**, so the surface
   * names every path before it asks — and the page's own reading of whether
   * the run has been undone comes from `list_checkout_runs` read again, not
   * from this answer.
   */
  async undoRun(id: string): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const path = this.board.picked.checkout("/manifest/undo_run");
    if (path === null) return NOT_SET_UP;
    const body: NamedRun = { id };
    const answer = await ask(port, "POST", path, body);
    return answer.ok === true ? { ok: true } : answer.outcome;
  }

  /** Every earlier run in this checkout, newest first, and what would not read. */
  async listRuns(): Promise<CheckoutRunListRead> {
    const port = this.board.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.board.picked.checkout("/manifest/runs");
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, runs: answer.body as CheckoutRunList };
  }

  /** One run's log, read back as a window that says it is one. */
  async getRunOutput(runId: string): Promise<RunOutputRead> {
    const port = this.board.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.board.picked.checkout(`/manifest/runs/${encodeURIComponent(runId)}/output`);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, output: answer.body as RunOutput };
  }

  /**
   * What one run changed, **against the snapshot it took just before it** —
   * never `HEAD`, which would show a person's own uncommitted work as the
   * run's. A snapshot that is gone comes back as a reading that says so, not
   * as a refusal. The patch is the expensive half, asked for by the person who
   * pressed *Open the diff* rather than carried on the run list.
   */
  async getRunDiff(runId: string): Promise<CheckoutRunDiffRead> {
    const port = this.board.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.board.picked.checkout(`/manifest/runs/${encodeURIComponent(runId)}/diff`);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, diff: answer.body as CheckoutRunDiff };
  }
}

/**
 * One checkout run's output, as a window is watching it. **`RunSocket`'s
 * shape, one owner over** — a run id and no Job — and one at a time for the
 * same reason: the page shows one run's output, and opening another replaces
 * it.
 */
export class CheckoutRunSocket {
  private readonly publish: (followed: CheckoutRunFollowed) => void;
  private readonly picked: Picked;
  private socket: WebSocket | null = null;
  private held: CheckoutRunFollowed = { state: "none" };

  constructor(publish: (followed: CheckoutRunFollowed) => void, picked: Picked) {
    this.publish = publish;
    this.picked = picked;
  }

  /** Which run is being read, or `null` to stop. */
  open(port: number | null, runId: string | null): void {
    const held = this.held;
    if (
      runId !== null &&
      held.state !== "none" &&
      held.state !== "failed" &&
      held.runId === runId
    ) {
      return;
    }
    this.close();
    if (runId === null) {
      this.set({ state: "none" });
      return;
    }
    if (port === null) {
      this.set({ state: "failed", runId, detail: "Fleet is not connected." });
      return;
    }
    const path = this.picked.checkout(`/manifest/runs/${encodeURIComponent(runId)}/observe`);
    if (path === null) {
      this.set({ state: "failed", runId, detail: "No repository is picked." });
      return;
    }
    this.set({ state: "opening", runId });

    const socket = new WebSocket(`ws://${HOST}:${port}${path}`);
    this.socket = socket;
    socket.on("message", (data: WebSocket.RawData) => this.arrived(runId, String(data)));
    socket.on("error", (cause: Error) => this.broke(runId, cause.message));
    socket.on("close", () => this.broke(runId, "the connection closed"));
  }

  close(): void {
    const socket = this.socket;
    this.socket = null;
    if (socket === null) return;
    socket.removeAllListeners();
    // One listener stays, `RunSocket`'s reason: a socket still shaking hands
    // reports its abort as an `error` on the next tick, and one with nothing
    // listening is thrown by Node.
    socket.on("error", () => {});
    socket.close();
  }

  private set(followed: CheckoutRunFollowed): void {
    this.held = followed;
    this.publish(followed);
  }

  private arrived(runId: string, text: string): void {
    let message: CheckoutRunMessage;
    try {
      message = JSON.parse(text) as CheckoutRunMessage;
    } catch {
      this.broke(runId, "Fleet sent a message this Bridge could not read.");
      return;
    }

    if (message.message === "opened") {
      this.set({
        state: "following",
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
  private broke(runId: string, detail: string): void {
    if (this.socket === null) return;
    this.socket.removeAllListeners();
    this.socket = null;
    const held = this.held;
    if (held.state === "following") {
      this.set({ ...held, ended: held.ended ?? "broke" });
      return;
    }
    this.set({ state: "failed", runId, detail });
  }
}
