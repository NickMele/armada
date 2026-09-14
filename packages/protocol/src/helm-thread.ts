// What `GET /helm/observe`'s socket comes to, folded — the one place either is
// decided, `reads.ts`'s reason for `turnArrived`: a fold written once here
// cannot disagree with a fold written again at the call site, and a
// conversation recorded off a real Fleet replays through the same function a
// live one draws through.
//
// **A `HelmThreadItem` is one of four things a viewer is shown, and only one
// of them is a `Turn`.** `asked`, `fresh` and `unanswered` are facts about the
// conversation itself that a Drone's transcript never has to carry — nobody
// ever typed into a Job. Where a message *is* a Drone-shaped row, it becomes
// exactly the `Turn` `turn.ts` already declares, so a surface drawing Helm's
// reply reads it the way a step's own log already does.

import type { HelmMessage } from "./helm";
import type { Turn } from "./reads";

/** One thing a viewer of a Helm conversation is shown, in order. */
export type HelmThreadItem =
  /** What a person sent. */
  | { kind: "asked"; id: string; ts: string; text: string }
  /** One row of the session's own reply — `turn.ts`'s own shape. */
  | { kind: "row"; id: string; turn: Turn }
  /** The stored session could not be resumed; a new one wrote what follows. */
  | { kind: "fresh"; id: string; ts: string; because: string }
  /** No reply came, and why. */
  | { kind: "unanswered"; id: string; ts: string; why: string };

/**
 * One repository's Helm conversation, as a viewer holds it.
 *
 * **`open` carries a thread that may be empty**, unlike `Observed`'s
 * `watching`: a conversation nobody has asked anything of yet is a real state
 * — the dock at rest — and not the same as `opening`, which is the socket
 * still shaking hands.
 *
 * **`cleared` is its own state and not folded into `open` with no items.**
 * Start fresh answers at once and the old thread's `closed` message follows on
 * the socket; the moment between is a conversation a person just emptied on
 * purpose, which reads differently from one that has simply never been asked
 * anything.
 */
export type HelmThread =
  | { state: "none" }
  | { state: "opening"; manifestId: string }
  | {
      state: "open";
      manifestId: string;
      /** A reply is being written, or a message is waiting for one. */
      replying: boolean;
      /** Older messages the bounded backfill left out, from `opened`. */
      skipped: number;
      /** Messages this viewer fell behind and lost. */
      missed: number;
      items: HelmThreadItem[];
    }
  | { state: "cleared"; manifestId: string }
  | {
      state: "failed";
      manifestId: string;
      replying: boolean;
      skipped: number;
      missed: number;
      items: HelmThreadItem[];
      detail: string;
    };

/** Nothing is being watched. */
export const NO_HELM_THREAD: HelmThread = { state: "none" };

/** What the fold holds between messages — `HelmThread`'s `open` and `failed` states, without the tag. */
export type HeldHelmThread = {
  replying: boolean;
  skipped: number;
  missed: number;
  items: HelmThreadItem[];
};

/** Nothing has arrived yet, and nothing has been lost. */
export const NO_HELM_ITEMS: HeldHelmThread = { replying: false, skipped: 0, missed: 0, items: [] };

/**
 * What one message on the socket does to the thread held so far.
 *
 * `seq` is an item's own identity, since none carries one — counted by the
 * caller, once per item that is actually added; `added` says whether this
 * message was one, so a caller only advances its own counter then. `ended` is
 * present where Fleet forgot the thread this connection was showing, and
 * carries why — always `started_fresh` today.
 */
export function helmArrived(
  held: HeldHelmThread,
  message: HelmMessage,
  seq: number,
): { held: HeldHelmThread; added: boolean; ended?: string } {
  if (message.message === "opened") {
    return {
      held: { replying: message.replying, skipped: message.skipped, missed: 0, items: [] },
      added: false,
    };
  }

  if (message.message === "asked") {
    return {
      held: {
        ...held,
        replying: true,
        items: [...held.items, { kind: "asked", id: String(seq), ts: message.ts, text: message.text }],
      },
      added: true,
    };
  }

  if (message.message === "row") {
    const { message: _tag, ts, step, by, ...saw } = message;
    const turn: Turn = { ts, seq, step, by: by ?? "drone", saw };
    // `ended` is the last row a session writes for one reply; anything else
    // means it is still going.
    const replying = saw.event !== "ended";
    return {
      held: { ...held, replying, items: [...held.items, { kind: "row", id: String(seq), turn }] },
      added: true,
    };
  }

  if (message.message === "fresh") {
    return {
      held: {
        ...held,
        replying: true,
        items: [
          ...held.items,
          { kind: "fresh", id: String(seq), ts: message.ts, because: message.because },
        ],
      },
      added: true,
    };
  }

  if (message.message === "unanswered") {
    return {
      held: {
        ...held,
        replying: false,
        items: [...held.items, { kind: "unanswered", id: String(seq), ts: message.ts, why: message.why }],
      },
      added: true,
    };
  }

  if (message.message === "missed") {
    // Counted and said, never skipped quietly — `turnArrived`'s own reason.
    return { held: { ...held, missed: held.missed + message.dropped }, added: false };
  }

  // `closed`. The thread this connection was showing is gone; nothing here
  // reopens it — the socket's own owner does, onto the new, empty one.
  return { held, added: false, ended: message.because };
}
