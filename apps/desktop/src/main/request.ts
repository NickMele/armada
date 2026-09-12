// The HTTP half of Bridge's one connection: one function, and the two shape
// checks its answers need.
//
// It sits beside `connection.ts` rather than inside it because the connection
// is a socket, a runtime file and a state machine, and a fetch with a timeout
// is none of those. Nothing here holds state, so nothing here can drift from
// what the socket believes.

import type {
  CallRead,
  CheckOutputRead,
  FrameRead,
  Holdings,
  Outcome,
  TransportFault,
} from "@armada/protocol";
import type { FleetCapacity, JobSummary, ManifestReading } from "@armada/protocol";
import type { ServerList } from "@armada/protocol";
import type { CallArguments, CheckOutput } from "@armada/protocol";
import type { ManifestSummary, ModelChoices, WorkflowSummary } from "@armada/protocol";
import { refusedWith } from "@armada/protocol";
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
 * How long a frame waits. **Longer than a command, for the one reason a bound
 * is ever raised here**: what is on the wire is a file rather than a row, and
 * `COMMAND_MS` is sized for a read that answers off the store in milliseconds.
 *
 * Fleet opens one file it has already checked is there, so the work is the
 * transfer — and the number a reader compares this against is `KeptFrame.bytes`
 * on the row they pressed, which is why the bound is generous rather than
 * derived from it. A screenshot that took thirty seconds to come back over
 * loopback is a machine in trouble, not a large image.
 */
const FRAME_MS = 30_000;

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
      return { ok: false, outcome: refusedWith(answer.status, text, asked) };
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
 * What Fleet's last read of `armada.yml` came to.
 *
 * **Two `null`s that mean the same thing on screen and are not the same fact.**
 * Fleet answers `null` where it has not re-read the file since it started, and
 * this answers `null` where the read itself failed. Both draw nothing, which is
 * right — there is no reading to report either way — and neither is worth a
 * third state a surface would have to say something about.
 *
 * Asked once per connection rather than on a timer: the reading changes when
 * somebody saves a file, and `manifest.reread` is what says so. This is for the
 * window that opened *after* the save, which an event cannot reach.
 */
export async function manifestReadingOf(port: number): Promise<ManifestReading | null> {
  const answer = await ask(port, "GET", "/manifest/reading");
  return answer.ok === true ? ((answer.body as ManifestReading | null) ?? null) : null;
}

/**
 * Every server Fleet holds — each Job's and the main checkout's — and the last
 * instance of each that ended.
 *
 * **`null` where Fleet did not answer**, `capacityOf`'s reason: a server's
 * phase is live and a stale list would draw a *Serving* row for one that has
 * since exited. Read once per connection, like the capacity and the Manifest
 * reading beside it — after that, `server.*` on `/events` carries each row
 * whole, so nothing here is re-fetched on a timer.
 */
export async function serversOf(port: number): Promise<ServerList | null> {
  const answer = await ask(port, "GET", "/servers");
  return answer.ok === true ? (answer.body as ServerList) : null;
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

/**
 * One Check's own output, read into the app.
 *
 * **`callArgumentsOf`'s shape one record over, and for its reasons.** A
 * recorded output cannot move, it is asked for by one reader about one Check,
 * and it is a test runner's whole log — so it is answered to the caller rather
 * than held by `reader.ts` and republished on every event.
 *
 * `kept` is the row's own file name, the last component of `CheckRun`'s
 * `output_path`. **Nothing here composes a path**: `artifacts.ts` owns that
 * rule, and Fleet resolves the name against its own record before it opens
 * anything.
 *
 * The refusal is carried through whole. On this route it is either the Job
 * being gone, which the panel is already saying, or no row of it holding an
 * output under that name — the Check's own business, said inside the reading.
 */
export async function checkOutputOf(
  port: number,
  jobId: string,
  kept: string,
): Promise<CheckOutputRead> {
  const answer = await ask(
    port,
    "GET",
    `/jobs/${encodeURIComponent(jobId)}/checks/${encodeURIComponent(kept)}/output`,
  );
  if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
  return { ok: true, output: answer.body as CheckOutput };
}

/**
 * One frame a step's harness produced, read into the app as the file it is.
 *
 * **The one read here that does not parse JSON**, and the reason is upstream:
 * an image has no window, so Fleet answers the file rather than an envelope
 * describing part of it. `ask` is not reused because its whole tail is
 * `JSON.parse` — what is shared is the refusal, which this asks it for by
 * reading the body twice over two paths rather than by branching inside it.
 *
 * **`kept` is two path segments and one id**, so it is split rather than
 * encoded whole: a frame's name is the harness's own and the run's directory in
 * front of it is what identifies a row. Nothing here composes it — Fleet handed
 * it over on the row, and this sends back what it was given.
 *
 * The bytes stop here on their way to a `Blob`. **The renderer never reaches
 * Fleet**, which is the rule every other read on this seam follows and the
 * reason a frame is fetched by main at all rather than put in an `img` tag
 * pointed at a port.
 */
export async function frameOf(port: number, jobId: string, kept: string): Promise<FrameRead> {
  const path = kept
    .split("/")
    .map((part) => encodeURIComponent(part))
    .join("/");
  const asked = { method: "GET" as const, path: `/jobs/${jobId}/frames/${path}` };
  try {
    const answer = await fetch(`http://${HOST}:${port}${asked.path}`, {
      method: "GET",
      signal: AbortSignal.timeout(FRAME_MS),
    });
    if (!answer.ok) {
      // The refusal is JSON even though the success is not, so it is read as
      // text here and handed to the same parser every other route uses.
      return { ok: false, outcome: refusedWith(answer.status, await answer.text(), asked) };
    }
    return {
      ok: true,
      bytes: new Uint8Array(await answer.arrayBuffer()),
      // What Fleet said it is, never what this guesses. Fleet reads the media
      // type off the file's own name and sends `nosniff` with it; a second
      // opinion here would be the one that is wrong.
      type: answer.headers.get("content-type") ?? "application/octet-stream",
    };
  } catch (cause) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    const fault: TransportFault =
      cause instanceof Error && cause.name === "TimeoutError"
        ? { ...asked, why: "timed_out", waitedMs: FRAME_MS }
        : { ...asked, why: "unreachable" };
    return { ok: false, outcome: { ok: false, why: "transport", detail, fault } };
  }
}
