// The HTTP half of Bridge's one connection: one function, and the two shape
// checks its answers need.
//
// It sits beside `connection.ts` rather than inside it because the connection
// is a socket, a runtime file and a state machine, and a fetch with a timeout
// is none of those. Nothing here holds state, so nothing here can drift from
// what the socket believes.

import type { CallRead, Holdings, Outcome, TransportFault } from "@armada/protocol";
import type { FleetCapacity, JobSummary, WireError } from "@armada/protocol";
import type { CallArguments } from "@armada/protocol";
import type { ManifestSummary, ModelChoices, WorkflowSummary } from "@armada/protocol";
import { HOST } from "./runtime-file";

/** How long a command waits for an answer before it is a transport failure. */
const COMMAND_MS = 5000;

/**
 * What a route waits when a model call is inside the request.
 *
 * **Five seconds is a bound on a socket, not on a question.** Every route but
 * two answers off the store and returns in milliseconds, and `COMMAND_MS` is
 * sized for those. `POST /jobs/:id/rerun_gate` asks the Judge inside the
 * request, and Fleet's own bound on one call is `PROVISIONAL_JUDGE_BUDGET` in
 * `crates/armada/src/serve.rs` — two minutes, and a gate re-run may make
 * several. Bridge was giving it five seconds.
 *
 * **Deliberately longer than Fleet's bound**, so that Fleet's own refusal —
 * with a code, a run id and a chain — is what arrives rather than Bridge's
 * abort. Nothing generates this from the Rust constant; the two are coupled by
 * this comment.
 */
export const MODEL_CALL_MS = 150_000;

/**
 * No wait at all. **What `POST /jobs/from_request` takes**, and the only route
 * that does.
 *
 * A wait is a guess about how long is too long, made by the side that knows
 * least and enforced on somebody who was not asked. It was worth making while
 * Bridge had nothing else to offer — a socket held open forever with a blank
 * form in front of it is worse than an abort. It is not worth making now:
 * `proposal.moved` says how far the call has got and `stopProposal` kills it,
 * so the person watching decides, with Fleet's budget as the backstop.
 *
 * **Bridge giving up first is strictly worse than either.** It throws away
 * Fleet's coded answer, and it leaves the call running and spending with nobody
 * to read what it decides.
 */
export const NO_WAIT = 0;

/**
 * A route under one Job. The id is a path segment, so it is encoded.
 *
 * **Beside `ask` rather than beside the acts**, because two modules send acts
 * now: `command.ts` and `clearing.ts`. A copy in each is the same three lines
 * twice, and the one that drifts is whichever is edited second.
 */
export function route(jobId: string, operation: string): string {
  return `/jobs/${encodeURIComponent(jobId)}/${operation}`;
}

/** What came back: a body to read, or the refusal to render. */
export type Answer = { ok: true; body: unknown } | { ok: false; outcome: Outcome };

/**
 * One request to Fleet. Loopback plus the port from the runtime file.
 *
 * `waitMs` is the wait, and it is a parameter rather than one constant because
 * two routes put a model call inside the request — see [`MODEL_CALL_MS`]. Every
 * other caller takes the default and is right to.
 *
 * **Every failure here names the route it was about.** A transport failure
 * carries no `WireError`, so what a person quoting it has is whatever this
 * builds: which of the three it was, what was asked, and how long Bridge
 * waited. See `TransportFault` for why the three are three.
 */
export async function ask(
  port: number,
  method: "GET" | "POST",
  path: string,
  body?: unknown,
  waitMs: number = COMMAND_MS,
): Promise<Answer> {
  const asked = { method, path };
  try {
    const answer = await fetch(`http://${HOST}:${port}${path}`, {
      method,
      headers: body === undefined ? undefined : { "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      // Zero is no signal at all rather than an immediate abort — see
      // `NO_WAIT`. `AbortSignal.timeout(0)` would fire on the next tick.
      ...(waitMs === NO_WAIT ? {} : { signal: AbortSignal.timeout(waitMs) }),
    });
    const text = await answer.text();
    if (!answer.ok) {
      const error = refusal(text);
      return {
        ok: false,
        outcome:
          error === null
            ? {
                ok: false,
                why: "transport",
                detail: `Fleet answered ${answer.status}`,
                fault: { ...asked, why: "unanswerable", status: answer.status },
              }
            : { ok: false, why: "refused", error },
      };
    }
    return { ok: true, body: JSON.parse(text) };
  } catch (cause) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    // `AbortSignal.timeout` rejects with a `TimeoutError`, and it is the one
    // failure here that says the request went out. Told apart by name rather
    // than by the message, which is the vendor's wording and not a contract.
    const fault: TransportFault =
      cause instanceof Error && cause.name === "TimeoutError"
        ? { ...asked, why: "timed_out", waitedMs: waitMs }
        : { ...asked, why: "unreachable" };
    return { ok: false, outcome: { ok: false, why: "transport", detail, fault } };
  }
}

