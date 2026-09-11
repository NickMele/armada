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

import type { Observed, Turns } from "@armada/protocol";
import { NO_TURNS, SOCKET_CLOSED, turnArrived } from "@armada/protocol";
import type { TurnMessage } from "@armada/protocol";
import { HOST } from "./runtime-file";


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
  private turns: Turns = NO_TURNS;
  /** Monotonic per connection. A row's own identity, since none carries one. */
  private seq = 0;

  constructor(publish: (observed: Observed) => void) {
    this.publish = publish;
  }

  /** Which Job's turns are being watched, or `null` to stop watching. */
  open(port: number | null, jobId: string | null): void {
    this.close();
    this.jobId = jobId;
    this.turns = NO_TURNS;
    this.seq = 0;
    if (jobId === null) {
      this.publish({ state: "none" });
      return;
    }
    if (port === null) {
      this.publish({ state: "failed", jobId, turns: this.turns, detail: "Fleet is not connected." });
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
    socket.on("close", () => this.ended(SOCKET_CLOSED));
  }

  /** Whether a socket is up. A reconnecting Fleet reopens one that is not. */
  attached(): boolean {
    return this.socket !== null;
  }

  close(): void {
    const socket = this.socket;
    this.socket = null;
    if (socket === null) return;
    socket.removeAllListeners();
    // **One listener stays.** A socket still shaking hands does not close; ws
    // aborts the handshake and reports that abort as an `error`, on the next
    // tick and not this one — so removing every listener and closing leaves an
    // `error` with nothing listening, which Node throws. It crashed the main
    // process whenever a Job was opened while Fleet was still connecting.
    // Closing on purpose is not a failure a reader needs told, so this one
    // swallows it rather than routing it to `broke`.
    socket.on("error", () => {});
    socket.close();
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

    // What the message does to the turns is `turnArrived`'s, in the wire
    // package, so a recorded Job replayed in Storybook folds the same way.
    const next = turnArrived(this.turns, message, this.seq);
    if (message.message === "row") this.seq += 1;
    this.turns = next.turns;
    if (next.ended === undefined) {
      this.publish({ state: "watching", jobId, turns: this.turns });
      return;
    }

    // The socket is let go here rather than left to close under its own
    // event: Fleet sends `closed` and *then* closes, so a listener still
    // attached would answer the transport's close by restating the transport —
    // overwriting `drone_ended`, the one reason a viewer actually wanted, with
    // "the connection closed".
    this.close();
    this.publish({ state: "ended", jobId, turns: this.turns, because: next.ended });
  }

  private ended(because: string): void {
    const jobId = this.jobId;
    if (jobId === null || this.socket === null) return;
    this.socket.removeAllListeners();
    this.socket = null;
    this.turns = { ...this.turns, live: false };
    this.publish({ state: "ended", jobId, turns: this.turns, because });
  }

  /**
   * The socket could not be read, and what it had read stays read.
   *
   * **The rows travel with the failure.** They used to be dropped, so one
   * unreadable frame emptied a step's whole log and the panel went back to
   * reading as a step nothing had happened on — #324.
   */
  private broke(detail: string): void {
    const jobId = this.jobId;
    if (jobId === null) return;
    // **Let go, not merely dropped** — `journal.ts` carries why at length: a
    // frame this Bridge could not parse leaves a healthy socket, and dropping
    // it without closing left Fleet holding a connection for a pane that had
    // stopped listening.
    this.close();
    this.turns = { ...this.turns, live: false };
    this.publish({ state: "failed", jobId, turns: this.turns, detail });
  }
}
