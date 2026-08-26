// Bridge's one connection to Armada API: WebSocket for events, HTTP for
// queries and commands.
//
// It lives in the main process because that is the only process allowed to hold
// it. The renderer never opens a socket and never fetches — it reads the state
// published from here and calls back through the preload for the two commands
// it may initiate. A component that wants data it does not have is missing a
// preload call, not a fetch of its own.
//
// **Bridge never talks to a Drone.** Everything below names Fleet.

import WebSocket from "ws";

import { PROTOCOL_VERSION } from "../shared/generated/protocol-version";
import type { BridgeState, Connection, Draft, Holdings, Outcome } from "../shared/bridge";
import type {
  JobSummary,
  ManifestSummary,
  ModelChoices,
  ProposeJob,
  StreamMessage,
  WireError,
  WorkflowSummary,
} from "../shared/protocol";
import { HOST, machinePath, read } from "./runtime-file";

/** How long to wait before reading the runtime file again. */
const RETRY_MS = 2000;
/** How long a command waits for an answer before it is a transport failure. */
const COMMAND_MS = 5000;

/** Time is injected, never read: a connection that calls the clock cannot be replayed. */
export type Clock = () => number;

export type Wiring = {
  home: string | undefined;
  publish: (state: BridgeState) => void;
  now: Clock;
};

const NOTHING_HELD: Holdings = { workflows: [], manifests: [], models: null };

const EMPTY: BridgeState = {
  connection: { state: "reading" },
  jobs: [],
  unreadable: [],
  missed: 0,
  readAt: null,
  approving: [],
  holds: NOTHING_HELD,
};

export class FleetConnection {
  private readonly wiring: Wiring;
  private current: BridgeState = EMPTY;
  private socket: WebSocket | null = null;
  private retry: ReturnType<typeof setTimeout> | null = null;
  private unreachableSince: number | null = null;
  private readonly approving = new Set<string>();
  private stopped = false;

  constructor(wiring: Wiring) {
    this.wiring = wiring;
  }

  /**
   * What Bridge holds, brought current first.
   *
   * **This is the reload fix.** A renderer that reloads re-runs its own initial
   * fetch, but that fetch reads what main already has — and main only learns
   * anything when the socket says so. The socket does not resync on a window
   * reload, because the window is not the client: main is, and its connection
   * never dropped. So a Job created before the connection existed, or missed
   * for any other reason, stayed missing however many times the window was
   * reloaded.
   *
   * A fresh reader gets current state regardless of what it missed, which is
   * the same promise the resync makes on the wire. It costs one `GET /jobs` per
   * window load.
   */
  async state(): Promise<BridgeState> {
    const fleet = this.connected();
    if (fleet !== null) {
      await this.reread(fleet);
      await this.readHoldings(fleet);
    }
    return this.current;
  }

