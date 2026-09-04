// What `proposeFromRequest` answered, and which of two places it lands in.
//
// **Arithmetic, so it is tested as arithmetic.** Every case here is a function
// of the wire — no DOM to start and no story to write — and the whole value of
// `answeredAs` is the routing decision, which is invisible in a rendering.
//
// The three things worth pinning:
//
// - A refused request comes back. That claim is only true if the *echoed*
//   string is what the field is told to hold, and two of the four answers echo.
// - The two refusals are two drawings. One is a state on the surface and the
//   other is an `Outcome` the app draws where it draws every command failure,
//   and a regression here is silent — both look like "something went wrong".
// - Nothing mints a workflow name or a status. Both come off the wire, and a
//   proposal naming an id is a proposal nobody can check.

import { expect, test } from "vitest";
import type { BridgeIdentity, JobSummary, WireError, WorkflowSummary } from "@armada/protocol";

import { answeredAs } from "./proposal";
import type { Proposed } from "./proposal";

const BRIDGE: BridgeIdentity = { fleetProtocol: "5.2", auditPath: null };

const WORKFLOWS: WorkflowSummary[] = [
  { id: "wf_bug", name: "bug", version: 1, steps: [], manifest_id: "mf_1" },
  { id: "wf_feature", name: "feature", version: 1, steps: [], manifest_id: "mf_1" },
];

const SENT = "The board flickers every time an event lands.";

function seen() {
  return { sent: SENT, workflows: WORKFLOWS, bridge: BRIDGE, at: "2026-09-02T22:14:03Z" };
}

/** The shape `board.test.ts` builds, cut to what a proposal reads. */
function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "job_1",
    title: "Stop the board flickering",
    status: "awaiting_approval",
    workflow_id: "wf_bug",
    owner_manifest_id: "mf_1",
    origin: "proposer",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-02T22:14:03Z",
    ...over,
  };
}

function error(over: Partial<WireError> = {}): WireError {
  return {
    code: "fleet.model.budget_exhausted",
    message: "The proposer was not called: the model budget is spent.",
    run_id: "run_8f21c0",
    fields: { budget_window: "day" },
    chain: [],
    ...over,
  };
}

test("a proposal names the workflow, never its id", () => {
  const read = answeredAs({ ok: true, jobs: [job()] }, seen());
  expect(read.proposal).toEqual({
    at: "proposed",
    request: SENT,
    jobs: [
      {
        id: "job_1",
        title: "Stop the board flickering",
        workflow: "bug",
        status: "awaiting_approval",
      },
    ],
  });
  expect(read.outcome).toBeNull();
});

/**
 * A workflow Fleet no longer holds falls back to the id rather than to a blank.
 * A row naming nothing is a row that reads as a rendering bug.
 */
test("a workflow the roster does not hold falls back to its id", () => {
  const read = answeredAs({ ok: true, jobs: [job({ workflow_id: "wf_gone" })] }, seen());
  expect(read.proposal).toMatchObject({ jobs: [{ workflow: "wf_gone" }] });
});

/** The order the wire gave is the order drawn. It is the whole of the graph. */
test("several jobs keep the order they arrived in", () => {
  const read = answeredAs(
    {
      ok: true,
      jobs: [job({ id: "a" }), job({ id: "b" }), job({ id: "c", workflow_id: "wf_feature" })],
    },
    seen(),
  );
  expect(read.proposal).toMatchObject({ jobs: [{ id: "a" }, { id: "b" }, { id: "c" }] });
});

/**
 * The status is the job's own. A hardcoded `awaiting_approval` would look right
 * on every proposal and be a lie the day Fleet drafts one anywhere else.
 */
test("the badge reads the job's own status", () => {
  const read = answeredAs({ ok: true, jobs: [job({ status: "queued" })] }, seen());
  expect(read.proposal).toMatchObject({ jobs: [{ status: "queued" }] });
});

/**
 * No workflow resolved is a state on the surface, not an error. **It carries no
 * `Outcome`**, so the app draws nothing above it — Fleet answered and declined,
 * which is Armada working.
 */
test("no workflow resolved is drawn on the surface and echoes the request", () => {
  const answer: Proposed = {
    ok: false,
    why: "unresolved",
    request: SENT,
    outcome: { ok: false, why: "no_workflow" },
  };
  const read = answeredAs(answer, seen());
  expect(read.proposal).toEqual({ at: "unresolved" });
  expect(read.outcome).toBeNull();
  expect(read.request).toBe(SENT);
});

/**
 * The fault is the error treatment, with the code and the message off the wire.
 * The instant is the one passed in, so a payload quoted an hour later says when
 * it was taken rather than when it was read.
 */
test("a fault carrying a wire error is drawn inline, with its code", () => {
  const answer: Proposed = {
    ok: false,
    why: "faulted",
    request: SENT,
    outcome: { ok: false, why: "refused", error: error() },
  };
  const read = answeredAs(answer, seen());
  expect(read.proposal).toMatchObject({
    at: "faulted",
    code: "fleet.model.budget_exhausted",
    message: "The proposer was not called: the model budget is spent.",
  });
  expect(read.outcome).toBeNull();
  expect(read.request).toBe(SENT);
  const faulted = read.proposal;
  if (faulted.at !== "faulted") throw new Error("the fault was not drawn as one");
  expect(faulted.payload?.at).toBe("2026-09-02T22:14:03Z");
  expect(faulted.payload?.code).toBe("fleet.model.budget_exhausted");
});

/**
 * A fault with no code goes to the app's own failure pipeline. **Nothing here
 * mints one** — the `bridge.` namespace is declared beside the builder that
 * raises it, and a code invented at a call site is a second producer.
 */
test("a fault with no code goes back as an outcome", () => {
  const answer: Proposed = {
    ok: false,
    why: "faulted",
    request: SENT,
    outcome: {
      ok: false,
      why: "transport",
      detail: "socket closed",
      fault: { why: "unreachable", method: "POST", path: "/jobs/from_request" },
    },
  };
  const read = answeredAs(answer, seen());
  expect(read.proposal).toEqual({ at: "unasked" });
  expect(read.outcome).toEqual({
    ok: false,
    why: "transport",
    detail: "socket closed",
    // Carried whole. The fault is what the app's failure surface builds its
    // code, its route and its next step from, and a reading that dropped it
    // would put the old one-line message back.
    fault: { why: "unreachable", method: "POST", path: "/jobs/from_request" },
  });
  // Still echoed: the request survives a fault that said nothing about it.
  expect(read.request).toBe(SENT);
});

/**
 * A command refused before it was sent is not a proposer refusal at all, and it
 * reads exactly like an approval refused for the same reason.
 */
test("a refusal before sending leaves the field alone", () => {
  const answer: Proposed = {
    ok: false,
    why: "refused",
    outcome: { ok: false, why: "not_connected" },
  };
  const read = answeredAs(answer, seen());
  expect(read.proposal).toEqual({ at: "unasked" });
  expect(read.outcome).toEqual({ ok: false, why: "not_connected" });
  expect(read.request).toBeNull();
});
