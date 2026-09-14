// The Board row's own run time — measured from `started_at`, never
// `created_at`, so a Job waiting for approval or a slot shows no run time.
// `#1008`.

import { describe, expect, it } from "vitest";

import type { JobSummary } from "@armada/protocol";
import { elapsedOf } from "./Row";

const NOW = Date.parse("2026-09-13T09:05:00Z");

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-13T09:00:00Z",
    ...over,
  };
}

describe("elapsedOf", () => {
  it("draws nothing for a Job waiting at the approval gate", () => {
    expect(elapsedOf(job({ status: "awaiting_approval", started_at: undefined }), NOW)).toBeUndefined();
  });

  it("draws nothing for a Job approved and waiting in the queue for a slot", () => {
    expect(elapsedOf(job({ status: "queued", started_at: undefined }), NOW)).toBeUndefined();
  });

  it("spans from started_at once the Job's first Drone has started", () => {
    expect(elapsedOf(job({ status: "running", started_at: "2026-09-13T09:00:00Z" }), NOW)).toBe("5m 00s");
  });

  it("keeps counting from the first started_at through a return to the queue", () => {
    // The clock never resets once set: a Job back at `queued` for a restart
    // still spans from the first arrival at `running`, not from zero and not
    // paused for the wait.
    expect(elapsedOf(job({ status: "queued", started_at: "2026-09-13T09:00:00Z" }), NOW)).toBe("5m 00s");
  });

  it("draws nothing once the Job is over", () => {
    expect(
      elapsedOf(job({ status: "completed_success", started_at: "2026-09-13T09:00:00Z" }), NOW),
    ).toBeUndefined();
  });
});
