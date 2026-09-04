// The step's story, as one stream — every line in the order it happened, and
// every one of them naming who.
//
// # Three voices, and the socket carries all three
//
// A step is a conversation: Armada opens it with an instruction, the Drone
// works, and Fleet runs the Checks and reads what came out. The transcript
// stamps each row with which of the three wrote it, so the log renders the
// wire's own attribution rather than inferring one from the kind — `started`
// and `ended` are Fleet's record of a run opening and closing, and inferring
// that from the kind was as far as attribution could honestly go before.
//
// # A step is a boundary the socket already carries
//
// Every row since 2026-08-28 carries the step it ran under, so the log is
// filtered to the open step rather than being the Job's whole transcript under
// a step's heading. **A row with no step is kept**, because Fleet wrote no
// migration and the step an older row ran under is unrecoverable — dropping it
// would silently shorten the step's own story.
//
// # Every line opens, and what opens is what was sent
//
// Opening a row is the one gesture whose whole purpose is seeing the payload,
// so a payload that resolves to a sentence about the payload makes the
// affordance a lie. Where a call's arguments arrived cut, the row shows what
// was sent and says how much of it there is — never that Bridge was given
// nothing.
//
// **The size is the wire's and never the string's.** `detail_length` is what
// the argument had before anything was cut; `detail.length` is what survived
// the cut, so a row measuring itself reports 200 of 200 on an argument of
// fourteen thousand characters and calls the fetch pointless. Where the wire
// carries no size the row says nothing about size at all — an old transcript
// cannot recover one, and a number nobody measured is worse than no number.

import type { Journalled, Noted, Observed, Turn } from "@armada/protocol";
import type { ChangedFile, CheckRun } from "@armada/protocol";
import { CHECK_ADVANCES, CHECK_OUTCOME } from "@armada/components";
import { clock } from "./duration";

/** Who wrote the line. The wire's own three, as `LogEntry` spells them. */
export type LogActor = "armada" | "drone" | "fleet";

/**
 * One line of a payload. `named` is what the line *is* rather than how bad it
 * is — the echoed command, the result, the trailer saying where and how long.
 *
 * **`heading` says structure and the rest say outcome, and it is still one
 * field.** `echo`, `passed`, `failed` and `meta` are what a Check's run came
 * to; `heading` is a block heading in a turn Armada wrote. A renderer that
 * coloured by this field alone would paint `JOB BRIEF` as though something had
 * happened, so each surface reads the values it draws and leaves the rest as
 * body — the log does not hue a heading, and the brief does not hue a result.
 *
 * **It is the wire's word and never this file's.** `Saw.instructed` carries
 * `headings`, because only the code that wrote the block knows which line was
 * the heading; nothing here looks at a line's length or its capitals.
 */
export type LogLine = {
  text: string;
  named?: "echo" | "passed" | "failed" | "meta" | "heading";
};

/** One row of the activity log, ready for `LogEntry`. */
export type LogRow = {
  /** Stable across re-renders. The socket's own sequence. */
  id: string;
  at: string;
  actor: LogActor;
  /**
   * The wire's own kind for the row.
   *
   * **Carried so a surface can select rows without re-reading `saw`.** Chapter
   * one is the turns Fleet sent, which is `instructed` and not "everything in
   * Armada's voice" — the two stopped being the same set once a turn the
   * harness replays onto the Drone's stream started arriving correctly
   * attributed, and selecting by voice counted each of Armada's turns twice.
   *
   * **`note` is not one of the transcript's kinds**, and it is in this union
   * because both streams draw through one row type: it is a line of the Job's
   * own log, off the second socket, and every selection above is by transcript
   * kind so none of them claims one.
   */
  kind: Turn["saw"]["event"] | "note";
  /** The one line the row shows closed. */
  message: string;
  /** Whether `message` is machine-derived. Sans names work, mono names machinery. */
  mono?: boolean;
  /** The Drone still producing this line — the thinking marker. */
  working?: boolean;
  /** What the row opens to. Empty is a row that carried nothing to open. */
  payload: LogLine[];
  /**
   * The call whose argument the wire cut, where it cut one. Present makes the
   * open row offer the rest — `Log` fetches it and drops it into this payload.
   *
   * **Keyed off `truncated` and never off a size.** A row with no
   * `detail_length` is a transcript written before Fleet stamped one: it says
   * nothing about how much there is, and there is still more of it to fetch.
   */
  call?: CutCall;
};

/** A cut argument: which call, how much of it the row has, and how much there is. */
export type CutCall = {
  /** The call id, which is what `readCall` asks for. */
  id: string;
  /** Characters the row is showing — the cut string's own length. */
  shown: number;
  /** Characters the argument had. Absent on a pre-existing transcript. */
  length?: number;
};

