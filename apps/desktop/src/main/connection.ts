// Bridge's one connection to Armada API: WebSocket for events, HTTP for
// queries and commands, held in the one process allowed to hold it.
//
// The renderer never opens a socket and never fetches. It reads what is
// published from here and calls back through the preload — a component wanting
// data it does not have is missing a preload call, not a fetch of its own.
//
// **Bridge never talks to a Drone.** Everything below names Fleet.
//
// What is here is the socket, the runtime file and the state machine. What is
// none of those sits beside it and is handed a port: `request.ts` sends,
// `command.ts` acts on a Job, `reader.ts` holds one Job's read and the rule
// that drops a stale one, `review.ts` reads the work, `observe.ts` holds the
// second socket.

import WebSocket from "ws";

import { PROTOCOL_VERSION } from "../shared/generated/protocol-version";
import { connectedTo, identifying, NOTHING_YET } from "../shared/bridge";
import { connects, skew } from "../shared/version";
import type { BridgeState, Connection } from "../shared/bridge";
import type { JobHistory, Recorded } from "../shared/history";
import type { JobDetail, JobSummary, StreamMessage } from "../shared/protocol";
import { JobCommands } from "./command";
import { ObserveSocket } from "./observe";
import { JobReader } from "./reader";
import { ReportsReader } from "./reports";
import { ask, capacityOf, holdingsOf } from "./request";
import { ReviewMaterial } from "./review";
import { HOST, machinePath, read, startingIdentity } from "./runtime-file";

/** How long to wait before reading the runtime file again. */
const RETRY_MS = 2000;

/** Time is injected, never read: a connection that calls the clock cannot be replayed. */
export type Clock = () => number;

export type Wiring = {
  home: string | undefined;
  publish: (state: BridgeState) => void;
  now: Clock;
};

export class FleetConnection {
  private readonly wiring: Wiring;
  private current: BridgeState = NOTHING_YET;
  private socket: WebSocket | null = null;
  private retry: ReturnType<typeof setTimeout> | null = null;
  private unreachableSince: number | null = null;
  /**
   * The open Job, read whole and kept current. Here rather than in the renderer
   * because every event naming this Job re-reads it, which is what makes a rail
   * redraw when a step advances.
   */
  private readonly watched: JobReader<{ detail: JobDetail }>;
  /**
   * The open Job's transition history, where a surface unfolded one.
   *
   * **Its own operation, asked for rather than paid for.** `get_job` is fetched
   * on every open of a Job; a history has no bound — it grows for as long as
   * the Job lives, and a retried step is a row per attempt plus the moves
   * around it. So the surface that draws it says when it wants one.
   */
  private readonly history: JobReader<{ moves: Recorded[] }>;
  /** The Job whose turns are open. A second socket to Fleet — see `observe.ts`. */
  private observing: string | null = null;
  private readonly turns: ObserveSocket;
  /** The claims and the patch, each read when a surface asks — see `review.ts`. */
  private readonly material: ReviewMaterial;
  /**
   * Every filed report, where a surface asked — see `reports.ts`. **The one
   * read here no Job scopes**, because a report outlives the Job it is about.
   */
  private readonly reports = new ReportsReader((reports) => this.publish({ reports }));
  /**
   * Every act on a Job — see `command.ts`. Reached through this rather than
   * re-exported one method at a time: a delegator carries no reasoning, and
   * nine of them would be nine places for the reasoning to go missing.
   */
  readonly commands: JobCommands;
  private stopped = false;

