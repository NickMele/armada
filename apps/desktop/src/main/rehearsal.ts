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

import type {
  NamedRun,
  RunFollowed,
  RunList,
  RunListRead,
  RunMessage,
  RunOutput,
  RunOutputRead,
  RunSheet,
  RunUnderway,
  ServerState,
  StartRun,
} from "@armada/protocol";
import type { Outcome } from "@armada/protocol";
import type { CheckoutRunListRead, StartCheckoutRun } from "@armada/protocol";
import type { CheckoutRunDiffRead } from "@armada/protocol";
import type { BridgeState, PickedView } from "../shared/bridge";
import { ask, route, serversOf } from "./request";
import { CheckoutRunCommands, CheckoutRunSocket, CheckoutSheetReader, DriftReader } from "./checkout-runs";
import { JobReader } from "./reader";
import { HOST } from "./runtime-file";
import { ServerCommands } from "./servers";
import { Picked, type PickedByWindow } from "./picked";

/** What starting and stopping a run needs of the connection, and no more. */
export type RunBoard = {
  port: () => number | null;
  /** Follow this run's output from the instant it answers as underway. */
  follow: (port: number, jobId: string, runId: string) => void;
  /** Read the sheet again — its own `running` field is what moved. */
  refreshSheet: (port: number) => Promise<void>;
};

/** `start_run`, `stop_run`, `undo_run`, and the two reads the sheet's own
 * history draws from — `list_runs` and `get_run_output`. */
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

  /**
   * Put back the files one run changed, from the snapshot taken just before
   * it. Refused while a Drone is working, while a run is in flight, on a run
   * already undone or with no snapshot, and where a changed path has moved
   * since. The sheet's own reading of the run — its `undone_at` — comes from
   * `list_runs` read again, not from this answer.
   */
  async undoRun(jobId: string, id: string): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const body: NamedRun = { id };
    const answer = await ask(port, "POST", route(jobId, "undo_run"), body);
    return answer.ok === true ? { ok: true } : answer.outcome;
  }

  /** Every earlier run from the sheet, newest first, and what would not read. */
  async listRuns(jobId: string): Promise<RunListRead> {
    const port = this.board.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "GET", route(jobId, "runs"));
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, runs: answer.body as RunList };
  }

  /** One run's log, read back as a window that says it is one — `get_check_output`'s shape. */
  async getRunOutput(jobId: string, runId: string): Promise<RunOutputRead> {
    const port = this.board.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "GET", route(jobId, `runs/${encodeURIComponent(runId)}/output`));
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, output: answer.body as RunOutput };
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

/** The Manifest surface's own four, one set per window — `checkout-runs.ts`. */
type CheckoutFacades = {
  checkoutSheet: CheckoutSheetReader;
  checkoutFollow: CheckoutRunSocket;
  checkoutRuns: CheckoutRunCommands;
  drift: DriftReader;
};

/**
 * Journey 9 and servers, both, as one facade `connection.ts` holds one field
 * for. **Split out of `connection.ts`**, which named the run sheet and the
 * servers it started as its own subject once the state machine and the
 * arrival handler had the file back at the 900-line gate — the run sheet's
 * own reader, its output socket, its commands and the server commands all
 * moved here together, because the four were one thing added at once and
 * splitting them again would put one feature's wiring in four files.
 *
 * **The run sheet above is a Job's, shared** — every window reading the same
 * Job sees the same sheet. **The Manifest surface's four below are a window's
 * own** — `windows`, one set per window off that window's own `Picked`.
 */
export class RehearsalConnection {
  private readonly publish: (change: Partial<BridgeState>) => void;
  private readonly publishToWindow: (windowId: number, change: Partial<PickedView>) => void;
  private readonly port: () => number | null;
  private readonly pickedByWindow: PickedByWindow;
  private readonly sheet: JobReader<{ sheet: RunSheet }>;
  private readonly follow: RunSocket;
  private readonly runs: RunCommands;
  private readonly servers: ServerCommands;
  private readonly windows = new Map<number, CheckoutFacades>();