/**
 * Whether a row is the echo PR 350 made visible, and never drawn.
 *
 * **Withheld the way `quota_moved` and `missed` are** — `crates/ipc/src/
 * turn.rs`'s `Shown::of` refuses those two before they leave Fleet, and this
 * is the same refusal one layer up: a row that arrives and is dropped before
 * it becomes a [`LogRow`], rather than a case `rowOf` special-cases.
 *
 * Fleet stamps a `said` row `armada` for exactly one reason — the harness
 * echoed a turn back off the Drone's own input channel — and Fleet owns that
 * channel outright: every text it ever puts there goes down as an
 * `instructed` row first, at the send site itself (`crates/fleet/src/
 * dispatch.rs`, `converging.rs`, `resume.rs`, `scope.rs`, `silence.rs`,
 * `questioning.rs`, `reviewing.rs`, `working.rs`, one call per occasion). So
 * an `armada`-voiced echo never arrives without its first-hand row already on
 * the transcript, and withholding it loses nothing — the text is on the
 * transcript from its author, which is `instructed`, not this row.
 *
 * **The Drone's own `said` is never withheld on this reasoning**, because
 * nothing else on the transcript is its author. #110 attributed it rather
 * than dropped it, and this is not a reversal of that: it is the one row a
 * Drone's prose has, kept, beside a second row of Fleet's own prose that had
 * two.
 */
function isEcho(row: Turn): boolean {
  return row.saw.event === "said" && row.by === "armada";
}

/** What a step with no rows says. Ordinary, and never an error. */
export const NOTHING_YET_ON_THIS_STEP =
  "Nothing has happened on this step yet.";

/**
 * What the socket says about its own reading, or nothing while it is reading.
 *
 * **The third answer, beside "no rows yet" and "rows".** Four of the five
 * states carry no live rows and none of them was drawn, so `Fleet is not
 * connected.` and `the connection closed` both rendered as
 * [`NOTHING_YET_ON_THIS_STEP`] — the sentence that means the step has not
 * started. A step that genuinely has not started must still read as one, which
 * is why this answers `undefined` rather than a sentence of its own while the
 * socket is carrying rows. #324.
 *
 * `whyNoFootprint` in `files.ts` is the precedent: a surface that cannot read
 * something says which reading failed.
 */
export function whyNotWatching(observed: Observed): string | undefined {
  switch (observed.state) {
    case "watching":
      return undefined;
    case "opening":
      return "Armada is opening this job's transcript.";
    case "none":
      return "Armada is not reading this job's transcript.";
    case "failed":
      // The detail is main's own sentence — which port, which peer, which
      // frame — and it is the half a reader can act on.
      return `The transcript could not be read. ${observed.detail}`;
    case "ended":
      return whyItEnded(observed.because);
  }
}

/**
 * Why the socket closed, in a sentence.
 *
 * **The two `Silence` variants are told apart**, because they are different
 * facts: one is a Drone that finished and one is a Job nothing was writing when
 * the socket opened. Anything else is a transport close, and it carries main's
 * own words rather than a word invented here — the reading `story.ts` gives an
 * unrecognised row.
 */
function whyItEnded(because: string): string {
  switch (because) {
    case "drone_ended":
      return "The drone that was writing this transcript has finished.";
    case "nothing_writing":
      return "No drone is writing this job's transcript.";
    default:
      return `The transcript stopped: ${because}.`;
  }
}

/**
 * The rows for one step.
 *
 * **Bounded by the socket, not here.** The backfill is bounded on Fleet's side
 * and `skipped` says how much it left out; this only says which rows belong to
 * the step being read.
 */
export function entriesOf(rows: readonly Turn[], stepId: string | undefined): LogRow[] {
  return rows
    .filter((row) => stepId === undefined || row.step === undefined || row.step === stepId)
    .filter((row) => !isEcho(row))
    .map(rowOf);
}

/**
 * One row as a line. **The kind is the wire's own word** — no vocabulary in the
 * repository carries a verb per turn kind, so the spelling renders rather than
 * copy invented here.
 */