  constructor(wiring: Wiring) {
    this.wiring = wiring;
    // Resolved once, from the home main can see. A failure that cannot say
    // where its log is is half a failure.
    this.current = { ...NOTHING_YET, bridge: startingIdentity(wiring.home) };
    this.turns = new ObserveSocket((observed) => this.publish({ observed }));
    this.material = new ReviewMaterial((change) => this.publish(change));
    this.watched = new JobReader<{ detail: JobDetail }>({
      route: (jobId) => `/jobs/${encodeURIComponent(jobId)}`,
      keeps: (body) => ({ detail: body as JobDetail }),
      keepsLastGood: true,
      // `readAt` moves only where a reading did, so a failure leaves the screen
      // saying when what it shows was last current.
      publish: (watched) =>
        this.publish(
          watched.state === "read" ? { watched, readAt: this.wiring.now() } : { watched },
        ),
    });
    this.history = new JobReader<{ moves: Recorded[] }>({
      // **The rows are carried, never folded.** `crates/store/src/fold.rs` owns
      // the machine, and Fleet loads the Job before it reads the log — so a
      // history that arrives is one the machine already admitted, and a second
      // fold here would agree with the first only until one of them changed.
      route: (jobId) => `/jobs/${encodeURIComponent(jobId)}/events`,
      keeps: (body) => ({ moves: (body as JobHistory).moves }),
      publish: (history) => this.publish({ history }),
    });
    this.commands = new JobCommands({
      port: () => this.connected()?.port ?? null,
      fold: (job) => this.fold(job),
      forget: (jobId) => this.forget(jobId),
      reread: (port) => this.reread(port),
      refresh: (port, jobId) => this.refresh(port, jobId),
      publish: (change) => this.publish(change),
    });
  }

  /**
   * What Bridge holds, brought current first. Also the bar's Refresh.
   *
   * **A window reload is not a resync**: main is the client and its connection
   * never dropped, so anything main missed stayed missing however many reloads
   * later. One round trip here makes a fresh reader current.
   */
  async state(): Promise<BridgeState> {
    const fleet = this.connected();
    if (fleet !== null) {
      await this.reread(fleet.port);
      await this.readHoldings(fleet.port);
      await this.watched.again(fleet.port);
      await this.history.again(fleet.port);
      await this.material.reread(fleet.port);
      // A no-op where nothing has them open. Nothing but Bridge files a
      // report, so the list moves when somebody presses a button in a window —
      // and a second window is a second somebody, which is what Refresh is for.
      await this.reports.again(fleet.port);
    }
    return this.current;
  }

  /** Read the runtime file, verify the pid, connect. That order, always. */
  start(): void {
    this.stopped = false;
    void this.attach();
  }

  stop(): void {
    this.stopped = true;
    if (this.retry !== null) clearTimeout(this.retry);
    this.retry = null;
    this.socket?.close();
    this.socket = null;
    // Watching ends with the window; the Job does not, because nothing observed
    // is written onto it.
    this.observing = null;
    this.history.close();
    this.turns.close();
    this.material.close();
    this.reports.close();
  }

  // -------------------------------------------------------------- connecting
  private async attach(): Promise<void> {
    if (this.stopped) return;
    const path = machinePath(this.wiring.home);
    if (path === null) {
      this.settle({
        state: "runtime_file_refused",
        fault: {
          why: "unreadable",
          path: "",
          detail: "HOME is not set, so the machine directory cannot be resolved",
        },
      });
      return this.later();
    }

    const presence = await read(path);
    if (this.stopped) return;

    if (presence.at === "absent" || presence.at === "stale") {
      // Both render as "Fleet is not running", and the screen says which.
      // Neither opens a socket: a stale file's port may not be Fleet's.
      this.unreachableSince = null;
      this.settle({ state: "not_running", absence: presence.absence });
      return this.later();
    }
    if (presence.at === "refused") {
      this.unreachableSince = null;
      this.settle({ state: "runtime_file_refused", fault: presence.fault });
      return this.later();
    }

    const fleet = presence.fleet;
    // Read before connecting, so a version Bridge will not speak is a refusal
    // rather than a bad first message. A minor gap one way round is not one.
    const reading = skew({ fleet: fleet.protocolVersion, bridge: PROTOCOL_VERSION });
    if (!connects(reading)) {
      const speaks = fleet.protocolVersion;
      const expected = PROTOCOL_VERSION;
      this.settle({ state: "version_skew", fleet, why: reading, speaks, expected });
      return this.later();
    }

    this.settle(
      this.unreachableSince === null
        ? { state: "connecting", fleet }
        : {
            state: "unreachable",
            fleet,
            detail: "the socket has not answered",
            sinceMs: this.unreachableSince,
          },
    );
    this.open(fleet.port, fleet);
  }

