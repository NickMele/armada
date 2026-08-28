// One Drone's turns, as TypeScript sees them. `crates/ipc/src/turn.rs`.
//
// Its own file for the reason the Rust side gives it one: this is the only
// query whose transport is a socket, and it is read-only all the way down —
// nothing here has a request half, and nothing here reaches a Drone.
//
// Hand-written like `protocol.ts`, and a second statement of the Rust shapes
// for the same reason: the codegen that would emit both does not exist yet.

import type { Missed } from "./protocol";
import type { ProtocolVersion } from "./version";

/** One message on a Job's Observe socket. `crates/ipc/src/turn.rs`. */
export type TurnMessage =
  | ({ message: "opened" } & Opened)
  | ({ message: "row"; ts: string; step?: string } & Saw)
  | ({ message: "missed" } & Missed)
  | ({ message: "closed" } & Closed);

/** The first message on every connection, before any row. */
export type Opened = {
  protocol_version: ProtocolVersion;
  job_id: string;
  /** Whether a Drone was writing when this opened. `false` is ordinary. */
  live: boolean;
  /** Older rows the bounded backfill left out. Never a silent truncation. */
  skipped: number;
};

/** Nothing more is coming, and why. A socket that simply stops says nothing. */
export type Closed = {
  /** `drone_ended` or `nothing_writing`. */
  because: string;
};

/**
 * One row of a Drone's transcript, as a viewer is shown it.
 *
 * The step a row was written under travels beside this rather than inside it,
 * as `step` on the row message: it is true of every kind, and it is optional
 * because a row written before Fleet recorded one carries no step and nothing
 * can recover which it was.
 *
 * The tag is `event` and not `kind`, because `unrecognised` already carries a
 * `kind`. Three of the file's kinds never arrive here — `quota_moved`, `ended`
 * and the sink's own `missed` — so no case is written for them.
 */
export type Saw =
  | { event: "started"; session: string; model: string; mcp_servers: number }
  /**
   * The Drone reached for a tool, and what it reached for it with.
   *
   * **Both fields always arrive.** `crates/ipc/src/turn.rs` declares `detail`
   * and `truncated` on every `Saw::Called`, and the Fleet that predates them is
   * a Fleet behind, which Bridge refuses rather than reads. So neither is
   * optional here, and **empty is a value rather than an absence**: it means
   * the vocabulary had no name for that tool's arguments, which is what the
   * pane falls back to the call id for.
   *
   * The detail is bounded and may be cut — a `Write` argument is a whole file —
   * and `truncated` is how a row says so, because a command can legitimately
   * end in an ellipsis.
   */
  | {
      event: "called";
      tool: string;
      call: string;
      detail: string;
      truncated: boolean;
    }
  | { event: "answered"; call: string; failed: boolean }
  | { event: "said"; text: string }
  | { event: "refused"; tool: string; call: string; because: string }
  | { event: "unrecognised"; kind: string }
  | { event: "unreadable"; line: string; why: string };
