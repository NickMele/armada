// One Drone's turns, as rows a person reads: the Called/Answered join, and the
// wire's words where no vocabulary supplies one.
//
// **The join is Bridge's, and that is a decision rather than an oversight.**
// Fleet puts both events on the wire with their call ids because joining them
// there would mean holding a call open until its result arrived — unbounded
// buffering in the loop that advances the Job, in the path that must never
// block. Watching a Job cannot be allowed to change its outcome, so the cost of
// the join lands here, where the worst case is a row that reads "no answer yet".
//
// # What a run cost is drawn here and nowhere else
//
// `ended` carries a Drone run's turn count, its spend and its refusals, and the
// transcript is the only per-run surface Bridge has: a Job that retried has one
// `ended` per Drone, so a figure on the Job — a board column, an outcome row, a
// header fact — would be a total the wire deliberately declines to compute.
// It is also the only surface where the value can be present at all, because it
// arrives on the Observe socket, and the outcome regions of a finished or
// stopped Job are drawn with that socket closed. A labelled blank there would
// be worse than the absence it replaced.
//
// So the run says what it spent on its own last row, and that is still the only
// **per-run** figure there is.
//
// **The Job-level total is served now** — `JobDetail.spend`, since protocol
// 5.5, which `work.ts` draws against the Job's cap. What that closes is a
// person opening one Job and seeing what the whole of it has cost. What is
// still open is comparing Jobs **without** opening one: `JobSummary` carries no
// spend, so the board has no column and cannot have one until it does.

import type { DroneTurn, TurnStep } from "@armada/components";

import type { Turn } from "../../shared/bridge";
import type { JobDetail as JobWhole } from "../../shared/protocol";
import { clock } from "./duration";

/**
 * The rows, with each call carrying whatever came back for it.
 *
 * An `answered` never renders as a row of its own: it is folded into the
 * `called` that shares its id, and two rows would separate a command from its
 * output by everything that happened while it ran. An answer whose call was
 * never seen — the backfill was bounded and cut between them — keeps its own
 * row rather than being dropped, because a dropped row is a gap nobody can see.
 *
 * `whole` is the Job's frozen workflow, and it is what turns a `step_id` into
 * the name the rail already draws. `null` where the detail has not been read;
 * the id renders then, in mono, which is the rail's own fallback.
 */
export function turnsOf(rows: readonly Turn[], whole: JobWhole | null): DroneTurn[] {
  const answers = new Map<string, Answer>();
  for (const row of rows) {
    if (row.saw.event === "answered") answers.set(row.saw.call, { failed: row.saw.failed });
  }

  const seen = new Set<string>();
  const turns: DroneTurn[] = [];
  for (const row of rows) {
    const saw = row.saw;
    if (saw.event === "answered") {
      // Folded into its call, unless nothing here holds that call.
      if (seen.has(saw.call)) continue;
      turns.push({
        id: String(row.seq),
        at: clock(row.ts),
        kind: saw.event,
        step: stepOf(row, whole),
        subject: saw.call,
        answer: ANSWER[saw.failed ? "failed" : "ok"],
      });
      continue;
    }
    if (saw.event === "called") seen.add(saw.call);
    turns.push({
      id: String(row.seq),
      at: clock(row.ts),
      kind: saw.event,
      step: stepOf(row, whole),
      ...bodyOf(saw, answers),
    });
  }
  return turns;
}

/**
 * Which step a row ran under, named the way the rail names one.
 *
 * **A row carrying no step gets none, and never the first one.** Fleet began
 * recording the step on 2026-08-28 and wrote no migration, because the step an
 * older row ran under is unrecoverable — so an absence stays an absence and the
 * pane says so, rather than a transcript claiming a four-step Job happened
 * during step one.
 *
 * **The label is Fleet's and the id is the fallback**, which is `rail.ts`'s
 * rule for the same substitution: `StepDetail.label` is never blank because
 * Fleet substitutes the `step_id` where the workflow declares no name, and a
 * label that *is* the id renders in mono so a reader can tell. A step the
 * frozen workflow does not carry, or a detail nobody has read yet, takes the
 * same fallback — the id is what arrived, and nothing composes a name from it.
 */
