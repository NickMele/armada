// What a Check printed, fetched by the person who opened the Check.
//
// **`calls.ts`'s shape one record over, and for its reasons.** Every other read
// Bridge makes is published by main and kept current, because the thing it
// draws moves as the Job does. A recorded output is finished the moment the
// Check exited, and it is asked for by one reader about one Check — putting it
// in the published state would make one person opening a log something the
// whole window re-renders on, and would keep a test runner's whole output alive
// for as long as the Job is open.
//
// **Keyed by the row's own file name and held for the Job.** `fleet` builds
// that name out of the row's key — step, attempt, ordinal — so it names exactly
// one run of one Check inside one Job. Dropped when the Job changes, because a
// name belongs to the Job whose `.armada` directory holds it.
//
// **Nothing is fetched on its own.** Opening the chapter is the whole trigger:
// output is the payload the event stream is bounded to keep off itself, and
// pre-fetching every row that kept one would spend on the screen exactly what
// the split was made to avoid.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { ConsoleRegion, ConsoleRow } from "@armada/components";
import type { CheckOutput, CheckOutputRead, FollowedLog } from "@armada/protocol";

/**
 * What one Check's output is, as this window has it.
 *
 * `undefined` — what the map answers for a Check nobody opened — is a reading
 * that has not been asked for, which is every Check until somebody opens one.
 */
export type OutputState =
  /** Asked for, nothing back yet. The reader says so and does not send twice. */
  | { state: "fetching" }
  /** The window the record holds, and every number describing the file. */
  | { state: "got"; output: CheckOutput }
  /**
   * Nothing came back, and the reader says which in one sentence.
   *
   * **The Check's own, never the screen's.** An output the record does not hold
   * is a 422: the Job is standing and no row of it kept a file under that name,
   * which is a thing to say inside one reading rather than a state for a Job
   * that is otherwise being read perfectly well.
   */
  | { state: "absent"; note: string };

/** What a chapter needs to read a Check's output: what is held, and how to ask. */
export type Outputs = {
  of: (kept: string) => OutputState | undefined;
  fetch: (kept: string) => void;
};

/**
 * Reading one Check's output, as the screen's caller hands it in.
 *
 * **An argument, not a global**, which is `ReadCall`'s rule: what a Check
 * printed is a reading, and fetching it is a round trip to the process that has
 * the file on disk. The screen does the first and is handed the second.
 */
export type ReadCheckOutput = (jobId: string, kept: string) => Promise<CheckOutputRead>;

/**
 * Hold one Job's fetched Check outputs.
 *
 * The Job id is a dependency rather than an argument to `fetch`, because what
 * is held is only meaningful under the Job it was read for — carried into the
 * next Job, a file name would name a file that Job never wrote.
 */
export function useCheckOutputs(read: ReadCheckOutput, jobId: string): Outputs {
  const [held, setHeld] = useState<Record<string, OutputState>>({});
  useEffect(() => setHeld({}), [jobId]);

  // Read through a ref inside the callback so opening a chapter does not
  // re-create the function on every answer. The story is rebuilt on every tick
  // of the clock, and a prop that changed with it would remount nothing
  // usefully.
  const current = useRef(held);
  current.current = held;

  const fetch = useCallback(
    (kept: string) => {
      // Already asked, or already answered. A second open while one is in
      // flight is the same request, and Fleet reads a file for each one.
      if (current.current[kept] !== undefined) return;
      setHeld((was) => ({ ...was, [kept]: { state: "fetching" } }));
      void read(jobId, kept).then(
        (read) => setHeld((was) => ({ ...was, [kept]: settled(read) })),
        // A rejected invoke is main gone, which is the window closing. Recorded
        // as an absence like any other so the reader is not left saying
        // `Reading` for the rest of its life.
        () => setHeld((was) => ({ ...was, [kept]: { state: "absent", note: NOT_ANSWERED } })),
      );
    },
    [read, jobId],
  );

  const of = useCallback((kept: string) => current.current[kept], []);
  return { of, fetch };
}

