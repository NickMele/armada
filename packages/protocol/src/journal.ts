// What Fleet did to a Job, as TypeScript sees it. `crates/ipc/src/journal.rs`.
//
// Its own file for the reason `turn.ts` has one: this is a socket, it is
// read-only all the way down, and nothing here has a request half.
//
// # The third voice, and it reached nowhere
//
// `ActivityLog` opens by saying it is one stream carrying the Drone's turns,
// Armada's injected turns and Fleet's own events. Two of the three travelled
// the Observe socket. The third was written to `.armada/logs/<job-id>.jsonl`
// and read by nothing — so a Job with no Drone on it, which is a Job having a
// worktree cut and its dependencies installed, drew a blank panel for however
// long that took.
//
// # Not a fourth `Saw`
//
// Folding these into that union would have been a variant this Bridge's switch
// has no arm for, which `docs/practices/protocol.md` makes a major bump. A DTO
// and a socket of their own are additive, and additive is what a Fleet ahead of
// Bridge is allowed to be.
//
// Hand-written like `turn.ts`, and a second statement of the Rust shapes for
// the same reason: the codegen that would emit both does not exist yet.

import type { Voice } from "./turn";
import type { ProtocolVersion } from "./version";

/** One message on a Job's log socket. `crates/ipc/src/journal.rs`. */
export type JournalMessage =
  | ({ message: "opened" } & JournalOpening)
  | ({ message: "note" } & LogNote)
  | ({ message: "closed" } & JournalClosed);

/**
 * The first message on every connection, before any note.
 *
 * **`JournalOpening`, not `Opened`** — `artifacts.ts` has an `Opened` and
 * `turn.ts` has an `Opening`, and this package is imported whole.
 */
export type JournalOpening = {
  protocol_version: ProtocolVersion;
  job_id: string;
  /** Older notes the bounded first read left out. Never a silent truncation. */
  skipped: number;
};

/** Nothing more is coming, and why. A socket that simply stops says nothing. */
export type JournalClosed = {
  /** `unreadable` — the log is there and Fleet could not read it. */
  because: string;
};

/**
 * How bad a note is, in the log envelope's own five.
 *
 * **Rendered, not branched on for meaning.** A surface picks a hue from it; a
 * spelling this Bridge has no hue for draws as an ordinary line rather than as
 * nothing, because the event happened either way.
 */
export type NoteLevel = "trace" | "debug" | "info" | "warn" | "error";

/** One name-and-value out of a note's fields. Values are always text. */
export type NotedField = {
  name: string;
  value: string;
};

/**
 * One line of a Job's own log, as a viewer is shown it.
 *
 * **`step` absent is the case this stream exists for**, and it is not a note
 * whose step is unknown. Fleet cutting a worktree, running a repository's
 * preparation commands or reclaiming a Job belongs to no step — there is no
 * step running yet — so a note with no step is the Job's own, and is drawn
 * above the steps rather than under the one about to start. Attaching it there
 * would read as a step that has begun when it has not, which is the confusion
 * that made a wedged Job look healthy.
 */
export type LogNote = {
  /** When Fleet wrote it. */
  at: string;
  /**
   * Whose note this is. **On the wire and never inferred from the route it
   * arrived on** — the whole point of folding three sources into one column is
   * that each one says which it is.
   */
  by: Voice;
  level: NoteLevel;
  /** The one line, as Fleet wrote it. Never carries an interpolated id. */
  msg: string;
  step?: string;
  drone?: string;
  /** What the note opens to. Absent is a note that carried nothing structured. */
  fields?: NotedField[];
};
