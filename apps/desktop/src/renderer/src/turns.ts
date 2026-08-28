// One Drone's turns, as rows a person reads: the Called/Answered join, and the
// wire's words where no vocabulary supplies one.
//
// **The join is Bridge's, and that is a decision rather than an oversight.**
// Fleet puts both events on the wire with their call ids because joining them
// there would mean holding a call open until its result arrived — unbounded
// buffering in the loop that advances the Job, in the path that must never
// block. Watching a Job cannot be allowed to change its outcome, so the cost of
// the join lands here, where the worst case is a row that reads "no answer yet".

import type { DroneTurn } from "@armada/components";

import type { Turn } from "../../shared/bridge";
import { clock } from "./duration";

/**
 * The rows, with each call carrying whatever came back for it.
 *
 * An `answered` never renders as a row of its own: it is folded into the
 * `called` that shares its id, and two rows would separate a command from its
 * output by everything that happened while it ran. An answer whose call was
 * never seen — the backfill was bounded and cut between them — keeps its own
 * row rather than being dropped, because a dropped row is a gap nobody can see.
 */
export function turnsOf(rows: readonly Turn[]): DroneTurn[] {
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
        subject: saw.call,
        answer: ANSWER[saw.failed ? "failed" : "ok"],
      });
      continue;
    }
    if (saw.event === "called") seen.add(saw.call);
    turns.push({ id: String(row.seq), at: clock(row.ts), kind: saw.event, ...bodyOf(saw, answers) });
  }
  return turns;
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