  constructor(wiring: {
    publish: (change: Partial<BridgeState>) => void;
    publishToWindow: (windowId: number, change: Partial<PickedView>) => void;
    port: () => number | null;
    pickedByWindow: PickedByWindow;
  }) {
    this.publish = wiring.publish;
    this.publishToWindow = wiring.publishToWindow;
    this.port = wiring.port;
    this.pickedByWindow = wiring.pickedByWindow;
    this.sheet = new JobReader<{ sheet: RunSheet }>({
      route: (jobId) => `/jobs/${encodeURIComponent(jobId)}/run_sheet`,
      keeps: (body) => ({ sheet: body as RunSheet }),
      publish: (runSheet) => this.publish({ runSheet }),
    });
    this.follow = new RunSocket((runFollowed) => this.publish({ runFollowed }));
    this.runs = new RunCommands({
      port: this.port,
      follow: (port, jobId, runId) => this.follow.open(port, jobId, runId),
      refreshSheet: (port) => this.sheet.again(port),
    });
    // Never held with a real listing: `startServer`'s own default, exercised only where a
    // caller supplies no `picked` — every real one does.
    this.servers = new ServerCommands({ port: this.port, picked: new Picked() });
  }

  /** This window's own Manifest surface facades, built on first ask. */
  private facadesFor(windowId: number): CheckoutFacades {
    let found = this.windows.get(windowId);
    if (found === undefined) {
      const picked = this.pickedByWindow.of(windowId);
      const checkoutFollow = new CheckoutRunSocket(
        (checkoutRunFollowed) => this.publishToWindow(windowId, { checkoutRunFollowed }),
        picked,
      );
      const checkoutSheet = new CheckoutSheetReader(
        (checkoutRunSheet) => this.publishToWindow(windowId, { checkoutRunSheet }),
        picked,
      );
      const checkoutRuns = new CheckoutRunCommands({
        port: this.port,
        picked,
        follow: (port, runId) => checkoutFollow.open(port, runId),
        refreshSheet: (port) => checkoutSheet.again(port),
      });
      const drift = new DriftReader((manifestDrift) => this.publishToWindow(windowId, { manifestDrift }), picked);
      found = { checkoutSheet, checkoutFollow, checkoutRuns, drift };
      this.windows.set(windowId, found);
    }
    return found;
  }

  /** The window closed. Its Manifest surface facades go with it. */
  dropWindow(windowId: number): void {
    const found = this.windows.get(windowId);
    if (found === undefined) return;
    found.checkoutSheet.close();
    found.checkoutFollow.close();
    found.drift.close();
    this.windows.delete(windowId);
  }

  close(): void {
    this.sheet.close();
    this.follow.close();
    for (const windowId of this.windows.keys()) this.dropWindow(windowId);
  }

  /** Every server Fleet holds. Read once per connection; `server.*` on
   * `/events` keeps the list current from there — see `onServerEvent`. */
  async readServers(port: number): Promise<void> {
    const servers = await serversOf(port);
    if (servers !== null) this.publish({ servers });
  }

  /** A run finished. A rehearsal, so only the sheet's own reading moves, and
   * only where it is this run's Job. */
  onRunFinished(jobId: string, port: number): void {
    if (this.sheet.jobId === jobId) void this.sheet.again(port);
  }

  /**
   * A run in the main checkout finished — `checkout_run.finished`, its own
   * kind because the record names no Job. Every window whose Manifest surface
   * holds its sheet open reads it again; the event arrives on all of them.
   */
  onCheckoutRunFinished(port: number): void {
    for (const { checkoutSheet } of this.windows.values()) {
      if (checkoutSheet.open) void checkoutSheet.again(port);
    }
  }

  /**
   * A server moved. **Both sheets draw it off their own `servers[].instance`**, not the list above,
   * so the one holding it is read again — `onCheckoutRunFinished`'s terms. Fleet picks which
   * instance a sheet shows, live or last ended, and a second copy of that rule here would drift.
   */
  onServerMoved(row: ServerState, port: number): void {
    if (row.job_id === undefined) {
      for (const { checkoutSheet } of this.windows.values()) {
        if (checkoutSheet.open) void checkoutSheet.again(port);
      }
    } else if (this.sheet.jobId === row.job_id) {
      void this.sheet.again(port);
    }
  }

  /** One `server.*` event, folded into the list it replaces a row in or joins. */
  onServerEvent(current: readonly ServerState[], row: ServerState): ServerState[] {
    return current.some((one) => one.id === row.id)
      ? current.map((one) => (one.id === row.id ? row : one))
      : [row, ...current];
  }

  /** Read the run sheet — Journey 9 — or `null` to stop. Opened by the sheet. */
  async watchRunSheet(jobId: string | null): Promise<void> {
    await this.sheet.want(this.port(), jobId);
  }

  /** One run's output, or `null` to stop — a run `startRun` just began, or one
   * the sheet is reopening onto in flight. */
  async observeRun(jobId: string | null, runId: string | null): Promise<void> {
    this.follow.open(this.port(), jobId, runId);
  }

