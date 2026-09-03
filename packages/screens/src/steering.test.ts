// What a job that has not stopped offers, and what it does not read to decide.
//
// **The cases that matter are the two absences.** A job with no drone on it
// must offer nothing rather than a redirect with nowhere to land, and a job
// whose detail has not arrived must still offer the act — the whole difference
// between this reading and `recovery.ts` is that one of them needs a second
// read of the wire and this one does not.

import { describe, expect, it } from "vitest";

import type { JobDetail, JobSummary } from "@armada/protocol";
import { steeringOf } from "./steering";

/** A job that is running, with a drone on it. */
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
    branch: "armada/01M130Y1380016YK5S0JXBXDQ5",
    assigned_drone: "01M1D0X0000016YK5S0JXBXDQ5",
    ...over,
  };
}

/** The detail, with or without a redirect outstanding. */
function detail(over: Partial<JobDetail> = {}): JobDetail {
  return {
    job: job(),
    created_at: "2026-08-31T09:00:00Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
    ...over,
  };
}

describe("what a working job offers", () => {
  it("offers a redirect to the drone that is there", () => {
    const steering = steeringOf(job(), detail());
    expect(steering.act).toBe("redirect");
    expect(steering.says.redirect).toBeDefined();
  });

  it("offers it before the detail has arrived", () => {
    // The act is on the summary, so it does not wait on `GET /jobs/:job_id`.
    // Every act on a job that stopped does, because `stuck` is only there.
    expect(steeringOf(job(), null).act).toBe("redirect");
  });

  it("offers nothing where no drone holds the job", () => {
    // A queued job, a job waiting on a slot, and the moment between one step's
    // drone ending and the next one's starting. There is no session for an
    // instruction to go into and nothing on the wire holds a note for a
    // running job, so the honest answer is no control.
    const nobody = steeringOf(job({ assigned_drone: undefined }), detail());
    expect(nobody.act).toBeUndefined();
    expect(nobody.says).toEqual({});
    expect(nobody.sent).toBeUndefined();
  });

  it("never says a step is being started again", () => {
    // `docs/concepts/drone.md` makes this the rule and not a preference: the
    // work already done is kept, and a redirect that read as a restart would
    // be the screen describing the wrong act.
    expect(steeringOf(job(), detail()).says.redirect).toContain("not started again");
  });
});

describe("the redirect that is out", () => {
  it("says it was sent, with the time it went into the session", () => {
    const sent = steeringOf(
      job(),
      detail({ redirecting: { sent_at: "2026-08-31T09:04:00Z" } }),
    ).sent;
    expect(sent).toContain("Sent, waiting for the drone");
  });

  it("says nothing moves when it lands", () => {
    // The half that differs from an escalated job, and the reason this reading
    // exists at all: nothing was held, so the drone's turn releases nothing.
    const sent = steeringOf(
      job(),
      detail({ redirecting: { sent_at: "2026-08-31T09:04:00Z" } }),
    ).sent;
    expect(sent).toContain("never held");
    expect(sent).not.toContain("escalated");
  });

  it("says nothing where nothing is outstanding", () => {
    expect(steeringOf(job(), detail()).sent).toBeUndefined();
    expect(steeringOf(job(), null).sent).toBeUndefined();
  });
});
