// Bridge's one connection to Armada API: WebSocket for events, HTTP for
// queries and commands, held in the one process allowed to hold it.
//
// The renderer never opens a socket and never fetches. It reads what is
// published from here and calls back through the preload — a component wanting
// data it does not have is missing a preload call, not a fetch of its own.
//
// **Bridge never talks to a Drone.** Everything below names Fleet.

import WebSocket from "ws";

import { PROTOCOL_VERSION } from "../shared/generated/protocol-version";
import { connectedTo, NOTHING_YET } from "../shared/bridge";
import { connects, skew } from "../shared/version";
import type { BridgeState, Connection, Draft, Outcome, Watched } from "../shared/bridge";
import type { JobHistory } from "../shared/history";
import type {
  JobDetail,
  JobSummary,
  ProposeJob,
  Redirection,
  Redispatched,
  StreamMessage,
  WireError,
} from "../shared/protocol";
import { ObserveSocket } from "./observe";
import { ask, holdingsOf, isJobSummary } from "./request";
import { decide, ReviewMaterial, type Decision } from "./review";
import { auditPath, HOST, machinePath, read } from "./runtime-file";

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
  private readonly approving = new Set<string>();
  private readonly redispatching = new Set<string>();
  private readonly killing = new Set<string>();
  private readonly redirecting = new Set<string>();
  private readonly restarting = new Set<string>();
  /** The Job whose detail is open. `null` is no detail, and no read. */
  private watching: string | null = null;
  /** The Job whose turns are open. A second socket to Fleet — see `observe.ts`. */
  private observing: string | null = null;
  /** The Job whose transition history is unfolded. `null` is no read. */
  private recounting: string | null = null;
  private readonly turns: ObserveSocket;
  /** The claims and the patch, each read when a surface asks — see `review.ts`. */
  private readonly material: ReviewMaterial;
  /** Jobs with a decision on the work in flight. One press sends one decision. */
  private readonly deciding = new Set<string>();
  private stopped = false;

  constructor(wiring: Wiring) {
    this.wiring = wiring;
    // Resolved once, from the home main can see. A failure that cannot say
    // where its log is is half a failure.
    this.current = { ...NOTHING_YET, bridge: { auditPath: auditPath(wiring.home) } };
    this.turns = new ObserveSocket((observed) => this.publish({ observed }));
    this.material = new ReviewMaterial((change) => this.publish(change));
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
      await this.reread(fleet);
      await this.readHoldings(fleet);
      await this.readWatched(fleet);
      await this.readMoves(fleet);
      await this.material.reread(fleet.port);
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
    this.recounting = null;
    this.turns.close();
    this.material.close();
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
      void this.readHoldings(fleet);
      // A resync says nothing about the open Job's steps, so it is re-read.
      void this.readWatched(fleet);
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
      this.refresh(fleet, event.job.id);
      return;
    }

    if (event.kind === "job.files_changed") {
      // **Only the open Job's, and the whole list rather than a fold.** The
      // reading replaces what is held, so a file that stopped being changed
      // leaves by not being in the next one — a stream of additions could never
      // say that. A reading about a Job nobody has open is dropped: nothing on
      // the Board changes when a file does.
      const mine = this.watching === event.job_id;
      this.publish({
        connection,
        ...(mine ? { footprint: { state: "read" as const, jobId: event.job_id, reading: event } } : {}),
      });
      return;
    }

    const held = this.current.jobs.find((job) => job.id === event.job_id);
    if (held === undefined) {
      // `job.created` covers the ordinary case, so a move about a Job this
      // window has never seen means a message was missed.
      this.publish({ connection });
      void this.reread(fleet);
      return;
    }
    const moved: JobSummary = { ...held, status: event.to, reason: event.reason };
    this.publish({
      connection,
      jobs: this.current.jobs.map((job) => (job.id === moved.id ? moved : job)),
      readAt: this.wiring.now(),
    });
    this.refresh(fleet, moved.id);
  }

  private async reread(fleet: BridgeStateFleet): Promise<void> {
    const answer = await ask(fleet.port, "GET", "/jobs");
    if (answer.ok !== true) return;
    const list = answer.body as { jobs: JobSummary[]; unreadable?: [] };
    this.publish({
      jobs: list.jobs,
      unreadable: list.unreadable ?? [],
      readAt: this.wiring.now(),
    });
  }

  /** What a proposal may name. The reads are `request.ts`'s; the state is here. */
  private async readHoldings(fleet: BridgeStateFleet): Promise<void> {
    this.publish({ holds: await holdingsOf(fleet.port, this.current.holds) });
  }

  // ----------------------------------------------------------- one Job, whole
  /**
   * Read one Job whole and keep it current, or `null` to stop. Here rather
   * than in the renderer because every event naming this Job re-reads it, which
   * is what makes a rail redraw when a step advances.
   */
  async watchJob(jobId: string | null): Promise<void> {
    this.watching = jobId;
    // A footprint belongs to the Job it was read from. Carrying one into the
    // next Job opened would draw another Drone's files under this Job's title.
    const footprint = this.current.footprint;
    if (footprint.state === "read" && footprint.jobId !== jobId) {
      this.publish({ footprint: { state: "none" } });
    }
    if (jobId === null) {
      this.publish({ watched: { state: "none" } });
      return;
    }
    this.publish({ watched: { state: "reading", jobId } });
    const fleet = this.connected();
    if (fleet === null) {
      this.publish({ watched: { state: "failed", jobId, outcome: { ok: false, why: "not_connected" } } });
      return;
    }
    await this.readWatched(fleet);
  }

  /** Re-read the open Job, where the event was about it. */
  private refresh(fleet: BridgeStateFleet, jobId: string): void {
    // A history that is unfolded grows as the Job moves, so the move that was
    // just delivered is read back rather than left off the end of the list.
    if (this.recounting === jobId) void this.readMoves(fleet);
    if (this.watching !== jobId) return;
    void this.readWatched(fleet);
  }

  /**
   * `GET /jobs/:job_id`, published whole. A failed read keeps the last good
   * detail rather than blanking the screen, but only for the same Job — a first
   * read that fails has nothing to fall back to and says so.
   */
  private async readWatched(fleet: BridgeStateFleet): Promise<void> {
    const jobId = this.watching;
    if (jobId === null) return;
    const path = `/jobs/${encodeURIComponent(jobId)}`;
    const answer = await ask(fleet.port, "GET", path);
    // The open Job changed mid-read: nobody has this answer's Job open.
    if (this.watching !== jobId) return;
    if (answer.ok !== true) {
      const held = this.current.watched;
      if (held.state === "read" && held.jobId === jobId) return;
      this.publish({ watched: { state: "failed", jobId, outcome: answer.outcome } });
      return;
    }
    const detail = answer.body as JobDetail;
    this.publish({ watched: { state: "read", jobId, detail }, readAt: this.wiring.now() });
  }

  // ------------------------------------------------------ one Job's history
  /**
   * Read one Job's transition history, or `null` to stop.
   *
   * **Its own operation, asked for rather than paid for.** `get_job` is fetched
   * on every open of a Job; a history has no bound — it grows for as long as
   * the Job lives, and a retried step is a row per attempt plus the moves
   * around it. So the surface that draws it says when it wants one.
   */
  async readHistory(jobId: string | null): Promise<void> {
    this.recounting = jobId;
    if (jobId === null) {
      this.publish({ history: { state: "none" } });
      return;
    }
    this.publish({ history: { state: "reading", jobId } });
    const fleet = this.connected();
    if (fleet === null) {
      const outcome: Outcome = { ok: false, why: "not_connected" };
      this.publish({ history: { state: "failed", jobId, outcome } });
      return;
    }
    await this.readMoves(fleet);
  }

  /**
   * `GET /jobs/:job_id/events`, published whole.
   *
   * **The rows are carried, never folded.** `crates/store/src/fold.rs` owns the
   * machine, and Fleet loads the Job before it reads the log — so a history
   * that arrives is one the machine already admitted, and a second fold here
   * would agree with the first only until one of them changed.
   */
  private async readMoves(fleet: BridgeStateFleet): Promise<void> {
    const jobId = this.recounting;
    if (jobId === null) return;
    const path = `/jobs/${encodeURIComponent(jobId)}/events`;
    const answer = await ask(fleet.port, "GET", path);
    // The open section changed mid-read: nobody has this answer's Job open.
    if (this.recounting !== jobId) return;
    if (answer.ok !== true) {
      this.publish({ history: { state: "failed", jobId, outcome: answer.outcome } });
      return;
    }
    const read = answer.body as JobHistory;
    this.publish({ history: { state: "read", jobId, moves: read.moves } });
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

  // ---------------------------------------------------------------- commands

  /** Draft a Job onto the approval gate. What comes back is not a running Job. */
  async proposeJob(draft: Draft): Promise<Outcome> {
    // Refused here and not only in the form, whose check is a courtesy.
    if (draft.title.trim() === "") return { ok: false, why: "empty_title" };
    if (draft.brief.trim() === "") return { ok: false, why: "empty_brief" };
    // Ids Fleet holds. An empty one is an unfilled form, not a value to send.
    if (draft.workflowId === "") return { ok: false, why: "no_workflow" };
    if (draft.manifestId === "") return { ok: false, why: "no_manifest" };
    const fleet = this.connected();
    if (fleet === null) return { ok: false, why: "not_connected" };

    const proposal: ProposeJob = {
      // Rust stores a trimmed `Title`, so padding makes what comes back
      // differ from what was sent.
      title: draft.title.trim(),
      workflow_id: draft.workflowId,
      owner_manifest_id: draft.manifestId,
      origin: draft.origin,
      urgency: draft.urgency,
      atomic: draft.atomic,
      // Omitted rather than sent empty: `""` reads like a value, and Fleet
      // fills it from configuration.
      ...(draft.model === "" ? {} : { model: draft.model }),
      facts: draft.brief,
      // Sent unconditionally, even empty: unlike `model` there is no
      // meaningful absent-vs-empty reading for Fleet to fill in.
      attachments: draft.attachments.map((attachment) => ({
        staged_path: attachment.path,
        filename: attachment.filename,
        mime_type: attachment.mimeType,
      })),
    };
    const answer = await ask(fleet.port, "POST", "/jobs", proposal);
    if (answer.ok !== true) return answer.outcome;
    this.fold(answer.body as JobSummary);
    return { ok: true };
  }

  /** Release a Job to spawn. Approving twice does not spawn twice. */
  async approveDispatch(jobId: string): Promise<Outcome> {
    if (this.approving.has(jobId)) return { ok: false, why: "already_approving" };
    const fleet = this.connected();
    if (fleet === null) return { ok: false, why: "not_connected" };

    this.approving.add(jobId);
    this.publish({ approving: [...this.approving] });
    try {
      const path = `/jobs/${encodeURIComponent(jobId)}/approve_dispatch`;
      const answer = await ask(fleet.port, "POST", path);
      if (answer.ok !== true) return answer.outcome;
      this.fold(answer.body as JobSummary);
      return { ok: true };
    } finally {
      this.approving.delete(jobId);
      this.publish({ approving: [...this.approving] });
    }
  }

  /**
   * Kill the failed Job and mint its replacement. **Nothing is reopened.**
   *
   * Two Jobs come back because a redispatch is two acts, and both are folded so
   * the board shows the lineage. Accepted only from `escalated`.
   */
  async redispatchJob(jobId: string): Promise<Outcome> {
    if (this.redispatching.has(jobId)) return { ok: false, why: "already_redispatching" };
    const fleet = this.connected();
    if (fleet === null) return { ok: false, why: "not_connected" };

    this.redispatching.add(jobId);
    try {
      const path = `/jobs/${encodeURIComponent(jobId)}/redispatch`;
      const answer = await ask(fleet.port, "POST", path);
      if (answer.ok !== true) return answer.outcome;
      const both = answer.body as Redispatched;
      // Folding whatever a route answered would put a malformed row on the
      // board.
      if (!isJobSummary(both.replaced) || !isJobSummary(both.dispatched)) {
        await this.reread(fleet);
        return { ok: true };
      }
      this.fold(both.replaced);
      this.fold(both.dispatched);
      // The replacement's id: the Job the caller asked about is over, and the
      // one worth opening did not exist a moment ago.
      return { ok: true, jobId: both.dispatched.id };
    } finally {
      this.redispatching.delete(jobId);
    }
  }

  /** Kill the Drone. **The Job survives**, its worktree held for a redispatch. */
  async killDrone(jobId: string): Promise<Outcome> {
    return this.kill(jobId, "kill_drone");
  }

  /**
   * End the Job at `killed`, terminal. Legal from every non-terminal status,
   * including those no Drone ran under — which is why it is not the same act,
   * or the same button, as killing one.
   */
  async killJob(jobId: string): Promise<Outcome> {
    return this.kill(jobId, "kill_job");
  }

  /** One in flight per Job covers both kills: a second press aims at a row
   * that has already moved. */
  private async kill(jobId: string, operation: "kill_drone" | "kill_job"): Promise<Outcome> {
    if (this.killing.has(jobId)) return { ok: false, why: "already_killing" };
    const fleet = this.connected();
    if (fleet === null) return { ok: false, why: "not_connected" };

    this.killing.add(jobId);
    try {
      const path = `/jobs/${encodeURIComponent(jobId)}/${operation}`;
      const answer = await ask(fleet.port, "POST", path);
      if (answer.ok !== true) return answer.outcome;
      if (isJobSummary(answer.body)) this.fold(answer.body);
      else await this.reread(fleet);
      this.refresh(fleet, jobId);
      return { ok: true };
    } finally {
      this.killing.delete(jobId);
    }
  }

  /**
   * Say something to the Drone that is there. **The Job comes back
   * `running`**, at the same step, with the same Drone — nothing was spawned
   * and nothing was thrown away. Blank is refused before the request is
   * sent, matching the 422 Fleet would give it.
   */
  async redirectDrone(jobId: string, instruction: string): Promise<Outcome> {
    if (instruction.trim() === "") return { ok: false, why: "empty_instruction" };
    if (this.redirecting.has(jobId)) return { ok: false, why: "already_redirecting" };
    const fleet = this.connected();
    if (fleet === null) return { ok: false, why: "not_connected" };

    this.redirecting.add(jobId);
    try {
      const path = `/jobs/${encodeURIComponent(jobId)}/redirect`;
      const body: Redirection = { instruction };
      const answer = await ask(fleet.port, "POST", path, body);
      if (answer.ok !== true) return answer.outcome;
      if (isJobSummary(answer.body)) this.fold(answer.body);
      else await this.reread(fleet);
      this.refresh(fleet, jobId);
      return { ok: true };
    } finally {
      this.redirecting.delete(jobId);
    }
  }

  /**
   * Put a fresh Drone on the worktree the last one left, at the step that
   * stopped. **One Job comes back**, resuming rather than replacing — the
   * whole difference between this and a redispatch.
   */
  async restartStep(jobId: string): Promise<Outcome> {
    if (this.restarting.has(jobId)) return { ok: false, why: "already_restarting" };
    const fleet = this.connected();
    if (fleet === null) return { ok: false, why: "not_connected" };

    this.restarting.add(jobId);
    try {
      const path = `/jobs/${encodeURIComponent(jobId)}/restart_step`;
      const answer = await ask(fleet.port, "POST", path);
      if (answer.ok !== true) return answer.outcome;
      if (isJobSummary(answer.body)) this.fold(answer.body);
      else await this.reread(fleet);
      this.refresh(fleet, jobId);
      return { ok: true };
    } finally {
      this.restarting.delete(jobId);
    }
  }

  // ------------------------------------------------- one Job's work, reviewed
  // The two reads and the three acts. What each one is and why it is its own
  // entry is in `review.ts`; these hold the state the answers are published
  // into, which is the only part that belongs to the connection.

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

  /** Take the work. On the last step Fleet commits and delivers first. */
  async approveReview(jobId: string): Promise<Outcome> {
    return this.settleWork(jobId, "approve_review");
  }

  /** Send it back. **`running` again**, same step, same Drone. Blank refused. */
  async requestChanges(jobId: string, note: string): Promise<Outcome> {
    if (note.trim() === "") return { ok: false, why: "empty_note" };
    return this.settleWork(jobId, "request_changes", note);
  }

  /** A verdict on the work. **Terminal, and it ends the Drone.** */
  async rejectWork(jobId: string): Promise<Outcome> {
    return this.settleWork(jobId, "reject");
  }

  /**
   * One decision, sent once. **One in flight per Job covers all three**: a
   * second press aims at a Job that has already left `awaiting_review`, the
   * only status any of the three is legal on.
   */
  private async settleWork(jobId: string, what: Decision, note?: string): Promise<Outcome> {
    if (this.deciding.has(jobId)) return { ok: false, why: "already_deciding" };
    const fleet = this.connected();
    if (fleet === null) return { ok: false, why: "not_connected" };

    this.deciding.add(jobId);
    try {
      const answer = await decide(fleet.port, jobId, what, note);
      if (answer.ok !== true) return answer.outcome;
      if (isJobSummary(answer.body)) this.fold(answer.body);
      else await this.reread(fleet);
      this.refresh(fleet, jobId);
      return { ok: true };
    } finally {
      this.deciding.delete(jobId);
    }
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

  private settle(connection: Connection): void {
    this.publish({ connection });
  }

  private publish(change: Partial<BridgeState>): void {
    this.current = { ...this.current, ...change };
    this.wiring.publish(this.current);
  }
}

type BridgeStateFleet = Extract<Connection, { state: "connected" }>["fleet"];