function rowOf(row: Turn): LogRow {
  const saw = row.saw;
  const at = clock(row.ts);
  const id = String(row.seq);
  const actor = row.by;
  const kind = saw.event;
  switch (saw.event) {
    case "instructed":
      return {
        id,
        at,
        actor,
        kind,
        message: opening(saw.occasion),
        payload: lines(saw.text, saw.headings),
      };
    case "checked":
      return {
        id,
        at,
        actor,
        kind,
        message: checked(saw.run),
        payload: ranTo(saw.run),
      };
    case "produced":
      return {
        id,
        at,
        actor,
        kind,
        message: produced(saw.files),
        payload: saw.files.map((file) => ({ text: `${file.change}  ${file.path}` })),
      };
    case "started":
      return {
        id,
        at,
        actor,
        kind,
        message: `A Drone run opened on ${saw.model}`,
        payload: [
          { text: saw.session, named: "meta" },
          { text: `${saw.mcp_servers} mcp servers`, named: "meta" },
        ],
      };
    case "called": {
      const size = sizeOf(saw.detail.length, saw.detail_length);
      return {
        id,
        at,
        actor,
        kind,
        message: saw.detail === "" ? `${saw.tool}  ${saw.call}` : `${saw.tool}  ${saw.detail}`,
        mono: true,
        // What was sent, and how much of it there is where the wire said. The
        // row never reports an absence: `detail` is what arrived, and the size
        // is what tells a reader whether they are looking at all of it.
        payload:
          saw.detail === ""
            ? [{ text: saw.call, named: "meta" }]
            : [
                { text: saw.detail },
                ...(size === undefined ? [] : [{ text: size, named: "meta" as const }]),
              ],
        // Offered wherever the wire cut the argument, size or no size. A row
        // that arrived whole has nothing behind it and gets no control.
        ...(saw.truncated
          ? { call: { id: saw.call, shown: saw.detail.length, length: saw.detail_length } }
          : {}),
      };
    }
    case "answered":
      return {
        id,
        at,
        actor,
        kind,
        message: saw.failed ? "The call failed" : "The call answered",
        payload: [{ text: saw.call, named: saw.failed ? "failed" : "meta" }],
      };
    case "said":
      // **The only `said` row `entriesOf` still hands this function is the
      // Drone's own.** `isEcho` withholds an `armada`-voiced one before this
      // runs, so there is no second prose to name apart from — reading the
      // Drone's own words is the point of watching, and they are the message.
      return { id, at, actor, kind, message: saw.text, payload: lines(saw.text) };
    case "refused":
      return {
        id,
        at,
        actor,
        kind,
        message: `The harness refused ${saw.tool}`,
        payload: [
          { text: saw.because, named: "failed" },
          { text: saw.call, named: "meta" },
        ],
      };
    case "ended":
      return {
        id,
        at,
        actor,
        kind,
        message: "The Drone run ended",
        payload: [{ text: endedOf(saw.turns, saw.cost_micros, saw.refusals), named: "meta" }],
      };
    case "unrecognised":
      // A kind this Bridge has no case for. Drawn as itself rather than
      // dropped: a row nobody can read is a finding, and a missing row is not.
      return {
        id,
        at,
        actor,
        kind,
        message: "A row this Bridge has no reading for",
        payload: [{ text: saw.kind, named: "meta" }],
      };
    default:
      return {
        id,
        at,
        actor,
        kind,
        message: "A line the reader could not parse",
        payload: [{ text: saw.line }, { text: saw.why, named: "meta" }],
      };
  }
}

/**
 * What Armada's own turn was for, in the occasion's own word.
 *
 * **Spelled as `crates/fleet/src/session.rs` names the constructor that built
 * it.** No registry declares the set — it is decided by the code that sends
 * the turns — so an unknown occasion renders as itself rather than as a blank.
 */
function opening(occasion: string): string {
  switch (occasion) {
    case "opening":
      return "Armada opened the step.";
    case "outcome":
      return "Armada sent back what the gate came to.";
    case "redirect":
      return "Armada carried in a person's redirect.";
    case "answer":
      return "Armada carried in a person's answer.";
    case "drift":
      return "Armada said the work had gone outside the plan.";
    case "report":
      return "Armada asked for the step's evidence.";
    case "poke":
      return "Armada asked whether the Drone was still working.";
    default:
      return `Armada sent a ${occasion} turn.`;
  }
}

/** What one Check came to, in the registry's own verb. */
function checked(run: CheckRun): string {
  return `Check ${run.name} ${CHECK_OUTCOME[run.outcome]?.verb ?? run.outcome}`;
}

/** What the Check was measured against, and what it produced. */
function ranTo(run: CheckRun): LogLine[] {
  const passed = CHECK_ADVANCES[run.outcome] !== false;
  return [
    ...(run.expected === undefined ? [] : [{ text: run.expected, named: "echo" as const }]),
    ...(run.produced === undefined
      ? []
      : [{ text: run.produced, named: passed ? ("passed" as const) : ("failed" as const) }]),
    ...(run.output_path === undefined ? [] : [{ text: run.output_path, named: "meta" as const }]),
  ];
}

/** What the step wrote, counted. The names are the payload. */
function produced(files: ChangedFile[]): string {
  const outside = files.filter((file) => file.outside_plan === true).length;
  const counted = files.length === 1 ? "1 file" : `${files.length} files`;
  return outside === 0
    ? `The step's work read back — ${counted}`
    : `The step's work read back — ${counted}, ${outside} outside the plan`;
}

