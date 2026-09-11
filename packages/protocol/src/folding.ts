// What a Job's two sockets come to, folded, and what a refused read says — the
// one place either is decided, for Bridge and for a recorded Job replayed.
//
// **Moved out of `apps/desktop/src/main`, and Storybook is why.** Bridge's main
// process was the only thing that turned Fleet's messages into the reads a
// screen renders, so a story could only be handed reads somebody had typed —
// and a typed read agrees with the fold only while its author remembers every
// rule below. A Job recorded off a real Fleet and replayed through these same
// functions cannot disagree with the app, because there is one fold.
//
// **Pure, and it imports nothing**, which is this package's rule. The sockets,
// the retries and which answer is current stay in main, beside the connection.
// What is here is what one message does to what was already held.

import type { JournalMessage } from "./journal";
import type { WireError } from "./protocol";
import type { JobLog, Journalled, Noted, Observed, Outcome, Turn, Turns } from "./reads";
import type { TurnMessage } from "./turn";

/** Nothing has arrived yet, and nothing has been lost. */
export const NO_TURNS: Turns = { live: false, skipped: 0, missed: 0, rows: [] };

/** Nothing has arrived yet, and nothing was left out. */
export const NO_NOTES: JobLog = { skipped: 0, notes: [] };

/**
 * Why a stream ended where Fleet never said — the transport closed under it.
 *
 * Named once, because a stream recorded without its `closed` message has to
 * end on the words Bridge would have drawn, and a second spelling would be a
 * replay that disagrees with the app by one sentence.
 */
export const SOCKET_CLOSED = "the connection closed";

/**
 * What one transcript message does to the turns held so far.
 *
 * `seq` is the row's identity, since none carries one, and counting it is the
 * caller's — once per row, per connection. `ended` is present where the
 * message closed the stream, and carries why.
 */
export function turnArrived(
  held: Turns,
  message: TurnMessage,
  seq: number,
): { turns: Turns; ended?: string } {
  if (message.message === "opened") {
    // `live` and `skipped` are stated once, on the first message, and are
    // the two facts a reader needs before the first row: whether anything is
    // still writing, and whether the history in front of them is whole.
    return { turns: { ...NO_TURNS, live: message.live, skipped: message.skipped } };
  }

  if (message.message === "row") {
    // `step` is named in the rest pattern rather than left to fall into it:
    // the wire carries it beside the row's kind, `Saw` declares no such
    // field, and a spread would put it on the union at runtime where no
    // reader can see it. It travelled that way, undrawn, until #160.
    // `by` is named for `step`'s reason and carries `drone` where it is
    // absent — every row written before Fleet stamped the field decoded from
    // a Drone's own output, so the default is the truth rather than a guess.
    const { message: _tag, ts, step, by, ...saw } = message;
    const row: Turn = { ts, seq, step, by: by ?? "drone", saw };
    return { turns: { ...held, rows: [...held.rows, row] } };
  }

  if (message.message === "missed") {
    // Counted and said, never skipped quietly: a transcript with a silent
    // gap reads as a Drone that went quiet, which is the one thing this
    // record exists to tell apart.
    return { turns: { ...held, missed: held.missed + message.dropped } };
  }

  // `closed` carries why, because a socket that simply stops is
  // indistinguishable from one that broke. The rows are kept.
  return { turns: { ...held, live: false }, ended: message.because };
}

/**
 * What one log message does to the notes held so far. `seq` and `ended` are
 * `turnArrived`'s, for the same reasons.
 */
export function noteArrived(
  held: JobLog,
  message: JournalMessage,
  seq: number,
): { log: JobLog; ended?: string } {
  if (message.message === "opened") {
    // `skipped` is stated once, on the first message, and is the one fact a
    // reader needs before the first note: whether what follows is whole.
    return { log: { ...NO_NOTES, skipped: message.skipped } };
  }

  if (message.message === "note") {
    const { message: _tag, ...note } = message;
    const noted: Noted = { ...note, seq };
    return { log: { ...held, notes: [...held.notes, noted] } };
  }

  // `closed` carries why, for the transcript's reason.
  return { log: held, ended: message.because };
}

/**
 * A recorded transcript, replayed into the read Bridge would be holding.
 *
 * **`open` is whether the socket was still up when the recording stopped.** A
 * running Job's stream has no end to replay, so it stays `watching`; one that
 * closed without a `closed` message ends the way Bridge's own socket does. With
 * nothing arrived and the socket up, it is `opening`, which is what Bridge draws
 * before the first message.
 */
export function observedFrom(jobId: string, messages: readonly TurnMessage[], open: boolean): Observed {
  if (messages.length === 0 && open) return { state: "opening", jobId };
  let turns = NO_TURNS;
  let seq = 0;
  for (const message of messages) {
    const next = turnArrived(turns, message, seq);
    if (message.message === "row") seq += 1;
    turns = next.turns;
    if (next.ended !== undefined) return { state: "ended", jobId, turns, because: next.ended };
  }
  return open
    ? { state: "watching", jobId, turns }
    : { state: "ended", jobId, turns: { ...turns, live: false }, because: SOCKET_CLOSED };
}

/** A recorded log, replayed. `open` is `observedFrom`'s. */
export function journalledFrom(
  jobId: string,
  messages: readonly JournalMessage[],
  open: boolean,
): Journalled {
  if (messages.length === 0 && open) return { state: "opening", jobId };
  let log = NO_NOTES;
  let seq = 0;
  for (const message of messages) {
    const next = noteArrived(log, message, seq);
    if (message.message === "note") seq += 1;
    log = next.log;
    if (next.ended !== undefined) return { state: "ended", jobId, log, because: next.ended };
  }
  return open ? { state: "watching", jobId, log } : { state: "ended", jobId, log, because: SOCKET_CLOSED };
}

/** The request an unreadable answer names, so the fault can say which. */
export type AskedRoute = { method: "GET" | "POST"; path: string };

/**
 * What an answer outside 2xx comes to.
 *
 * **A body that is a `WireError` is Fleet refusing**, and says so in its own
 * code. Anything else is reported as the transport failure it is, with the
 * status, rather than guessed at.
 */
export function refusedWith(status: number, text: string, asked: AskedRoute): Outcome {
  const error = wireErrorIn(text);
  return error === null
    ? {
        ok: false,
        why: "transport",
        detail: `Fleet answered ${status}`,
        fault: { ...asked, why: "unanswerable", status },
      }
    : { ok: false, why: "refused", error };
}

/**
 * A refusal, as the wire carries it, or `null` where the body is not one.
 *
 * **Nothing here mints a code.** A code's declaration lives beside the variant
 * that raises it and `cargo xtask verify-error-codes` collects them across both
 * languages, so a code invented here would be collected from nowhere and mean
 * nothing to the lookup Bridge does.
 */
function wireErrorIn(text: string): WireError | null {
  try {
    const parsed = JSON.parse(text) as WireError;
    if (typeof parsed.code === "string" && typeof parsed.message === "string") return parsed;
  } catch {
    return null;
  }
  return null;
}
