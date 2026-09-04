// The socket that reads one Job's own log — what Fleet did to it — held here
// beside the connection and beside `observe.ts`.
//
// **Two sockets on one Job, and they answer different questions.** Observe is a
// Drone's transcript and exists only while a Drone is writing: it says
// `nothing_writing` and closes on exactly the Job this one is for. This is
// Fleet's own narration, and it is the only thing there is to draw while a
// worktree is being cut and a repository's preparation commands are running.
//
// **It reads and it cannot do anything else.** There is no send on this socket
// and no route behind it that takes one.

import WebSocket from "ws";

import type { JournalMessage, Journalled, JobLog, Noted } from "@armada/protocol";
import { HOST } from "./runtime-file";

/** Nothing has arrived yet, and nothing was left out. */
const FRESH: JobLog = { skipped: 0, notes: [] };

/**
 * One Job's log connection.
 *
 * One at a time, for `ObserveSocket`'s reason: a viewer opens one Job
 * deliberately and closing the pane ends the reading. A second open replaces
 * the first rather than holding two sockets against Fleet.
 */
export class JournalSocket {
  private readonly publish: (journalled: Journalled) => void;
  private socket: WebSocket | null = null;
  private jobId: string | null = null;
  private log: JobLog = FRESH;
  /** Monotonic per connection. A note's own identity, since none carries one. */
  private seq = 0;

  constructor(publish: (journalled: Journalled) => void) {
    this.publish = publish;
  }

  /** Which Job's log is being read, or `null` to stop reading. */
  open(port: number | null, jobId: string | null): void {
    this.close();
    this.jobId = jobId;
    this.log = FRESH;
    this.seq = 0;
    if (jobId === null) {
      this.publish({ state: "none" });
      return;
    }
    if (port === null) {
      this.publish({ state: "failed", jobId, log: this.log, detail: "Fleet is not connected." });
      return;
    }
    this.publish({ state: "opening", jobId });

    const path = `/jobs/${encodeURIComponent(jobId)}/log`;
    const socket = new WebSocket(`ws://${HOST}:${port}${path}`);
    this.socket = socket;
    socket.on("message", (data: WebSocket.RawData) => this.arrived(String(data)));
    // A 404 before the upgrade and a socket that broke both land here. Neither
    // is "Fleet has done nothing to this Job", which is an ordinary answer with
    // no notes in it — so this says what happened rather than drawing as a Job
    // nothing is happening to, which is the exact reading this stream exists to
    // stop being wrong.
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
    let message: JournalMessage;
    try {
      message = JSON.parse(text) as JournalMessage;
    } catch {
      this.broke("Fleet sent a message this Bridge could not read.");
      return;
    }

    if (message.message === "opened") {
      // `skipped` is stated once, on the first message, and is the one fact a
      // reader needs before the first note: whether what follows is whole.
      this.log = { ...FRESH, skipped: message.skipped };
      this.publish({ state: "watching", jobId, log: this.log });
      return;
    }

    if (message.message === "note") {
      const { message: _tag, ...note } = message;
      const noted: Noted = { ...note, seq: this.seq++ };
      this.log = { ...this.log, notes: [...this.log.notes, noted] };
      this.publish({ state: "watching", jobId, log: this.log });
      return;
    }

    // `closed` carries why, and the socket is let go here rather than left to
    // close under its own event — Fleet sends `closed` and *then* closes, so a
    // listener still attached would overwrite the reason a reader wanted with
    // "the connection closed". `observe.ts` learned this the hard way.
    this.close();
    this.publish({ state: "ended", jobId, log: this.log, because: message.because });
  }

  private ended(because: string): void {
    const jobId = this.jobId;
    if (jobId === null || this.socket === null) return;
    this.socket.removeAllListeners();
    this.socket = null;
    this.publish({ state: "ended", jobId, log: this.log, because });
  }

  /**
   * The socket could not be read, and what it had read stays read.
   *
   * **The notes travel with the failure**, for the reason `observe.ts` states:
   * one unreadable frame used to empty a log that was full a moment before, and
   * the panel went back to reading as a Job nothing had happened to.
   */
  private broke(detail: string): void {
    const jobId = this.jobId;
    if (jobId === null) return;
    this.socket?.removeAllListeners();
    this.socket = null;
    this.publish({ state: "failed", jobId, log: this.log, detail });
  }
}