  private open(port: number, fleet: BridgeStateFleet): void {
    const socket = new WebSocket(`ws://${HOST}:${port}/events`);
    this.socket = socket;

    socket.on("message", (data: WebSocket.RawData) => this.arrived(String(data), fleet));
    socket.on("error", (cause: Error) => this.dropped(fleet, cause.message));
    socket.on("close", () => this.dropped(fleet, "the connection closed"));
  }

  /** A drop says so. It never leaves stale state reading as live. */
  private dropped(fleet: BridgeStateFleet, detail: string): void {
    if (this.socket === null || this.stopped) return;
    this.socket.removeAllListeners();
    this.socket = null;
    if (this.unreachableSince === null) this.unreachableSince = this.wiring.now();
    this.settle({ state: "unreachable", fleet, detail, sinceMs: this.unreachableSince });
    this.later();
  }

  private later(): void {
    if (this.stopped || this.retry !== null) return;
    this.retry = setTimeout(() => {
      this.retry = null;
      void this.attach();
    }, RETRY_MS);
  }

  // --------------------------------------------------------------- arrivals
  private arrived(text: string, fleet: BridgeStateFleet): void {
    let message: StreamMessage;
    try {
      message = JSON.parse(text) as StreamMessage;
    } catch {
      // The stream carries no error message, so an unparseable one is a
      // connection to drop rather than a state to fold.
      this.socket?.close();
      return;
    }

    if (message.message === "resync") {
      // Again, because a Fleet restarted under a live socket is not the one
      // the runtime file described.
      const reading = skew({ fleet: message.protocol_version, bridge: PROTOCOL_VERSION });
      if (!connects(reading)) {
        this.socket?.close();
        const speaks = message.protocol_version;
        const expected = PROTOCOL_VERSION;
        this.settle({ state: "version_skew", fleet, why: reading, speaks, expected });
        return;
      }
      this.unreachableSince = null;
      this.publish({
        connection: connectedTo(fleet, message.cursor),
        jobs: message.jobs.jobs,
        unreadable: message.jobs.unreadable ?? [],
        readAt: this.wiring.now(),
      });
      // What a proposal may name: read once per connection, because it changes
      // when Fleet restarts rather than when a Job moves.
      void this.readHoldings(fleet.port);
      // And how full the fleet is, which changes when a Job moves and is
      // therefore read again below on every status move.
      void this.readCapacity(fleet.port);
      // A resync says nothing about the open Job's steps, so it is re-read.
      void this.watched.again(fleet.port);
      // A pane left open across a Fleet restart reopens its own socket. Only
      // where it has none: a resync arrives after every dropped event too, and
      // reopening a working socket would restart the transcript from the top.
      if (this.observing !== null && !this.turns.attached()) {
        this.turns.open(fleet.port, this.observing);
      }
      return;
    }

    if (message.message === "missed") {
      // The count alone cannot repair what Bridge holds. A resync always
      // follows; until it lands the screen says how many were lost.
      this.publish({ missed: this.current.missed + message.dropped });
      return;
    }

    const event = message.event;
    const connection: Connection = connectedTo(fleet, message.cursor);

    if (event.kind === "job.created") {
      // The row travels whole, so the list gains it without a round trip — a
      // Job proposed over the API used to publish nothing and never appear.
      this.publish({ connection });
      this.fold(event.job);
      return;
    }

    if (event.kind === "job.step_advanced") {
      // **The row is replaced, not patched.** `current_step_id` has already
      // moved on the Job travelling with the event, and `event.status` is the
      // status the move happened *beneath* rather than a transition — folding
      // either by hand is how half a row goes stale.
      this.publish({ connection });
      this.fold(event.job);
      this.refresh(fleet.port, event.job.id);
      return;
    }

    if (event.kind === "job.files_changed") {
      // **Only the open Job's, and the whole list rather than a fold.** The
      // reading replaces what is held, so a file that stopped being changed
      // leaves by not being in the next one — a stream of additions could never
      // say that. A reading about a Job nobody has open is dropped: nothing on
      // the Board changes when a file does.
      const mine = this.watched.jobId === event.job_id;
      this.publish({
        connection,
        ...(mine ? { footprint: { state: "read" as const, jobId: event.job_id, reading: event } } : {}),
      });
      return;
    }

    if (event.kind === "job.judging") {
      // **Re-read rather than fold.** The call in flight is served on the open
      // Job's own `StepDetail.judging`, which is what a Bridge opened mid-call
      // already reads, so folding this message into a second copy would give
      // one fact two homes — and the one a surface picked would be whichever
      // arrived last. The event is the wake-up; the detail is the answer.
      //
      // Only the open Job's, for `job.files_changed`'s reason: nothing on the
      // Board changes when a call goes out. Two reads per Judge call, against
      // a call that lasts seconds to two minutes.
      this.publish({ connection });
      this.refresh(fleet.port, event.job_id);
      return;
    }

    if (event.kind === "job.forgotten") {
      // The opposite of `job.created`: the id, and nothing to fold — the row
      // is gone at Fleet by the time this arrives, so it is dropped here
      // rather than replaced. Covers a forget made from another window, or a
      // window that raced the event past its own call's answer.
      this.publish({ connection });
      this.forget(event.job_id);
      return;
    }

    const held = this.current.jobs.find((job) => job.id === event.job_id);
    if (held === undefined) {
      // `job.created` covers the ordinary case, so a move about a Job this
      // window has never seen means a message was missed.
      this.publish({ connection });
      void this.reread(fleet.port);
      return;
    }
    const moved: JobSummary = { ...held, status: event.to, reason: event.reason };
    this.publish({
      connection,
      jobs: this.current.jobs.map((job) => (job.id === moved.id ? moved : job)),
      readAt: this.wiring.now(),
    });
    this.refresh(fleet.port, moved.id);
    // **A status move is the only thing that changes the occupancy**, so this
    // is where the reading is taken rather than on a timer. The machine half
    // rides along on the same call, which means a disk that fills while nothing
    // moves is not noticed until something does — and something moving is the
    // moment it starts mattering, because that is when admission next asks.
    void this.readCapacity(fleet.port);
  }

