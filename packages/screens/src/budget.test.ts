// Which of the two ceilings caught a job, and what it is offered because of it.
//
// **The defect this file gates was on the screen, not on the wire.** A job held
// on turns and a job out of money both arrive as `queued` with
// `over_budget`. Until `budget_hold` there was one predicate for both, so a job
// stopped at its turn cap read `Over budget` and was offered `Raise the cost
// cap` — a control that sends a request Fleet answers, and leaves the job
// exactly as stopped as it was. Job `01M22TYSAE0023MADDP5ZQEYGW` is the case:
// 393 turns against 300, every Check passed, nothing on the screen able to
// start it.
//
// So what is pinned here is the exclusivity in both directions, the fallback
// where an older Fleet says nothing, and that the header reads the finer word.

import { expect, test } from "vitest";
import type { JobSummary } from "@armada/protocol";

import { heldForMoney, heldForTurns } from "./Acts";
import { readingOf } from "./reading";

/** A job stopped at a ceiling, with the ceiling left to each case. */
function stopped(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M22TYSAE0023MADDP5ZQEYGW",
    handle: "581-a-turn-cap-somebody-can-move",
    title: "Raise the turn cap",
    status: "queued",
    queued_reason: "over_budget",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-09T09:00:00Z",
    ...over,
  };
}

/** The verb the header opens with, whichever shape the reading took. */
function verb(job: JobSummary): string | null {
  return readingOf(job).verb ?? null;
}

test("a job held on money is offered the cost cap and not the turn cap", () => {
  const job = stopped({ budget_hold: "cost_cap" });

  expect(heldForMoney(job)).toBe(true);
  expect(heldForTurns(job)).toBe(false);
});

test("a job held on turns is offered the turn cap and not the cost cap", () => {
  const job = stopped({ budget_hold: "turn_cap" });

  expect(heldForTurns(job)).toBe(true);
  // The whole of the defect: this was `true`, and the control it offered could
  // not have started the job.
  expect(heldForMoney(job)).toBe(false);
});

/**
 * The cost cap is the ceiling that had a route before the field existed, so an
 * older Fleet falls to it. The other reading would offer a press that reaches a
 * route that Fleet does not serve.
 */
test("a fleet that says nothing falls to the cost cap, which is the older act", () => {
  const job = stopped();

  expect(heldForMoney(job)).toBe(true);
  expect(heldForTurns(job)).toBe(false);
});

/**
 * A third ceiling would have a third act. Offering either of the two that exist
 * would be a guess about which one caught the job.
 */
test("a ceiling this build has never heard of is offered neither control", () => {
  const job = stopped({ budget_hold: "wall_clock" });

  expect(heldForMoney(job)).toBe(false);
  expect(heldForTurns(job)).toBe(false);
});

test("neither control is offered on a job nothing is holding", () => {
  const job = stopped({ queued_reason: undefined, budget_hold: undefined });

  expect(heldForMoney(job)).toBe(false);
  expect(heldForTurns(job)).toBe(false);
});

test("the header says which ceiling caught it, rather than folding both to one word", () => {
  expect(verb(stopped({ budget_hold: "cost_cap" }))).toBe("over the cost cap");
  expect(verb(stopped({ budget_hold: "turn_cap" }))).toBe("over the turn cap");
});

test("the coarse word stays where the field is absent, because it is still true", () => {
  expect(verb(stopped())).toBe("over budget");
});

/**
 * The fallback every surface takes for a spelling this build has never heard
 * of: the registry's word for what it does know, rather than copy invented at
 * the call site.
 */
test("a hold this build cannot name reads as the reason it refines", () => {
  expect(verb(stopped({ budget_hold: "wall_clock" }))).toBe("over budget");
});
