// The dispatch path where a person describes the work and the Job proposer
// reads it — the request, the plan that comes back, and which of the two
// refusals Fleet answered with.
//
// Beside `command.ts` rather than inside it for the reason `review.ts` is: it
// is one act whose whole difficulty is what comes back, and that file is at the
// length the gate warns about. `JobCommands.proposeFromRequest` delegates here
// in a line, the way `settleWork` delegates to `decide`.
//
// It holds no state and no connection. The `Board` it is handed is what any act
// needs — where to send, and what to do with the Jobs that come back — so
// nothing here can drift from what the socket believes.

import type { Outcome, Proposed } from "@armada/protocol";
import type { JobRequest, ProposedPlan } from "@armada/protocol";
// Read, never minted: Fleet's own spellings, and the only thing on the wire
// that tells a declined request from a call that could not be made.
import { FLEET_FAULT, NO_WORKFLOW_FITS, PROPOSER_UNREACHABLE } from "@armada/protocol";
import { ask, isJobSummary } from "./request";
// Type-only, and therefore not a cycle at runtime: `Board` is what an act needs
// of the connection, and it is declared where the acts are.
import type { Board } from "./command";

/**
 * Describe the work and let the Job proposer decide what it is: which workflow,
 * what to call it, and whether it is one Job or several.
 *
 * **The same 201 and the same approval gate as `proposeJob`** — what differs is
 * who filled the workflow in, and that one request can be several Jobs. Nothing
 * here is running: every member comes back at `awaiting_approval` and each takes
 * its own approval in turn.
 *
 * Blank is refused before the request is sent, matching the 422 Fleet would
 * give it, and trimmed for the reason `proposeJob` trims a title: Fleet trims
 * before it reads, so padding makes what comes back on a refusal differ from
 * what was sent.
 *
 * **No in-flight guard, matching `proposeJob`.** Nothing here is an act on a
 * Job, so there is no id to key one on — a second dispatch is a second request,
 * and the form is what stops a double press.
 */
export async function proposeFromRequest(board: Board, request: string): Promise<Proposed> {
  const said = request.trim();
  if (said === "") return { ok: false, why: "refused", outcome: { ok: false, why: "empty_brief" } };
  const port = board.port();
  if (port === null) {
    return { ok: false, why: "refused", outcome: { ok: false, why: "not_connected" } };
  }

  const body: JobRequest = { request: said };
  const answer = await ask(port, "POST", "/jobs/from_request", body);
  if (answer.ok !== true) return notProposed(said, answer.outcome);

  const plan = answer.body as ProposedPlan;
  const proposed = Array.isArray(plan?.jobs) ? plan.jobs : [];
  const jobs = proposed.filter(isJobSummary);
  for (const job of jobs) board.fold(job);
  // A member that is not a row would put a malformed Job on the board, so what
  // did not fold is re-read rather than dropped. **Still a success**: Fleet
  // answered 201 and the Jobs exist, and a refusal here would send somebody to
  // dispatch the same work a second time — the argument `fileReport` already
  // makes about a filing that happened.
  if (jobs.length !== proposed.length) await board.reread(port);
  return { ok: true, jobs };
}

/**
 * Which of the two refusals `propose_from_request` answered with.
 *
 * **The one place Bridge branches on an error code rather than rendering it**,
 * and the reason is that nothing else on the wire separates these two: `ask`
 * maps a body to a `WireError` and keeps no status, and both refusals are a
 * refusal carrying a request. `crates/fleet/src/refusing.rs` declares the codes
 * apart for exactly this — a client that rendered the outage as "nothing fits"
 * would tell a person their request was refused when it was never read.
 *
 * A code nobody here matches is a fault, which advises asking again. That is
 * the safe side to fail to: it never tells somebody their request was declined
 * on the strength of a code this file did not recognise.
 */
function notProposed(request: string, outcome: Outcome): Proposed {
  if (outcome.ok) return { ok: false, why: "refused", outcome };
  // No answer at all. The request was never read, so it is the fault arm, and
  // what comes back is what Bridge sent — there is no envelope to read it off.
  if (outcome.why === "transport") return { ok: false, why: "faulted", request, outcome };
  if (outcome.why !== "refused") return { ok: false, why: "refused", outcome };
  // Fleet returns the request unchanged on the envelope's own field, on both
  // refusals. What was sent is the fallback and not the value: what a person
  // retypes should be the string the proposer actually read.
  const returned = outcome.error.fields["request"];
  const carried = typeof returned === "string" ? returned : request;
  switch (outcome.error.code) {
    case NO_WORKFLOW_FITS:
      return { ok: false, why: "unresolved", request: carried, outcome };
    case PROPOSER_UNREACHABLE:
    case FLEET_FAULT:
      return { ok: false, why: "faulted", request: carried, outcome };
    default:
      return { ok: false, why: "refused", outcome };
  }
}