  /** Read the runtime file, verify the pid, connect. In that order, always. */
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
  }

  // ------------------------------------------------------------- connecting

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
      // Neither opens a socket: the port in a stale file may be held by
      // something that is not Fleet.
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
    if (fleet.protocolVersion !== PROTOCOL_VERSION) {
      // Read before connecting, so skew is a refusal rather than a malformed
      // first message.
      this.settle({
        state: "version_skew",
        fleet,
        speaks: fleet.protocolVersion,
        expected: PROTOCOL_VERSION,
      });
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

  /** A dropped connection says so. It never leaves stale state reading as live. */
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

  // ---------------------------------------------------------------- arrivals

  private arrived(text: string, fleet: BridgeStateFleet): void {
    let message: StreamMessage;
    try {
      message = JSON.parse(text) as StreamMessage;
    } catch {
      // The stream carries no error message, so a message that will not parse
      // is a connection to drop rather than a state to fold.
      this.socket?.close();
      return;
    }

    if (message.message === "resync") {
      if (message.protocol_version !== PROTOCOL_VERSION) {
        this.socket?.close();
        this.settle({
          state: "version_skew",
          fleet,
          speaks: message.protocol_version,
          expected: PROTOCOL_VERSION,
        });
        return;
      }
      this.unreachableSince = null;
      this.publish({
        connection: { state: "connected", fleet, cursor: message.cursor },
        jobs: message.jobs.jobs,
        unreadable: message.jobs.unreadable ?? [],
        readAt: this.wiring.now(),
      });
      // What a proposal may name. Not on the stream — it changes when Fleet
      // restarts, not when a Job moves — so it is read once per connection.
      void this.readHoldings(fleet);
      return;
    }

    if (message.message === "missed") {
      // The count alone cannot repair what Bridge holds. A resync always
      // follows; until it lands the screen says how many it will never see.
      this.publish({ missed: this.current.missed + message.dropped });
      return;
    }

    const event = message.event;
    const connection: Connection = { state: "connected", fleet, cursor: message.cursor };

    if (event.kind === "job.created") {
      // The row travels whole, so the list gains it without a round trip. This
      // is the event that did not exist: a Job proposed over the API while
      // Bridge was connected published nothing and never appeared.
      this.publish({ connection });
      this.fold(event.job);
      return;
    }

    const held = this.current.jobs.find((job) => job.id === event.job_id);
    if (held === undefined) {
      // A move about a Job this window has never seen. `job.created` covers the
      // ordinary case now, so reaching here means a message was missed rather
      // than that the vocabulary is short — and a re-read is what repairs that.
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
  }

  private async reread(fleet: BridgeStateFleet): Promise<void> {
    const answer = await this.request(fleet.port, "GET", "/jobs");
    if (answer.ok !== true) return;
    const list = answer.body as { jobs: JobSummary[]; unreadable?: [] };
    this.publish({
      jobs: list.jobs,
      unreadable: list.unreadable ?? [],
      readAt: this.wiring.now(),
    });
  }

  /**
   * The workflows, the Manifests and the models Fleet holds.
   *
   * Three calls rather than one, because they are three operations in the
   * inventory and a combined one would be a name nothing else agrees with. A
   * call that fails leaves what is held unchanged: the composer offering a
   * stale roster is better than one offering none, and Fleet refuses anything
   * that has since gone.
   */
  private async readHoldings(fleet: BridgeStateFleet): Promise<void> {
    const [workflows, manifests, models] = await Promise.all([
      this.request(fleet.port, "GET", "/workflows"),
      this.request(fleet.port, "GET", "/manifests"),
      this.request(fleet.port, "GET", "/models"),
    ]);
    this.publish({
      holds: {
        workflows: workflows.ok === true ? (workflows.body as WorkflowSummary[]) : this.current.holds.workflows,
        manifests: manifests.ok === true ? (manifests.body as ManifestSummary[]) : this.current.holds.manifests,
        models: models.ok === true ? (models.body as ModelChoices) : this.current.holds.models,
      },
    });
  }

  // ---------------------------------------------------------------- commands

  /**
   * Draft a Job onto the approval gate. **The gate is unchanged** — what comes
   * back is a Job at `awaiting_approval`, not a running one.
   */
  async proposeJob(draft: Draft): Promise<Outcome> {
    // Refused before the Job is created, and refused here rather than only in
    // the form: the renderer's check is a courtesy, this one is the rule.
    if (draft.title.trim() === "") return { ok: false, why: "empty_title" };
    if (draft.brief.trim() === "") return { ok: false, why: "empty_brief" };
    // Both are ids Fleet holds and refuses anything else for, so an empty one
    // is a form that was never filled in rather than a value to send.
    if (draft.workflowId === "") return { ok: false, why: "no_workflow" };
    if (draft.manifestId === "") return { ok: false, why: "no_manifest" };
    const fleet = this.connected();
    if (fleet === null) return { ok: false, why: "not_connected" };

    const proposal: ProposeJob = {
      // Trimmed here as well as in the form: the Rust side stores a `Title`
      // trimmed, and sending the padding would make the value that comes back
      // differ from the one that was sent.
      title: draft.title.trim(),
      workflow_id: draft.workflowId,
      owner_manifest_id: draft.manifestId,
      origin: draft.origin,
      urgency: draft.urgency,
      atomic: draft.atomic,
      // Omitted rather than sent empty. The field is optional on the wire and
      // Fleet fills it from configuration; `""` would say the same thing in a
      // way that reads like a value.
      ...(draft.model === "" ? {} : { model: draft.model }),
      facts: draft.brief,
    };
    const answer = await this.request(fleet.port, "POST", "/jobs", proposal);
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
      const answer = await this.request(fleet.port, "POST", path);
      if (answer.ok !== true) return answer.outcome;
      this.fold(answer.body as JobSummary);
      return { ok: true };
    } finally {
      this.approving.delete(jobId);
      this.publish({ approving: [...this.approving] });
    }
  }

  private connected(): BridgeStateFleet | null {
    const connection = this.current.connection;
    return connection.state === "connected" ? connection.fleet : null;
  }

  private async request(
    port: number,
    method: "GET" | "POST",
    path: string,
    body?: unknown,
  ): Promise<{ ok: true; body: unknown } | { ok: false; outcome: Outcome }> {
    try {
      const answer = await fetch(`http://${HOST}:${port}${path}`, {
        method,
        headers: body === undefined ? undefined : { "content-type": "application/json" },
        body: body === undefined ? undefined : JSON.stringify(body),
        signal: AbortSignal.timeout(COMMAND_MS),
      });
      const text = await answer.text();
      if (!answer.ok) {
        const error = refusal(text);
        return {
          ok: false,
          outcome:
            error === null
              ? { ok: false, why: "transport", detail: `Fleet answered ${answer.status}` }
              : { ok: false, why: "refused", error },
        };
      }
      return { ok: true, body: JSON.parse(text) };
    } catch (cause) {
      const detail = cause instanceof Error ? cause.message : String(cause);
      return { ok: false, outcome: { ok: false, why: "transport", detail } };
    }
  }

  // ------------------------------------------------------------------- state

  /** A Job Fleet answered a command with. New rows lead; a known row is replaced. */
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

/**
 * A refusal, as the wire carries it, or `null` where the body is not one.
 *
 * **Nothing here mints a code.** A code's declaration lives beside the variant
 * that raises it and `cargo xtask verify-error-codes` collects them, so a code
 * invented in Bridge would be in no manifest and mean nothing to the lookup
 * Bridge does. A body that is not a `WireError` is reported as the transport
 * failure it is.
 */
function refusal(text: string): WireError | null {
  try {
    const parsed = JSON.parse(text) as WireError;
    if (typeof parsed.code === "string" && typeof parsed.message === "string") return parsed;
  } catch {
    return null;
  }
  return null;
}
