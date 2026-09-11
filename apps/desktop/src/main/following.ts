// The socket that reads one running Check's log as it is written, held here
// beside `journal.ts` and for its reasons.
//
// **One at a time.** A person reads one Check's log; opening another replaces
// the first rather than holding two sockets against Fleet.
//
// **It reads and it cannot do anything else.** There is no send on this socket
// and no route behind it that takes one.

import WebSocket from "ws";

import type { FollowedLog, OutputMessage } from "@armada/protocol";
import { HOST } from "./runtime-file";

/**
 * How many lines main holds for the reader — the window a recorded Check log is
 * read in, so a live one and a recorded one show a person the same amount.
 */
const HELD = 2_000;

/** One running Check's log connection. */
export class FollowSocket {
  private readonly publish: (followed: FollowedLog) => void;
  private socket: WebSocket | null = null;
  private held: FollowedLog = { state: "none" };

  constructor(publish: (followed: FollowedLog) => void) {
    this.publish = publish;
  }

  /**
   * Which Check's log is being read, or `null` for either to stop.
   *
   * **The same log asked for again is the same reading.** A screen asks on
   * every render that names it, and a reading restarted each time would draw
   * its first window over and over.
   */
  open(port: number | null, jobId: string | null, kept: string | null): void {
    const held = this.held;
    if (
      jobId !== null &&
      kept !== null &&
      held.state !== "none" &&
      held.state !== "failed" &&
      held.jobId === jobId &&
      held.kept === kept
    ) {
      return;
    }
    this.close();
    if (jobId === null || kept === null) {
      this.set({ state: "none" });
      return;
    }
    if (port === null) {
      this.set({ state: "failed", jobId, kept, detail: "Fleet is not connected." });
      return;
    }
    this.set({ state: "opening", jobId, kept });

    const path = `/jobs/${encodeURIComponent(jobId)}/checks/${encodeURIComponent(kept)}/observe`;
    const socket = new WebSocket(`ws://${HOST}:${port}${path}`);
    this.socket = socket;
    socket.on("message", (data: WebSocket.RawData) => this.arrived(jobId, kept, String(data)));
    socket.on("error", (cause: Error) => this.broke(jobId, kept, cause.message));
    socket.on("close", () => this.broke(jobId, kept, "the connection closed"));
  }

  close(): void {
    const socket = this.socket;
    this.socket = null;
    if (socket === null) return;
    socket.removeAllListeners();
    // One listener stays, for `journal.ts`'s reason: a socket still shaking
    // hands reports its abort as an `error` on the next tick, and an `error`
    // with nothing listening is thrown by Node.
    socket.on("error", () => {});
    socket.close();
  }

  private set(followed: FollowedLog): void {
    this.held = followed;
    this.publish(followed);
  }

  private arrived(jobId: string, kept: string, text: string): void {
    let message: OutputMessage;
    try {
      message = JSON.parse(text) as OutputMessage;
    } catch {
      this.broke(jobId, kept, "Fleet sent a message this Bridge could not read.");
      return;
    }

    if (message.message === "opened") {
      this.set({
        state: "following",
        jobId,
        kept,
        name: message.name,
        attempt: message.attempt,
        path: message.path,
        fromLine: message.skipped + 1,
        lines: [],
      });
      return;
    }

    const held = this.held;
    if (held.state !== "following") return;

    if (message.message === "lines") {
      const all = [...held.lines, ...message.lines];
      const dropped = Math.max(0, all.length - HELD);
      this.set({ ...held, lines: all.slice(dropped), fromLine: held.fromLine + dropped });
      return;
    }

    // `closed` carries why, and the socket is let go first so its own `close`
    // cannot overwrite the reason — `journal.ts` learned this the hard way.
    this.close();
    this.set({ ...held, ended: message.because });
  }

  /**
   * The socket went without a sentence. **What had arrived stays arrived**, for
   * `journal.ts`'s reason: a reading that emptied on a dropped connection would
   * read as a Check that had printed nothing.
   */
  private broke(jobId: string, kept: string, detail: string): void {
    if (this.socket === null) return;
    this.socket.removeAllListeners();
    this.socket = null;
    const held = this.held;
    if (held.state === "following") {
      this.set({ ...held, ended: held.ended ?? "broke" });
      return;
    }
    this.set({ state: "failed", jobId, kept, detail });
  }
}