/**
 * A refusal, as the wire carries it, or `null` where the body is not one.
 *
 * **Nothing here mints a code.** A code's declaration lives beside the variant
 * that raises it and `cargo xtask verify-error-codes` collects them, so a code
 * invented in Bridge would be in no manifest and mean nothing to the lookup
 * Bridge does. A body that is not a `WireError` is reported as the transport
 * failure it is.
 */
function refusal(text: string): WireError | null {
  try {
    const parsed = JSON.parse(text) as WireError;
    if (typeof parsed.code === "string" && typeof parsed.message === "string") return parsed;
  } catch {
    return null;
  }
  return null;
}

/**
 * Whether a body is a Job row. Two fields, not the whole shape: a full check
 * would be a third statement of a type `crates/ipc` already owns and the
 * codegen has yet to emit — this only has to tell a Job from a route that
 * answered something else.
 */
export function isJobSummary(body: unknown): body is JobSummary {
  if (typeof body !== "object" || body === null) return false;
  const row = body as Partial<JobSummary>;
  return typeof row.id === "string" && typeof row.status === "string";
}

/**
 * The workflows, the Manifests and the models Fleet holds. Three calls, being
 * three operations in the inventory.
 *
 * A failed one keeps what was already held: a stale roster beats none, and
 * Fleet refuses an id that has gone, so the worst case is a picker offering one
 * value too many rather than a form with nothing in it.
 */
export async function holdingsOf(port: number, held: Holdings): Promise<Holdings> {
  const [workflows, manifests, models] = await Promise.all([
    ask(port, "GET", "/workflows"),
    ask(port, "GET", "/manifests"),
    ask(port, "GET", "/models"),
  ]);
  return {
    workflows: workflows.ok === true ? (workflows.body as WorkflowSummary[]) : held.workflows,
    manifests: manifests.ok === true ? (manifests.body as ManifestSummary[]) : held.manifests,
    models: models.ok === true ? (models.body as ModelChoices) : held.models,
  };
}

/**
 * How full the fleet is. **`null` where Fleet did not answer**, rather than the
 * last reading kept — unlike `holdingsOf` above, and for the opposite reason:
 * a workflow roster is stable and a stale one is nearly right, and this is a
 * live count that is wrong the moment a Job moves. A bar that keeps drawing
 * "2 of 2" off an answer it could not get is saying something it does not know.
 */
export async function capacityOf(port: number): Promise<FleetCapacity | null> {
  const answer = await ask(port, "GET", "/capacity");
  return answer.ok === true ? (answer.body as FleetCapacity) : null;
}

/**
 * One recorded tool call's arguments.
 *
 * **A read that answers rather than one that is held.** `reader.ts` exists for
 * the reads a Job moving invalidates, and its whole job is dropping an answer
 * whose id moved while it was in flight. A recorded argument cannot move and is
 * asked for by one reader about one row, so there is no id to check it against
 * and nothing to keep — which is why this sits here beside `capacityOf` rather
 * than becoming a fifth `JobReader`.
 *
 * The refusal is carried through whole. On this route it is either the Job
 * being gone, which the panel is already saying, or the call not being in its
 * transcripts, which is the row's own business — so the caller decides what to
 * say and nothing here turns one into a screen.
 */
export async function callArgumentsOf(
  port: number,
  jobId: string,
  callId: string,
): Promise<CallRead> {
  const answer = await ask(
    port,
    "GET",
    `/jobs/${encodeURIComponent(jobId)}/calls/${encodeURIComponent(callId)}`,
  );
  if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
  return { ok: true, call: answer.body as CallArguments };
}
