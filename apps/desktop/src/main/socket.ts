// The socket lifecycle: read the runtime file, verify the pid, connect, and
// retry on drop.
//
// **Split out of `connection.ts`**, which named this as the next real seam
// once seven other files had already come out of it and what was left was one
// thing — a state machine, plus the arrival handler that folds each message
// into it. This is not a slice of that: it owns the wire underneath the state
// machine and nothing about what a message means, which is why the split does
// not put the state machine's transitions in two files. Every message this
// reads is handed to the caller's own `arrived` whole and unopened; deciding
// what it means, and closing the socket on purpose because of what it meant,
// both stay in `connection.ts`.
//
// **Bridge finds Fleet through a runtime file carrying port, pid and protocol
// version, and verifies the pid before connecting** — `runtime-file.ts` is
// where that happens, and it is what tells "Fleet is not running" from
// "running and unreachable" apart, two states a bare connection timeout would
// render identical.

import WebSocket from "ws";

import { PROTOCOL_VERSION } from "@armada/protocol";
import { connects, skew } from "@armada/protocol";
import type { Connection } from "@armada/protocol";
import { HOST, machinePath, read } from "./runtime-file";

/** How long to wait before reading the runtime file again. */
const RETRY_MS = 2000;

/** The one connection state that carries which Fleet Bridge is talking to. */
export type BridgeStateFleet = Extract<Connection, { state: "connected" }>["fleet"];

export type FleetSocketWiring = {
  home: string | undefined;
  now: () => number;
  /** A connection state to settle, this instant. */
  settle: (connection: Connection) => void;
  /**
   * A fresh socket is open, before anything has arrived on it — the instant
   * the next resync is this socket's first, which is what tells Fleet coming
   * back from a plain gap in a stream that never stopped.
   */
  opened: () => void;
  /** A message arrived on the open socket, unparsed. */
  arrived: (text: string, fleet: BridgeStateFleet) => void;
};

/**
 * Read the runtime file, verify the pid, connect — and keep trying, on a
 * drop or a refusal, for as long as `start()` has been called more recently
 * than `stop()`.
 */
export class FleetSocket {
  private readonly wiring: FleetSocketWiring;
  private socket: WebSocket | null = null;
  private retry: ReturnType<typeof setTimeout> | null = null;
  private unreachableSince: number | null = null;
  private stopped = false;

  constructor(wiring: FleetSocketWiring) {
    this.wiring = wiring;
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
  }

  /**
   * Close the socket on purpose, because the caller read a message it could
   * not use — one this Bridge could not parse, or a resync naming a version
   * this Bridge will not speak. The socket's own `close` event carries on
   * from there exactly as an ordinary drop would.
   */
  close(): void {
    this.socket?.close();
  }

  /**
   * The resync that follows a reconnection clears what a drop had recorded.
   * `connection.ts` calls this once the greeting it was waiting for arrives.
   */
  resetUnreachable(): void {
    this.unreachableSince = null;
  }

  private async attach(): Promise<void> {
    if (this.stopped) return;
    const path = machinePath(this.wiring.home);
    if (path === null) {
      this.wiring.settle({
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
      this.wiring.settle({ state: "not_running", absence: presence.absence });
      return this.later();
    }
    if (presence.at === "refused") {
      this.unreachableSince = null;
      this.wiring.settle({ state: "runtime_file_refused", fault: presence.fault });
      return this.later();
    }

    const fleet = presence.fleet;
    // Read before connecting, so a version Bridge will not speak is a refusal
    // rather than a bad first message. A minor gap one way round is not one.
    const reading = skew({ fleet: fleet.protocolVersion, bridge: PROTOCOL_VERSION });
    if (!connects(reading)) {
      const speaks = fleet.protocolVersion;
      const expected = PROTOCOL_VERSION;
      this.wiring.settle({ state: "version_skew", fleet, why: reading, speaks, expected });
      return this.later();
    }

    this.wiring.settle(
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
    // The next resync to arrive is this socket's first, so it is Fleet coming
    // back rather than a gap in a stream that never stopped.
    this.wiring.opened();

    socket.on("message", (data: WebSocket.RawData) => this.wiring.arrived(String(data), fleet));
    socket.on("error", (cause: Error) => this.dropped(fleet, cause.message));
    socket.on("close", () => this.dropped(fleet, "the connection closed"));
  }

  /** A drop says so. It never leaves stale state reading as live. */
  private dropped(fleet: BridgeStateFleet, detail: string): void {
    if (this.socket === null || this.stopped) return;
    this.socket.removeAllListeners();
    this.socket = null;
    if (this.unreachableSince === null) this.unreachableSince = this.wiring.now();
    this.wiring.settle({ state: "unreachable", fleet, detail, sinceMs: this.unreachableSince });
    this.later();
  }

  private later(): void {
    if (this.stopped || this.retry !== null) return;
    this.retry = setTimeout(() => {
      this.retry = null;
      void this.attach();
    }, RETRY_MS);
  }
}
