// What the restart button says it is about to do, and the one job where it
// used to say the opposite.
//
// **Two jobs reach the same act and the button has to tell them apart.** On an
// ordinary stopped job the drone really has gone; on a job whose drone outlived
// the fleet that held its pipes it is running and unreachable, and the sentence
// claiming it was gone contradicted the step in flight on the same screen.
// `drone_unheard` is what fleet sends so bridge can say which one it is looking
// at, rather than guessing from `stopped_by` — which reads `gate_failure` here.

import { describe, expect, it } from "vitest";

import type { JobDetail as JobWhole, JobSummary, Stuck } from "@armada/protocol";
import { recourseOf } from "./recovery";

/** An escalated job, which is the status both acts ask for. */
function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    title: "Coalesce concurrent token refreshes",
    status: "escalated",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    branch: "armada/01M130Y1380016YK5S0JXBXDQ5",
    ...over,
  };
}

/** The classification, as fleet made it: a restart on offer and nothing else. */
function stuck(over: Partial<Stuck> = {}): Stuck {
  return {
    stopped_by: "gate_failure",
    step_id: "implement",
    recourse: ["restart_step", "redispatch_job"],
    worktree_on_disk: true,
    drone_unheard: false,
    ...over,
  };
}

function whole(over: Partial<Stuck> = {}): JobWhole {
  return {
    job: job(),
    created_at: "2026-08-31T09:00:00Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
    stuck: stuck(over),
  };
}

describe("what the restart button says", () => {
  it("says the drone is gone where it is", () => {
    const says = recourseOf(job(), whole()).says.restart_step;
    expect(says).toContain("The drone is gone.");
  });

  it("does not say the drone is gone where it is standing there", () => {
    // The defect, stated as the assertion. The job is running on screen and the
    // tooltip said the drone had gone.
    const says = recourseOf(job(), whole({ drone_unheard: true })).says.restart_step;
    expect(says).not.toContain("The drone is gone");
    expect(says).toContain("still running");
  });

  it("says that restarting is what ends the unreadable drone", () => {
    // The part a person would not guess from a job that looks like it is
    // working: the act does not wait for the drone, it takes it away.
    const says = recourseOf(job(), whole({ drone_unheard: true })).says.restart_step;
    expect(says).toContain("restarting ends it");
  });

  it("describes the same act in both, in the same words", () => {
    // Both cases are real and restart does the right thing in each, so what
    // happens next must not read as two different acts.
    const gone = recourseOf(job(), whole()).says.restart_step ?? "";
    const unheard = recourseOf(job(), whole({ drone_unheard: true })).says.restart_step ?? "";
    const queued = "The job goes back in the queue and a fresh one takes over at the step above";
    expect(gone).toContain(queued);
    expect(unheard).toContain(queued);
  });

  it("says the worktree took the restart away, not the missing step", () => {
    // A job-level escalation names no step, and on this one that is not why the
    // restart is absent — fleet read the disk.
    const stands = recourseOf(
      job(),
      whole({
        step_id: undefined,
        recourse: ["redispatch_job"],
        worktree_on_disk: false,
        drone_unheard: true,
      }),
    ).stands;
    expect(stands).toContain("the worktree it was working in is gone");
    expect(stands).not.toContain("no step to land on");
  });
});