  private async reread(port: number): Promise<void> {
    const answer = await ask(port, "GET", "/jobs");
    if (answer.ok !== true) return;
    const list = answer.body as { jobs: JobSummary[]; unreadable?: [] };
    this.publish({
      jobs: list.jobs,
      unreadable: list.unreadable ?? [],
      readAt: this.wiring.now(),
    });
  }

  /** What a proposal may name. The reads are `request.ts`'s; the state is here. */
  private async readHoldings(port: number): Promise<void> {
    this.publish({ holds: await holdingsOf(port, this.current.holds) });
  }

  /**
   * How full the fleet is. **A failed read publishes `null`**, which draws as
   * nothing rather than as the last count — the bar must not keep saying
   * "2 of 2" off an answer it could not get.
   */
  private async readCapacity(port: number): Promise<void> {
    this.publish({ capacity: await capacityOf(port) });
  }

  // -------------------------------------------- one Job, whole and recounted
  /** Read one Job whole and keep it current, or `null` to stop. */
  async watchJob(jobId: string | null): Promise<void> {
    // A footprint belongs to the Job it was read from. Carrying one into the
    // next Job opened would draw another Drone's files under this Job's title.
    const footprint = this.current.footprint;
    if (footprint.state === "read" && footprint.jobId !== jobId) {
      this.publish({ footprint: { state: "none" } });
    }
    await this.watched.want(this.connected()?.port ?? null, jobId);
  }

