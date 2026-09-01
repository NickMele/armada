// The step's story, as one stream — the entries the Observe socket carries,
// with every one of them naming who.
//
// # One voice arrives where the design draws three
//
// The design's activity log carries the Drone's turns, Armada's injected turns
// and Fleet's own events in one stream. **Only the first is on this seam.** The
// Observe socket is a Drone's transcript: `crates/ipc/src/turn.rs` declares
// `started`, `called`, `answered`, `said`, `refused`, `ended` and
// `unrecognised`, and none of them is a Check result, a heartbeat or an
// injected turn. So the stream says so in its own cut line rather than drawing
// one voice and letting it read as three.
//
// Two of the seven are not the Drone talking — `started` and `ended` are the
// harness's record of a run opening and closing, written by Fleet — so those
// two name Fleet, which is the whole of what attribution can honestly say here.
//
// # A step is a boundary the socket already carries
//
// Every row since 2026-08-28 carries the step it ran under, so the log is
// filtered to the open step rather than being the Job's whole transcript under
// a step's heading. **A row with no step is kept and marked**, because Fleet
// wrote no migration and the step an older row ran under is unrecoverable —
// dropping it would silently shorten the step's own story.

import type { ActivityEntry } from "@armada/components";

import type { Turn } from "../../shared/bridge";
import { clock } from "./duration";

/** What the stream is missing, named where a cut is named. */
export const NOT_ONE_STREAM =
  "The Drone's turns only. Armada's injected turns and Fleet's own events — a Check result, a " +
  "heartbeat — are not on the Observe socket, so they are not in this stream.";

/** What a step with no rows says. Ordinary, and never an error. */
export const NOTHING_YET_ON_THIS_STEP =
  "Nothing has been recorded against this step yet. A Drone that has just started has taken no turns.";

/** What a Job that nothing is watching says, in the log's own place. */
export const NOT_WATCHING =
  "This Job's turns are not being read. Open them to fill this, or read the transcript named under " +
  "Where things are.";

/**
 * The entries for one step.
 *
 * **Bounded by the socket, not here.** The backfill is bounded on Fleet's side
 * and `skipped` says how much it left out; this only says which rows belong to
 * the step being read.
 */
export function entriesOf(rows: readonly Turn[], stepId: string | undefined): ActivityEntry[] {
  return rows
    .filter((row) => stepId === undefined || row.step === undefined || row.step === stepId)
    .map(entryOf);
}

/**
 * One row as an entry. **The kind is the wire's own word** — no vocabulary in
 * the repository carries a verb per turn kind, so the spelling renders rather
 * than copy invented here.
 */
function entryOf(row: Turn): ActivityEntry {
  const saw = row.saw;
  const at = clock(row.ts);
  const id = String(row.seq);
  switch (saw.event) {
    case "started":
      return {
        id,
        at,
        actor: "fleet",
        summary: `A Drone run opened on ${saw.model}`,
        subject: saw.session,
      };
    case "called":
      return {
        id,
        at,
        actor: "drone",
        summary: saw.tool,
        subject: saw.detail === "" ? undefined : saw.detail,
        // The whole of what the row carries. `truncated` says the detail was
        // cut upstream, which is a different cut from the log's own and is why
        // it is named rather than folded into the payload's line count.
        payload: saw.truncated ? "The call's arguments were cut before Bridge saw them." : undefined,
      };
    case "answered":
      return {
        id,
        at,
        actor: "drone",
        summary: saw.failed ? "The call failed" : "The call answered",
        subject: saw.call,
        named: saw.failed ? "failed" : undefined,
      };
    case "said":
      return { id, at, actor: "drone", summary: saw.text };
    case "refused":
      return {
        id,
        at,
        actor: "fleet",
        summary: `The harness refused ${saw.tool} — ${saw.because}`,
        subject: saw.call,
        named: "refused",
      };
    case "ended":
      return {
        id,
        at,
        actor: "fleet",
        summary: "The Drone run ended",
        ran: endedOf(saw.turns, saw.cost_micros, saw.refusals),
      };
    case "unrecognised":
      return {
        id,
        at,
        actor: "fleet",
        // A kind this Bridge has no case for. Drawn as itself rather than
        // dropped: a row nobody can read is a finding, and a missing row is not.
        summary: "A row this Bridge has no reading for",
        subject: saw.kind,
      };
    default:
      return {
        id,
        at,
        actor: "fleet",
        summary: "A line the reader could not parse",
        subject: saw.why,
        output: saw.line,
      };
  }
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
