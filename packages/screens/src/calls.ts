// The rest of a cut call argument, fetched by the person who opened the row.
//
// **Held here rather than in `BridgeState`.** Every other read Bridge makes is
// published by main and kept current, because the thing it draws moves as the
// Job does. A recorded argument is finished the moment it is written, and it is
// asked for by one reader about one row — putting it in the published state
// would make one person opening a payload something the whole window re-renders
// on, and would keep a megabyte alive for as long as the Job is open.
//
// **Keyed by call id and held for the Job.** A call id is unique inside a Job,
// and the story draws the same row in two logs — chapter one's turns and
// chapter two's preview — so a fetch made in one is already answered in the
// other. Dropped when the Job changes, because a call id belongs to the Job it
// was recorded under.
//
// **Nothing is fetched on its own.** The control is the whole trigger: an
// argument big enough to need this route is the payload the socket is bounded
// to keep off the stream, and pre-fetching every cut row would spend on the
// screen exactly what the split was made to avoid.

import { useCallback, useEffect, useRef, useState } from "react";

import type { CallRead } from "@armada/protocol";

/**
 * What one call's arguments are, as this window has them.
 *
 * `undefined` — the state the map answers for a call nobody pressed — is a row
 * that has not been asked about, which is every row until somebody opens one.
 */
export type CallState =
  /** Asked for, nothing back yet. The control says so and does not send twice. */
  | { state: "fetching" }
  /** What the record holds, and whether that is the whole argument. */
  | { state: "got"; arguments: string; whole: boolean; length?: number }
  /**
   * Nothing came back, and the row says which in one sentence.
   *
   * **The row's own, never the screen's.** A call the record does not carry is
   * a 422: the Job is standing and the id names nothing in it, which is a thing
   * to say inside one payload rather than a state for a Job that is otherwise
   * being read perfectly well.
   */
  | { state: "absent"; note: string };

/** What a log needs to offer the rest of a row: what is held, and how to ask. */
export type Calls = {
  of: (callId: string) => CallState | undefined;
  fetch: (callId: string) => void;
};

/**
 * Hold one Job's fetched arguments.
 *
 * The Job id is a dependency rather than an argument to `fetch`, because what
 * is held is only meaningful under the Job it was read for — carried into the
 * next Job, a call id would name a call that Job never made.
 */
/**
 * Reading one call's arguments, as the screen's caller hands it in.
 *
 * **An argument, not a global.** What a call did is a reading; fetching it is
 * a round trip to a process that has the log on disk. The screen does the
 * first and is handed the second.
 */
export type ReadCall = (jobId: string, callId: string) => Promise<CallRead>;

export function useCallArguments(read: ReadCall, jobId: string): Calls {
  const [held, setHeld] = useState<Record<string, CallState>>({});
  useEffect(() => setHeld({}), [jobId]);

  // Read through a ref inside the callback so pressing does not re-create the
  // function on every answer. The log is rebuilt on every tick of the clock and
  // a prop that changed with it would remount nothing usefully.
  const current = useRef(held);
  current.current = held;

  const fetch = useCallback(
    (callId: string) => {
      // Already asked, or already answered. A second press while one is in
      // flight is the same request, and Fleet reads a file for each one.
      if (current.current[callId] !== undefined) return;
      setHeld((was) => ({ ...was, [callId]: { state: "fetching" } }));
      void read(jobId, callId).then(
        (read) => setHeld((was) => ({ ...was, [callId]: settled(read) })),
        // A rejected invoke is main gone, which is the window closing. Recorded
        // as an absence like any other so the row is not left saying `Fetching`
        // for the rest of its life.
        () => setHeld((was) => ({ ...was, [callId]: { state: "absent", note: NOT_ANSWERED } })),
      );
    },
    [read, jobId],
  );

  const of = useCallback((callId: string) => current.current[callId], []);
  return { of, fetch };
}

/**
 * What came back, as the row will draw it.
 *
 * **Two sentences and neither names the transport.** A refusal on this route is
 * the Job standing and the call not being in its transcripts — an id off a row
 * whose transcript has since been reclaimed — and that is a different thing to
 * know from Fleet not answering at all. Nothing here reasons about the seam,
 * and nothing here reads a code: a Job that had gone would have blanked the
 * panel around this row long before anybody pressed.
 */
function settled(read: CallRead): CallState {
  if (read.ok) {
    return {
      state: "got",
      arguments: read.call.arguments,
      whole: read.call.whole,
      ...(read.call.length === undefined ? {} : { length: read.call.length }),
    };
  }
  const refused = !read.outcome.ok && read.outcome.why === "refused";
  return { state: "absent", note: refused ? NOT_IN_THE_RECORD : NOT_ANSWERED };
}

/** The call is not in this Job's transcripts. The 422, in the app's voice. */
const NOT_IN_THE_RECORD = "This call is not in the record.";

/** Fleet did not answer. The same sentence the brief and the run already use. */
const NOT_ANSWERED = "Fleet did not answer for this call.";
