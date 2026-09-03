// Which of the four renders a Job takes, and the one reason it takes the
// dead-end one.
//
// **The order of the tests in `renderFor` is the whole of the module**, and it
// is what this pins: escalation before everything, review before working,
// terminal-and-successful last. Each of those was a real defect once — a Job
// waiting on a person drawn with a live rail, a stopped Job drawn as if it were
// still going — and none of them is visible from the type.

import { describe, expect, it } from "vitest";

import { escalation, renderFor } from "./render";

import type { JobSummary } from "@armada/protocol";

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    ...over,
  };
}

describe("which render a job takes", () => {
  it("draws a moving job as working", () => {
    expect(renderFor(job({ status: "running" }))).toBe("working");
    expect(renderFor(job({ status: "queued" }))).toBe("working");
    expect(renderFor(job({ status: "piloted" }))).toBe("working");
  });

  it("draws a job at the review gate as reviewing, never as working", () => {
    // `awaiting_review` is non-terminal, so without its own rule it takes the
    // running render: a live rail and a per-step elapsed on a job that has
    // stopped and is waiting on a person.
    expect(renderFor(job({ status: "awaiting_review" }))).toBe("reviewing");
  });

  it("draws the one successful terminal status as finished", () => {
    expect(renderFor(job({ status: "completed_success" }))).toBe("finished");
  });

  it("draws every other terminal status as stopped", () => {
    for (const status of ["completed_failed", "killed", "rejected", "superseded"]) {
      expect(renderFor(job({ status })), status).toBe("stopped");
    }
  });

  it("draws an escalated job as stopped, because the dead end is the screen that says why", () => {
    expect(
      renderFor(job({ status: "escalated", reason: { named: "gate_failure" } })),
    ).toBe("stopped");
  });

  it("refuses to draw a status this build has no vocabulary for", () => {
    // Unrenderable is a real answer. Falling back to `working` would draw a
    // live rail over a status nothing in this build can name.
    expect(renderFor(job({ status: "translated_into_greek" }))).toBe("unrenderable");
  });
});

describe("the escalation reason", () => {
  it("reads the registry's rendering for a reason it holds", () => {
    const held = escalation(job({ status: "escalated", reason: { named: "gate_failure" } }));
    expect(held?.verb).toBe("stopped at the gate");
  });

  it("carries no reason where the job is not escalated", () => {
    // A `reason` travels on transitions that are not escalations too, and
    // reading one off a running job would send that job to the dead-end render.
    expect(escalation(job({ status: "running", reason: { named: "gate_failure" } }))).toBeUndefined();
    expect(renderFor(job({ status: "running", reason: { named: "gate_failure" } }))).toBe("working");
  });

  it("carries no reason where the registry has no such spelling", () => {
    expect(escalation(job({ status: "escalated", reason: { named: "sat_down" } }))).toBeUndefined();
  });

  /**
   * `escalated` is the one status where the render turns on the reason, not
   * just the status. Where the reason the job carries is a spelling
   * `ESCALATION_REASON` has no row for — or the job arrives with no `reason`
   * at all, which the wire permits — the dead-end render has nothing to
   * state. `unrenderable` says this build cannot describe the job; falling
   * back to `working` would draw a live rail and a running clock over a job
   * that has stopped and is waiting on a person.
   */
  it("draws an escalated job with no nameable reason as unrenderable, not as working", () => {
    expect(renderFor(job({ status: "escalated", reason: { named: "sat_down" } }))).toBe("unrenderable");
    expect(renderFor(job({ status: "escalated" }))).toBe("unrenderable");
  });
});