  /** Run one Check or Command in this Job's worktree, and start following it. */
  startRun(jobId: string, body: StartRun): Promise<Outcome> {
    return this.runs.startRun(jobId, body);
  }

  /** End a run's process group. Its log keeps what printed. */
  stopRun(jobId: string, id: string): Promise<Outcome> {
    return this.runs.stopRun(jobId, id);
  }

  /** Put back the files one run changed, from the snapshot taken just before it. */
  undoRun(jobId: string, id: string): Promise<Outcome> {
    return this.runs.undoRun(jobId, id);
  }

  /** Every earlier run from the sheet, newest first. */
  listRuns(jobId: string): Promise<RunListRead> {
    return this.runs.listRuns(jobId);
  }

  /** One run's log, read back as a window that says it is one. */
  getRunOutput(jobId: string, runId: string): Promise<RunOutputRead> {
    return this.runs.getRunOutput(jobId, runId);
  }

  /** Read what this window's own picked repository's Manifest declares, or `false` to stop. */
  async watchCheckoutRunSheet(windowId: number, want: boolean): Promise<void> {
    await this.facadesFor(windowId).checkoutSheet.want(this.port(), want);
  }

  /** One checkout run's output, or `null` to stop. */
  async observeCheckoutRun(windowId: number, runId: string | null): Promise<void> {
    this.facadesFor(windowId).checkoutFollow.open(this.port(), runId);
  }

  /** Run one Check or Command in this window's own checkout, and start following it. */
  startCheckoutRun(windowId: number, body: StartCheckoutRun): Promise<Outcome> {
    return this.facadesFor(windowId).checkoutRuns.startRun(body);
  }

  /** End a checkout run's process group. Its log keeps what printed. */
  stopCheckoutRun(windowId: number, id: string): Promise<Outcome> {
    return this.facadesFor(windowId).checkoutRuns.stopRun(id);
  }

  /** Put back the files one checkout run changed, from its own snapshot. */
  undoCheckoutRun(windowId: number, id: string): Promise<Outcome> {
    return this.facadesFor(windowId).checkoutRuns.undoRun(id);
  }

  /** Every earlier run in this window's own checkout, newest first. */
  listCheckoutRuns(windowId: number): Promise<CheckoutRunListRead> {
    return this.facadesFor(windowId).checkoutRuns.listRuns();
  }

  /** One checkout run's log, read back as a window that says it is one. */
  getCheckoutRunOutput(windowId: number, runId: string): Promise<RunOutputRead> {
    return this.facadesFor(windowId).checkoutRuns.getRunOutput(runId);
  }

  /** One checkout run's patch, against the snapshot it took. A read. */
  getCheckoutRunDiff(windowId: number, runId: string): Promise<CheckoutRunDiffRead> {
    return this.facadesFor(windowId).checkoutRuns.getRunDiff(runId);
  }

  /** Drift, or `false` to stop. Held open by this window's own Manifest surface alone. */
  async watchManifestDrift(windowId: number, want: boolean): Promise<void> {
    await this.facadesFor(windowId).drift.want(this.port(), want);
  }

  /**
   * This window's own picked repository moved, or gained its Manifest. What its Manifest
   * surface holds open is read again, and a run in the last one's checkout stops being followed.
   */
  async onRepositoryMoved(windowId: number, port: number): Promise<void> {
    const facades = this.facadesFor(windowId);
    facades.checkoutFollow.open(port, null);
    await Promise.all([
      facades.checkoutSheet.open ? facades.checkoutSheet.again(port) : null,
      facades.drift.open ? facades.drift.again(port) : null,
    ]);
  }

  /** Fleet re-read `armada.yml`; every window whose own pick reads that file draws its drift again. */
  onManifestReread(path: string, port: number): void {
    for (const [windowId, picked] of this.pickedByWindow.all()) {
      if (!picked.reads(path)) continue;
      const { drift } = this.facadesFor(windowId);
      if (drift.open) void drift.again(port);
    }
  }

  /** Verify — setup and every Check once, one after another, in this window's own checkout. */
  startCheckoutVerify(windowId: number, workspace?: string): Promise<Outcome> {
    return this.facadesFor(windowId).checkoutRuns.startVerify(workspace);
  }

  /** Start a declared server, for a Job's worktree or the main checkout — `picked` is the
   * calling window's own, where `jobId` is absent; unused otherwise. */
  startServer(name: string, jobId?: string, picked?: Picked): Promise<Outcome> {
    return this.servers.startServer(name, jobId, picked);
  }

  /** End a server's process group. Its log keeps what printed. */
  stopServer(id: string): Promise<Outcome> {
    return this.servers.stopServer(id);
  }
}
