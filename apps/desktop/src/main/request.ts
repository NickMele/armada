// The HTTP half of Bridge's one connection: one function, and the two shape
// checks its answers need.
//
// It sits beside `connection.ts` rather than inside it because the connection
// is a socket, a runtime file and a state machine, and a fetch with a timeout
// is none of those. Nothing here holds state, so nothing here can drift from
// what the socket believes.

import type { Holdings, Outcome } from "../shared/bridge";
import type { FleetCapacity, JobSummary, WireError } from "../shared/protocol";
import type { ManifestSummary, ModelChoices, WorkflowSummary } from "../shared/setup";
import { HOST } from "./runtime-file";

/** How long a command waits for an answer before it is a transport failure. */
const COMMAND_MS = 5000;

/** What came back: a body to read, or the refusal to render. */
export type Answer = { ok: true; body: unknown } | { ok: false; outcome: Outcome };

/** One request to Fleet. Loopback plus the port from the runtime file. */
export async function ask(
  port: number,
  method: "GET" | "POST",
  path: string,
  body?: unknown,
): Promise<Answer> {
  try {
    const answer = await fetch(`http://${HOST}:${port}${path}`, {
      method,
      headers: body === undefined ? undefined : { "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(COMMAND_MS),
    });
    const text = await answer.text();
    if (!answer.ok) {
      const error = refusal(text);
      return {
        ok: false,
        outcome:
          error === null
            ? { ok: false, why: "transport", detail: `Fleet answered ${answer.status}` }
            : { ok: false, why: "refused", error },
      };
    }
    return { ok: true, body: JSON.parse(text) };
  } catch (cause) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    return { ok: false, outcome: { ok: false, why: "transport", detail } };
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
