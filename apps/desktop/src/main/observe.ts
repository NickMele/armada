// The socket that watches one Job's turns, held here beside the connection
// rather than inside it.
//
// **It reads and it cannot do anything else.** There is no send on this socket
// and no route behind it that takes one, so watching a Job leaves the Job
// exactly as it was found — which is the whole difference between Observe and
// Pilot, and the reason nothing here offers to intervene.
//
// It is a second socket to the same peer, not a second peer:
// `docs/practices/protocol.md`'s "The second socket" is why transcript rows do
// not travel `/events`. Bridge still talks to Fleet and to nothing else.

import WebSocket from "ws";

import type { Observed, Turn, Turns } from "../shared/bridge";
import type { TurnMessage } from "../shared/turn";
import { HOST } from "./runtime-file";

/** Nothing has arrived yet, and nothing has been lost. */
const FRESH: Turns = { live: false, skipped: 0, missed: 0, rows: [] };

/**
 * One Job's Observe connection.
 *
 * One at a time, because a viewer opens one Job deliberately and closing the
 * pane ends the watching. A second open replaces the first rather than holding
 * two sockets against Fleet.
 */
export class ObserveSocket {
  private readonly publish: (observed: Observed) => void;
  private socket: WebSocket | null = null;
  private jobId: string | null = null;
  private turns: Turns = FRESH;
  /** Monotonic per connection. A row's own identity, since none carries one. */
  private seq = 0;

  constructor(publish: (observed: Observed) => void) {
    this.publish = publish;
  }

  /** Which Job's turns are being watched, or `null` to stop watching. */
  open(port: number | null, jobId: string | null): void {
    this.close();
    this.jobId = jobId;
    this.turns = FRESH;
    this.seq = 0;
    if (jobId === null) {
      this.publish({ state: "none" });
      return;
    }
    if (port === null) {
      this.publish({ state: "failed", jobId, detail: "Fleet is not connected." });
      return;
    }
    this.publish({ state: "opening", jobId });

    const path = `/jobs/${encodeURIComponent(jobId)}/observe`;
    const socket = new WebSocket(`ws://${HOST}:${port}${path}`);
    this.socket = socket;
    socket.on("message", (data: WebSocket.RawData) => this.arrived(String(data)));
    // A 404 before the upgrade and a socket that broke both land here. Neither
    // is "the Job has no turns", which is an ordinary answer with rows of its
    // own, so this says what happened rather than rendering as an empty pane.
    socket.on("error", (cause: Error) => this.broke(cause.message));
    socket.on("close", () => this.ended("the connection closed"));
  }

  /** Whether a socket is up. A reconnecting Fleet reopens one that is not. */
  attached(): boolean {
    return this.socket !== null;
  }

  close(): void {
    this.socket?.removeAllListeners();
    this.socket?.close();
    this.socket = null;
  }

  private arrived(text: string): void {
    const jobId = this.jobId;
    if (jobId === null) return;
    let message: TurnMessage;
    try {
      message = JSON.parse(text) as TurnMessage;
    } catch {
      this.broke("Fleet sent a message this Bridge could not read.");
      return;
    }

    if (message.message === "opened") {
      // `live` and `skipped` are stated once, on the first message, and are
      // the two facts a reader needs before the first row: whether anything is
      // still writing, and whether the history in front of them is whole.
      this.turns = { ...FRESH, live: message.live, skipped: message.skipped };
      this.publish({ state: "watching", jobId, turns: this.turns });
      return;
    }

    if (message.message === "row") {
      // `step` is named in the rest pattern rather than left to fall into it:
      // the wire carries it beside the row's kind, `Saw` declares no such
      // field, and a spread would put it on the union at runtime where no
      // reader can see it. It travelled that way, undrawn, until #160.
      const { message: _tag, ts, step, ...saw } = message;
      const row: Turn = { ts, seq: this.seq++, step, saw };
      this.turns = { ...this.turns, rows: [...this.turns.rows, row] };
      this.publish({ state: "watching", jobId, turns: this.turns });
      return;
    }

    if (message.message === "missed") {
      // Counted and said, never skipped quietly: a transcript with a silent
      // gap reads as a Drone that went quiet, which is the one thing this
      // record exists to tell apart.
      this.turns = { ...this.turns, missed: this.turns.missed + message.dropped };
      this.publish({ state: "watching", jobId, turns: this.turns });
      return;
    }

    // `closed` carries why, because a socket that simply stops is
    // indistinguishable from one that broke. The rows are kept, and the socket
    // is let go here rather than left to close under its own event: Fleet sends
    // `closed` and *then* closes, so a listener still attached would answer the
    // transport's close by restating the transport — overwriting `drone_ended`,
    // the one reason a viewer actually wanted, with "the connection closed".
    this.close();
    this.turns = { ...this.turns, live: false };
    this.publish({ state: "ended", jobId, turns: this.turns, because: message.because });
  }

  private ended(because: string): void {
    const jobId = this.jobId;
    if (jobId === null || this.socket === null) return;
    this.socket.removeAllListeners();
    this.socket = null;
    this.turns = { ...this.turns, live: false };
    this.publish({ state: "ended", jobId, turns: this.turns, because });
  }

  private broke(detail: string): void {
    const jobId = this.jobId;
    if (jobId === null) return;
    this.socket?.removeAllListeners();
    this.socket = null;
    this.publish({ state: "failed", jobId, detail });
  }
}