/**
 * A block of text as payload lines. The newlines are the author's.
 *
 * **`headed` is the wire's own list of line numbers and nothing here adds to
 * it.** A line is a heading because the code that wrote the block said so, so a
 * body line that happens to be short, or shouted, is body. An index the text is
 * too short for marks nothing, which is what an older row and a mismatched
 * pair both look like.
 */
function lines(text: string, headed: readonly number[] = []): LogLine[] {
  const headings = new Set(headed);
  return text
    .split("\n")
    .map((line, at) => (headings.has(at) ? { text: line, named: "heading" as const } : { text: line }));
}

/**
 * What a run cost, on its own last row.
 *
 * **The only per-run figure Bridge has, and it stays per run.** A Job that
 * retried has one `ended` per Drone, so a figure on the Job would be a total
 * the wire deliberately declines to compute.
 */
function endedOf(turns: number, costMicros: number, refusals: number): string {
  const said = [`${turns} turns`];
  if (refusals > 0) said.push(`${refusals} refusals`);
  said.push(`~$${(costMicros / 1_000_000).toFixed(2)}`);
  return said.join(" · ");
}

/**
 * *showing 200 of 14,320 characters*, or nothing where no size was carried.
 *
 * **Both numbers or neither.** A row that knows only what it is holding can say
 * `200 characters shown`, which reads as the whole of it — the sentence people
 * were given before the wire carried a true size, and the reason a cut argument
 * looked like a short one. Absent, the row says nothing about size and the
 * control beside it still offers the rest.
 *
 * Grouped, because five figures run together are read as a different number.
 */
export function sizeOf(shown: number, length: number | undefined): string | undefined {
  if (length === undefined) return undefined;
  return `showing ${shown.toLocaleString()} of ${length.toLocaleString()} characters`;
}

/**
 * The lines of a Job's own log, as rows — what Fleet did, off the second
 * socket.
 *
 * **One row type for both streams, deliberately.** A note and a turn are drawn
 * by the same component under the same rules — the instant, then who, then the
 * one line, then what it opens to — and a second row type would be a second
 * grammar in a column whose whole claim is that it has one.
 *
 * `by` is the wire's, exactly as it is for a turn. Nothing here infers Fleet
 * from the fact that this is Fleet's log: three voices in one column with
 * attribution inferred for one of them is the failure the component was written
 * against, one layer up.
 */
export function notesOf(notes: readonly Noted[]): LogRow[] {
  return notes.map((note) => ({
    id: `note-${note.seq}`,
    at: clock(note.at),
    actor: note.by,
    kind: "note" as const,
    message: note.msg,
    // The fields are what the row opens to, and a note with none opens to
    // nothing rather than to a sentence about itself. **Every entry opens to
    // its payload** is the component's rule; a Fleet event drawn as grey prose
    // in a column of openable rows is the second-class citizen this stream was
    // built not to be.
    payload: (note.fields ?? []).map((field) => ({
      text: `${field.name}  ${field.value}`,
      named: named(note.level),
    })),
  }));
}

/**
 * How a note's fields are named, from the level the line was written at.
 *
 * **Only the two levels that mean something went wrong are named.** `named` is
 * a hue on a payload line and most of what happens is not a verdict, so an
 * ordinary note draws as body — which is the same restraint `ranTo` shows about
 * a Check that passed.
 */
function named(level: Noted["level"]): LogLine["named"] {
  if (level === "error") return "failed";
  if (level === "warn") return "echo";
  return "meta";
}

/**
 * What the log socket says about its own reading, or nothing while it is
 * reading. [`whyNotWatching`]'s twin, and its sentences are about the Job's own
 * log rather than about a transcript.
 *
 * **A Job Fleet has done nothing to yet is not one of these.** That is
 * `watching` with no notes, and it draws the ordinary empty line — the state
 * these five exist to keep apart from it.
 */
export function whyNoNotes(journalled: Journalled): string | undefined {
  switch (journalled.state) {
    case "watching":
      return undefined;
    case "opening":
      return "Armada is opening this job's log.";
    case "none":
      return "Armada is not reading this job's log.";
    case "failed":
      return `This job's log could not be read. ${journalled.detail}`;
    case "ended":
      return journalled.because === "unreadable"
        ? "This job's log is there and Armada could not read it."
        : `The log stopped: ${journalled.because}.`;
  }
}

/** What a Job Fleet has recorded nothing about says. Ordinary, never an error. */
export const NOTHING_FROM_FLEET_YET = "Armada has not recorded anything about this job yet.";