/**
 * What came back, as the reader will draw it.
 *
 * **Two sentences and neither names the transport.** A refusal on this route is
 * the Job standing and no row of it holding an output under that name — a Job
 * whose `.armada` directory has been reclaimed — and that is a different thing
 * to know from Fleet not answering at all. Nothing here reasons about the seam:
 * a Job that had gone would have blanked the panel around this chapter long
 * before anybody opened it.
 */
function settled(read: CheckOutputRead): OutputState {
  if (read.ok) return { state: "got", output: read.output };
  const refused = !read.outcome.ok && read.outcome.why === "refused";
  return { state: "absent", note: refused ? NOT_IN_THE_RECORD : NOT_ANSWERED };
}

/** The file is not in this Job's record. The 422, in the app's voice. */
const NOT_IN_THE_RECORD = "This Check's output is no longer in the record.";

/** Fleet did not answer. The same sentence the brief and the run already use. */
const NOT_ANSWERED = "Fleet did not answer";

// ---------------------------------------------------------- what it draws as
//
// **The reading, beside the holding.** What comes back is a window and four
// numbers about the file it came from; these turn that into the rows and the
// region `ConsoleOutput` takes, and nothing else in the app spells either.

/**
 * The lines, as rows of the reading.
 *
 * **`at` is the file's own numbering**, which is the whole reason `from_line`
 * crosses: a region is a window onto a file and a citation names the file, so a
 * window that renumbered from one would send a reader to the wrong line.
 *
 * **No folds and no deletions.** A fold stands for a run of lines somebody
 * decided was uninteresting, and a deletion is a line that ran at the parent
 * commit and does not exist here — Armada produces neither. Nothing runs a
 * step's Checks at the commit its work branched from, and nothing marks a run
 * of output as skippable, so every row here is a line and says only what the
 * file says.
 */
export function rowsOf(output: CheckOutput): ConsoleRow[] {
  return output.lines.map((text, at) => ({
    row: "line" as const,
    at: output.from_line + at,
    text,
  }));
}

/**
 * Where the shown region sits in the file it came from.
 *
 * **A truncated reading says it is truncated.** `whole` is what decides which
 * sentence this is, and it is read rather than derived from the two counts
 * agreeing — the wire states it for that reason.
 *
 * The path is repo-relative, exactly as the row carries it, because that is
 * what a person pastes into a shell when the window is not enough. Nothing here
 * composes it.
 */
export function regionOf(output: CheckOutput): ConsoleRegion {
  const last = output.from_line + Math.max(output.lines.length - 1, 0);
  const where = output.whole
    ? `${output.total_lines.toLocaleString()} lines`
    : `lines ${output.from_line.toLocaleString()}–${last.toLocaleString()} of ` +
      `${output.total_lines.toLocaleString()}`;
  return {
    // **The Check's name leads, because a pane of output nobody can attribute
    // is a transcript.** It is the name off the answer rather than off the row
    // that was pressed, so it describes the file in front of the reader.
    says: `${output.name} · ${where}`,
    path: output.path,
    size: weighs(output.bytes),
  };
}

