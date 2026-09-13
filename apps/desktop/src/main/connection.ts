// Bridge's one connection to Armada API: WebSocket for events, HTTP for
// queries and commands, held in the one process allowed to hold it.
//
// The renderer never opens a socket and never fetches. It reads what is
// published from here and calls back through the preload.
//
// **Bridge never talks to a Drone.** Everything below names Fleet.
//
// What is here is the state — `current` and `publish` — and the wiring that
// builds everything reaching it. `socket.ts` is the socket's own lifecycle;
// `arrivals.ts` folds one stream message into the state, handed `ArrivalHost`,
// a narrow view of this file rather than a copy of it. `job-focus.ts` holds
// the open Job whole; `job-reads.ts` holds the port its reviewed work is read
// over. Only addresses moved off the file the gate measures.

import { identifying, NOTHING_YET } from "../shared/bridge";
import type { BridgeState } from "../shared/bridge";
import type { Connection, JobSummary } from "@armada/protocol";
import type { CallRead, CheckOutputRead, FrameRead } from "@armada/protocol";
import { applyArrival, readCapacity, reread } from "./arrivals";
import type { ArrivalHost } from "./arrivals";
import { JobCommands } from "./command";
import { FollowSocket } from "./following";
import { JournalSocket } from "./journal";
import { JobFocus } from "./job-focus";
import { JobReads } from "./job-reads";
import { ObserveSocket } from "./observe";
import { HeldReader } from "./holding";
import { RehearsalConnection } from "./rehearsal";
import { ManifestFileCommands } from "./editing";
import { RepositoryAllowsCommands } from "./repository-allows";
import { RepositoryReads } from "./repositories";
import { Picked } from "./picked";
import { ReportsReader } from "./reports";
import { ReviewMaterial } from "./review";
import { startingIdentity } from "./runtime-file";
import { FleetSocket, type BridgeStateFleet } from "./socket";

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
  /** The socket lifecycle — reading the runtime file, connecting, retrying. */
  private readonly socket: FleetSocket;
  /**
   * Whether the resync now arriving is a later one on a socket already up.
   *
   * **What tells a reconnection from a gap**, and they are not the same
   * recovery. A gap is events lost under a connection that held: HTTP answered
   * throughout, so only the reads events feed can be stale. A reconnection is a
   * socket that was down, and every read attempted while it was down failed —
   * which is a failure standing on a panel that nothing else will clear.
   *
   * Both arrive as the same message, so the difference has to be held here.
   * `crates/api/src/sockets.rs` sends one on every connection and one more
   * after every `Missed`.
   */
  private greeted = false;
  /** The open Job, its examination and its history — see `job-focus.ts`. */
  private readonly jobFocus: JobFocus;
  /** Journey 9's run sheet and the servers it starts — see `rehearsal.ts`.
   * One field for both, `commands`' reason: they are one feature. */
  readonly rehearsal: RehearsalConnection;
  /** The Manifest file, read and saved — see `editing.ts`. */
  readonly editing: ManifestFileCommands;
  /** Repository-wide always-allows, read and removed — see `repository-allows.ts`. */
  readonly repositoryAllows: RepositoryAllowsCommands;
  /** What Fleet serves and which repository was picked — see `repositories.ts`. */
  readonly repositories: RepositoryReads;
  /** The Job whose turns are open. A second socket to Fleet — see `observe.ts`. */
  private observing: string | null = null;
  /**
   * The Job whose own log is open. **The same Job as `observing`, always** —
   * both sockets are opened by the same act of opening a Job, and they are two
   * fields rather than one because each may be down while the other is up.
   */
  private reading: string | null = null;
  /**
   * The token this window sent with the proposal it is waiting on, or `null`.
   *
   * **What tells this window's proposal from anybody else's.** Fleet publishes
   * every proposal on one stream, and the id it mints is not known until the
   * first event arrives — so the correlation has to run the other way, on a
   * token the window chose before it sent.
   *
   * Held here rather than on `BridgeState` because it is not a fact about the
   * app; it is how this object recognises its own messages, and a renderer that
   * could read it could claim another window's call.
   */
  private proposalRef: string | null = null;
  private readonly turns: ObserveSocket;
  /** What Fleet did to the open Job — a third socket. See `journal.ts`. */
  private readonly notes: JournalSocket;
  /** One running Check's log, as it is written — a fourth socket. `following.ts`. */
  private readonly follow: FollowSocket;
  /** Not `private`, `commands`' reason: `remarks-poll.ts` (`#667`) reaches `remarksChanged` from `index.ts`. */
  readonly material: ReviewMaterial;
  /**
   * Every filed report, where a surface asked — see `reports.ts`. **The one
   * read here no Job scopes**, because a report outlives the Job it is about.
   */
  private readonly reports = new ReportsReader((reports) => this.publish({ reports }));
  /**
   * What Fleet is holding disk for, where a surface asked — see `holding.ts`.
   * **Scoped to nothing, like the reports above it**: what is being decided is
   * which of a set to give back, and no Job id asks that.
   */
  private readonly held = new HeldReader((held) => this.publish({ held }));
  /**
   * Every act on a Job — see `command.ts`. Reached through this rather than
   * re-exported one method at a time: a delegator carries no reasoning, and
   * nine of them would be nine places for the reasoning to go missing.
   */
  readonly commands: JobCommands;
  /** One Job's work, reviewed, and the two collection-wide reads — see `job-reads.ts`. */
  private readonly jobReads: JobReads;
  /** `arrivals.ts`'s switch, and the exact slice of this object it may reach. */
  private readonly arrivalHost: ArrivalHost;

  constructor(wiring: Wiring) {
    this.wiring = wiring;
    // Resolved once, from the home main can see. A failure that cannot say
    // where its log is is half a failure.
    this.current = { ...NOTHING_YET, bridge: startingIdentity(wiring.home) };
    this.socket = new FleetSocket({
      home: wiring.home,
      now: wiring.now,
      settle: (connection) => this.settle(connection),
      opened: () => {
        this.greeted = false;
      },
      arrived: (text, fleet) => this.arrived(text, fleet),
    });
    this.turns = new ObserveSocket((observed) => this.publish({ observed }));
    this.notes = new JournalSocket((journalled) => this.publish({ journalled }));
    this.follow = new FollowSocket((followed) => this.publish({ followed }));
    this.material = new ReviewMaterial((change) => this.publish(change));
    const port = (): number | null => this.connected()?.port ?? null;
    this.jobFocus = new JobFocus({
      port,
      current: () => this.current,
      now: wiring.now,
      publish: (change) => this.publish(change),
      turns: this.turns,
      notes: this.notes,
      review: this.material,
      observing: () => this.observing,
      reading: () => this.reading,
    });
    const [publish, picked] = [(change: Partial<BridgeState>) => this.publish(change), new Picked()];
    this.rehearsal = new RehearsalConnection({ publish, port, picked });
    const holds = () => this.current.holds;
    this.repositories = new RepositoryReads({ picked, publish, holds, rehearsal: this.rehearsal, port });
    this.editing = new ManifestFileCommands(port, picked, (at) => this.repositories.readHoldings(at));
    this.repositoryAllows = new RepositoryAllowsCommands(port, picked);
    this.commands = new JobCommands({
      port,
      picked,
      fold: (job) => this.fold(job),
      forget: (jobId) => this.forget(jobId),
      reread: (port) => reread(port, (change) => this.publish(change), this.wiring.now),
      refresh: (port, jobId) => this.jobFocus.refresh(port, jobId),
      publish: (change) => this.publish(change),
      watchProposal: (clientRef) => {
        this.proposalRef = clientRef;
        // A window that starts a proposal has nothing to show until Fleet says
        // the call went out; one that has finished with a proposal shows
        // nothing either. Both are the same clear, and it is here rather than
        // only on the coming-back event so that a request which never reached
        // Fleet does not leave a card standing.
        if (clientRef === null) this.publish({ proposing: null });
      },
      proposalOut: () => this.current.proposing,
      rereadCapacity: (port) => readCapacity(port, (change) => this.publish(change)),
    });
    this.jobReads = new JobReads({ port, material: this.material, reports: this.reports, held: this.held });
    // The exact slice of this object `arrivals.ts`'s switch may reach — built
    // once, after everything it names, so the switch never touches a private
    // field directly. See the module doc.
    this.arrivalHost = {
      current: () => this.current,
      now: () => this.wiring.now(),
      greeted: () => this.greeted,
      setGreeted: (value) => {
        this.greeted = value;
      },
      proposalRef: () => this.proposalRef,
      setProposalRef: (value) => {
        this.proposalRef = value;
      },
      watchedJobId: () => this.jobFocus.watchedJobId(),
      repositories: this.repositories,
      rehearsal: this.rehearsal,
      material: this.material,
      socket: this.socket,
      publish: (change) => this.publish(change),
      fold: (job) => this.fold(job),
      forget: (jobId) => this.forget(jobId),
      settle: (connection) => this.settle(connection),
      refresh: (port, jobId) => this.jobFocus.refresh(port, jobId),
      takeAgain: (port, again) => this.jobFocus.takeAgain(port, again),
    };
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
      await reread(fleet.port, (change) => this.publish(change), this.wiring.now);
      await this.repositories.readHoldings(fleet.port);
      // Every region of the open Job — the same list a reconnection takes, so
      // the two cannot drift apart. #472.
      await this.jobFocus.takeAgain(fleet.port, { because: "a_person_asked" });
      // A no-op where nothing has them open. Nothing but Bridge files a
      // report, so the list moves when somebody presses a button in a window —
      // and a second window is a second somebody, which is what Refresh is for.
      await this.reports.again(fleet.port);
      // What is held changes when a Job ends, when a sweep runs, and when
      // somebody reclaims one. A refresh is the cheapest of the three to be
      // sure about.
      await this.held.again(fleet.port);
    }
    return this.current;
  }

  /** Read the runtime file, verify the pid, connect. That order, always. */
  start(): void {
    this.socket.start();
  }

  stop(): void {
    this.socket.stop();
    // Watching ends with the window; the Job does not, because nothing observed
    // is written onto it.
    this.observing = null;
    this.reading = null;
    this.jobFocus.close();
    this.rehearsal.close();
    this.turns.close();
    this.notes.close();
    this.follow.close();
    this.material.close();
    this.reports.close();
    this.held.close();
  }

  // --------------------------------------------------------------- arrivals
  /** One stream message, folded into the state. See `arrivals.ts`. */
  private arrived(text: string, fleet: BridgeStateFleet): void {
    applyArrival(this.arrivalHost, text, fleet);
  }

  // -------------------------------------------- one Job, whole and recounted
  // These, and `job-focus.ts`'s own `refresh`/`takeAgain` above, are that
  // file — reached through this because the renderer and `index.ts` still
  // call these exact names, and only their bodies moved.

  async watchJob(jobId: string | null): Promise<void> {
    await this.jobFocus.watchJob(jobId);
  }

  async readResources(jobId: string | null): Promise<void> {
    await this.jobFocus.readResources(jobId);
  }

  async examineJob(jobId: string): Promise<void> {
    await this.jobFocus.examineJob(jobId);
  }

  async readHistory(jobId: string | null): Promise<void> {
    await this.jobFocus.readHistory(jobId);
  }

  // -------------------------------------------------------- one Job's turns
  /**
   * Watch one Job's turns, or `null` to stop. **A socket that only reads** —
   * see `observe.ts`, and `docs/concepts/observe.md` for why that is the whole
   * difference from Pilot.
   */
  async observeJob(jobId: string | null): Promise<void> {
    const port = this.connected()?.port ?? null;
    this.observing = jobId;
    this.turns.open(port, jobId);
    // **The Job's own log opens with the transcript, on one act and not two.**
    // A person opening a Job wants what happened to it; which of the two
    // sockets carries a given line is Fleet's business rather than theirs, and
    // a second channel would let a surface open one and forget the other —
    // which is the empty panel this exists to fix, arriving by another route.
    this.reading = jobId;
    this.notes.open(port, jobId);
  }

  /** One running Check's log, as it is written, or `null` to stop. Opened by a press. */
  followCheckOutput(jobId: string | null, kept: string | null): void {
    this.follow.open(this.connected()?.port ?? null, jobId, kept);
  }

  // ------------------------------------------------- one Job's work, reviewed
  // These, `readReports`, `readHeld` and `rereadHeld` below are `job-reads.ts`,
  // reached through this rather than re-exported one method at a time — the
  // decisions naming each one are `command.ts`'s reason repeated. The renderer
  // and `index.ts` still call these exact names; only their bodies moved.

  async readEvidence(jobId: string | null): Promise<void> {
    await this.jobReads.readEvidence(jobId);
  }

  async readDiff(jobId: string | null): Promise<void> {
    await this.jobReads.readDiff(jobId);
  }

  async readRemarks(jobId: string | null): Promise<void> {
    await this.jobReads.readRemarks(jobId);
  }

  async readCall(jobId: string, callId: string): Promise<CallRead> {
    return await this.jobReads.readCall(jobId, callId);
  }

  async readCheckOutput(jobId: string, kept: string): Promise<CheckOutputRead> {
    return await this.jobReads.readCheckOutput(jobId, kept);
  }

  async readFrame(jobId: string, kept: string): Promise<FrameRead> {
    return await this.jobReads.readFrame(jobId, kept);
  }

  async readReports(want: boolean): Promise<void> {
    await this.jobReads.readReports(want);
  }

  async readHeld(want: boolean): Promise<void> {
    await this.jobReads.readHeld(want);
  }

  async rereadHeld(): Promise<void> {
    await this.jobReads.rereadHeld();
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