function stepOf(row: Turn, whole: JobWhole | null): TurnStep | undefined {
  const id = row.step;
  if (id === undefined) return undefined;
  const label = whole?.steps.find((step) => step.step_id === id)?.label ?? id;
  return { id, label, labelIsAnIdentifier: label === id || undefined };
}

type Answer = { failed: boolean };

type Saw = Turn["saw"];

/**
 * What each kind puts in the row's body.
 *
 * **The kind itself is never translated.** `Saw` is the wire's enum and has no
 * `enum-verbs.toml` rows, so there is no sanctioned verb, glyph or hue for a
 * turn kind and none is written here — the spelling renders, which is
 * recoverable. Reported.
 */
function bodyOf(saw: Saw, answers: Map<string, Answer>): Omit<DroneTurn, "id" | "at" | "kind"> {
  switch (saw.event) {
    case "started":
      return { subject: `${saw.session} · ${saw.model} · ${count(saw.mcp_servers)}` };
    case "called": {
      const answer = answers.get(saw.call);
      const said = answer === undefined ? ANSWER.pending : ANSWER[answer.failed ? "failed" : "ok"];
      // With what the call did in hand, the call id stops being the only thing
      // telling one `Bash` row from the next and stops leading the row. Empty
      // is the wire saying the vocabulary had no name for this tool's
      // arguments — not a field that failed to arrive — and there the id is all
      // there is, so it stays.
      if (saw.detail === "") return { subject: `${saw.tool} · ${saw.call}`, answer: said };
      return { subject: saw.tool, detail: saw.detail, truncated: saw.truncated, answer: said };
    }
    case "said":
      return { said: saw.text };
    case "refused":
      return { subject: `${saw.tool} · ${saw.call}`, said: saw.because };
    // The Drone thinking. Collapsed by the pane into one line carrying the
    // count, because naming these by the decoder's failure to place them
    // describes the plumbing rather than what is happening.
    case "unrecognised":
      return { subject: saw.kind, quiet: true };
    case "unreadable":
      return { subject: saw.line, said: saw.why };
    // What the run cost, on the run's own last row. See `spent` for why the
    // figure hedges and the two counts do not.
    case "ended": {
      const ran = [turned(saw.turns), spent(saw.cost_micros), refused(saw.refusals)];
      return { subject: ran.join(" · ") };
    }
    // An `answered` reaching here is folded above; the arm exists so a kind
    // added to the wire fails to compile rather than rendering as a blank row.
    case "answered":
      return { subject: saw.call };
  }
}

/**
 * What an answer says. Three sentences, because the three are different facts:
 * the tool answered, the tool itself failed — which is not the Drone failing —
 * and the call is still running.
 */
const ANSWER = {
  ok: "Answered.",
  failed: "Answered, and the tool itself failed.",
  pending: "No answer yet.",
} as const;

/** `2 mcp servers`, and `1 mcp server`. */
function count(servers: number): string {
  return servers === 1 ? "1 mcp server" : `${servers} mcp servers`;
}

/** `41 turns`, and `1 turn`. */
function turned(turns: number): string {
  return turns === 1 ? "1 turn" : `${turns} turns`;
}

/**
 * What the run cost, from millionths of a dollar.
 *
 * **Hedged, because a spend figure is not an exit code.** The design contract's
 * P4 spells an estimated value `~$2.40` and never `$2.40`, and every spend
 * rendering in the component library already does: the harness reports the
 * number, nothing here has validated it, and a cost drawn with the authority of
 * a measured value teaches a reader to act on one that may be wrong.
 *
 * **Four places under a cent.** Two would round a run that cost a tenth of a
 * penny to `~$0.00`, which reads as free — and free is the one thing a spend
 * figure must never claim by accident.
 */
function spent(micros: number): string {
  const dollars = micros / 1_000_000;
  return `~$${dollars.toFixed(dollars > 0 && dollars < CENT ? 4 : 2)}`;
}

/** A cent, in dollars. Below this two decimal places round to nothing. */
const CENT = 0.01;

/**
 * How many of the run's calls the harness refused.
 *
 * **Zero is stated rather than left out.** It is the difference between a Drone
 * that was fenced off from what it needed and one that simply went quiet, and a
 * row that says nothing leaves a reader unable to tell "none" from "not
 * reported".
 */
function refused(refusals: number): string {
  if (refusals === 0) return "no calls refused";
  return refusals === 1 ? "1 call refused" : `${refusals} calls refused`;
}

