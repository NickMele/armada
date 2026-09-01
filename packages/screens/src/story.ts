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

import type { Turn } from "@armada/protocol";
import type { ChangedFile, CheckRun } from "@armada/protocol";
import { CHECK_ADVANCES, CHECK_OUTCOME } from "@armada/components";
import { clock } from "./duration";

/** Who wrote the line. The wire's own three, as `LogEntry` spells them. */
export type LogActor = "armada" | "drone" | "fleet";

/**
 * One line of a payload. `named` is what the line *is* rather than how bad it
 * is — the echoed command, the result, the trailer saying where and how long.
 */
export type LogLine = { text: string; named?: "echo" | "passed" | "failed" | "meta" };

/** One row of the activity log, ready for `LogEntry`. */
export type LogRow = {
  /** Stable across re-renders. The socket's own sequence. */
  id: string;
  at: string;
  actor: LogActor;
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

/** What a step with no rows says. Ordinary, and never an error. */
export const NOTHING_YET_ON_THIS_STEP =
  "Nothing has happened on this step yet.";

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
  switch (saw.event) {
    case "instructed":
      return {
        id,
        at,
        actor,
        message: opening(saw.occasion),
        payload: lines(saw.text),
      };
    case "checked":
      return {
        id,
        at,
        actor,
        message: checked(saw.run),
        payload: ranTo(saw.run),
      };
    case "produced":
      return {
        id,
        at,
        actor,
        message: produced(saw.files),
        payload: saw.files.map((file) => ({ text: `${file.change}  ${file.path}` })),
      };
    case "started":
      return {
        id,
        at,
        actor,
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
        message: saw.failed ? "The call failed" : "The call answered",
        payload: [{ text: saw.call, named: saw.failed ? "failed" : "meta" }],
      };
    case "said":
      return { id, at, actor, message: saw.text, payload: lines(saw.text) };
    case "refused":
      return {
        id,
        at,
        actor,
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
        message: "A row this Bridge has no reading for",
        payload: [{ text: saw.kind, named: "meta" }],
      };
    default:
      return {
        id,
        at,
        actor,
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

/** A block of text as payload lines. The newlines are the author's. */
function lines(text: string): LogLine[] {
  return text.split("\n").map((line) => ({ text: line }));
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