  /** Read one Job's transition history, or `null` to stop. */
  async readHistory(jobId: string | null): Promise<void> {
    await this.history.want(this.connected()?.port ?? null, jobId);
  }

  /** Re-read the open Job, where the event was about it. */
  private refresh(port: number, jobId: string): void {
    // A history that is unfolded grows as the Job moves, so the move that was
    // just delivered is read back rather than left off the end of the list.
    if (this.history.jobId === jobId) void this.history.again(port);
    if (this.watched.jobId !== jobId) return;
    void this.watched.again(port);
  }

  // -------------------------------------------------------- one Job's turns
  /**
   * Watch one Job's turns, or `null` to stop. **A socket that only reads** —
   * see `observe.ts`, and `docs/concepts/observe.md` for why that is the whole
   * difference from Pilot.
   */
  async observeJob(jobId: string | null): Promise<void> {
    this.observing = jobId;
    this.turns.open(this.connected()?.port ?? null, jobId);
  }

  // ------------------------------------------------- one Job's work, reviewed
  // The two reads. What each one is and why it is its own entry is in
  // `review.ts`; these hold the port the reads are made over, which is the only
  // part that belongs to the connection. The three decisions are `command.ts`'s.

  /** What one Job's Drones claimed. The cheap half of the pair. */
  async readEvidence(jobId: string | null): Promise<void> {
    await this.material.evidence(this.connected()?.port ?? null, jobId);
  }

  /**
   * One Job's worktree against its branch. **The expensive half, and the only
   * place the patch bytes are spent** — called by the surface that draws a diff
   * rather than by opening a Job, which is the separation
   * `crates/adapter-traits/src/work_product.rs` records.
   */
  async readDiff(jobId: string | null): Promise<void> {
    await this.material.diff(this.connected()?.port ?? null, jobId);
  }

  // ----------------------------------------------- every report, and the counts
  /**
   * Read every filed report, or drop what was read. **The one read here that no
   * Job scopes** — a report is about a Job and does not belong to one, so a
   * listing reached through a Job would lose the ones that outlived theirs.
   * `reports.ts` holds the read; this holds the port it is made over.
   */
  async readReports(want: boolean): Promise<void> {
    await this.reports.want(this.connected()?.port ?? null, want);
  }

  private connected(): BridgeStateFleet | null {
    const connection = this.current.connection;
    return connection.state === "connected" ? connection.fleet : null;
  }

  // ------------------------------------------------------------------ state
  /** A Job a command answered with. New rows lead; a known row is replaced. */
  private fold(job: JobSummary): void {
    const held = this.current.jobs.some((row) => row.id === job.id);
    this.publish({
      jobs: held
        ? this.current.jobs.map((row) => (row.id === job.id ? job : row))
        : [job, ...this.current.jobs],
      readAt: this.wiring.now(),
    });
  }

  /**
   * A Job's whole record is gone — `forget_job` answered for it, or
   * `job.forgotten` named it on the stream. **Removed, not folded**: unlike
   * every other event here there is no row left to replace it with, so a
   * client drops it from whatever it is holding.
   */
  private forget(jobId: string): void {
    this.publish({ jobs: this.current.jobs.filter((job) => job.id !== jobId) });
  }

  private settle(connection: Connection): void {
    this.publish({ connection });
  }

  private publish(change: Partial<BridgeState>): void {
    // Fleet's version rides on the identity, so it is brought current in the
    // one funnel every change passes through. `shared/bridge.ts` owns the rule.
    this.current = identifying({ ...this.current, ...change });
    this.wiring.publish(this.current);
  }
}

type BridgeStateFleet = Extract<Connection, { state: "connected" }>["fleet"];