/** What the file weighs, at the precision a person reads rather than counts. */
function weighs(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * What a reading with nothing in it says, per state.
 *
 * **Four sentences and none of them is an empty frame.** A Check that printed
 * nothing, a read in flight, a read that came back with nothing to read, and a
 * read that could not be taken are four different things to know, and a surface
 * with one sentence for all of them says a Check printed nothing when what is
 * true is that nobody has asked yet.
 */
export function noteFor(held: OutputState | undefined): string {
  if (held === undefined) return NOT_ASKED;
  if (held.state === "fetching") return READING;
  if (held.state === "absent") return held.note;
  return PRINTED_NOTHING;
}

// ------------------------------------------------------ a log still growing
//
// **A running Check's log is the one output that moves**, so it is not held
// here: it is streamed by main over its own socket and published, the way the
// Job's own log is. What this half holds is which Check a person picked and how
// to ask main to follow it — `useCheckOutputs`' shape, for a reading that is
// not finished.

/**
 * Following one running Check's log, as the screen's caller hands it in.
 * `null` for both stops. An argument, not a global, for `ReadCheckOutput`'s
 * reason.
 */
export type FollowCheckOutput = (jobId: string | null, kept: string | null) => unknown;

/** What a chapter needs to show a running Check's log as it is written. */
export type Following = {
  /** The log main is following for this Job, or `none`. */
  reading: FollowedLog;
  /** The running Check a person picked, by its log's name, or `null` for none. */
  picked: string | null;
  pick: (kept: string) => void;
  /** Follow this log, or `null` to stop. */
  follow: (kept: string | null) => void;
};

/**
 * Hold which running Check's log a person is reading, for one Job.
 *
 * **Stopped when the Job changes and when the screen goes**, so a socket is
 * never left following a log nobody is looking at.
 */
export function useFollowing(
  follow: FollowCheckOutput | undefined,
  followed: FollowedLog,
  jobId: string,
): Following {
  const [picked, setPicked] = useState<string | null>(null);
  useEffect(() => setPicked(null), [jobId]);
  useEffect(
    () => () => {
      follow?.(null, null);
    },
    [follow, jobId],
  );
  const start = useCallback(
    (kept: string | null) => {
      follow?.(kept === null ? null : jobId, kept);
    },
    [follow, jobId],
  );
  const reading: FollowedLog =
    followed.state !== "none" && followed.jobId === jobId ? followed : { state: "none" };
  return useMemo(
    () => ({ reading, picked, pick: setPicked, follow: start }),
    [reading, picked, start],
  );
}

/** A followed log's lines, as rows numbered the way the file numbers them. */
export function liveRowsOf(reading: FollowedLog, kept: string): ConsoleRow[] {
  if (reading.state !== "following" || reading.kept !== kept) return [];
  return reading.lines.map((text, at) => ({ row: "line" as const, at: reading.fromLine + at, text }));
}

/**
 * Where a followed log's window sits, and whose log it is. **Says it is still
 * being written**, so a pane that has stopped moving because the Check is quiet
 * does not read as a finished one.
 */
export function liveRegionOf(reading: FollowedLog, kept: string): ConsoleRegion | undefined {
  if (reading.state !== "following" || reading.kept !== kept) return undefined;
  const count = reading.fromLine - 1 + reading.lines.length;
  const where = reading.ended === undefined ? "being written" : `${count.toLocaleString()} lines`;
  return { says: `${reading.name} · ${where}`, path: reading.path };
}

/** What a followed log with no lines says, per state. */
export function liveNoteFor(reading: FollowedLog, kept: string): string {
  if (reading.state === "failed" && reading.kept === kept) return NOT_ANSWERED_LIVE;
  if (reading.state !== "following" || reading.kept !== kept) return OPENING_LIVE;
  if (reading.ended === undefined) return NOTHING_PRINTED_YET;
  if (reading.ended === "finished") return PRINTED_NOTHING;
  if (reading.ended === "unreadable") return UNREADABLE_LIVE;
  return CLOSED_LIVE;
}

/** The socket is opening. */
const OPENING_LIVE = "Opening this Check's log…";

/** Following, and the Check has printed nothing so far. */
const NOTHING_PRINTED_YET = "This Check has printed nothing yet.";

/** Fleet said it could not read the file. */
const UNREADABLE_LIVE = "Fleet could not read this Check's log.";

/** The connection went without Fleet saying why. */
const CLOSED_LIVE = "The connection to this Check's log closed.";

/** Fleet did not answer the ask to follow it. */
const NOT_ANSWERED_LIVE = "Fleet did not answer for this Check's log.";

/** Before the chapter is opened. Nothing has been asked for. */
const NOT_ASKED = "Open this chapter to read what the Check printed.";

/** The read is in flight. */
const READING = "Reading the output…";

/** The file is there and empty — a Check that exited without printing. */
const PRINTED_NOTHING = "This Check printed nothing.";
