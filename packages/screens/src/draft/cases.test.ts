// A case run is never an Evidence row, and a case with no spec is not covered.

import type { ShowAgain, ShownSet } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { caseRunsOf, caseViewsOf, scopeRevisionsOf } from "./cases";
import { sampleDetail, samplePlan, sampleTask } from "./sample";

function shown(over: Partial<ShownSet> = {}): ShownSet {
  return {
    press: 1,
    pressed_at: "2026-09-22T10:45:00Z",
    step_id: "implement",
    attempt: 1,
    spec: "specs/job-detail.spec.ts",
    frames: [
      {
        attempt: 1,
        name: "plan.png",
        path: ".armada/frames/1/plan.png",
        bytes: 94_000,
        kept: "1/plan.png",
      },
    ],
    ...over,
  };
}

function showAgain(over: Partial<ShowAgain> = {}): ShowAgain {
  return {
    harness: true,
    worktree_on_disk: true,
    drone_working: false,
    shown: [],
    specs: [
      {
        step_id: "implement",
        attempt: 1,
        spec: "specs/job-detail.spec.ts",
        on_disk: true,
      },
    ],
    ...over,
  };
}

describe("the cases a Job knows about", () => {
  it("is one per spec a Drone named, and none where no Drone named any", () => {
    expect(caseViewsOf(sampleDetail({ show_again: showAgain() }))).toHaveLength(1);
    expect(caseViewsOf(sampleDetail())).toEqual([]);
  });

  it("claims no coverage, because nothing reads a COVERS file yet", () => {
    expect(caseViewsOf(sampleDetail({ show_again: showAgain() }))[0]?.covers).toEqual([]);
  });

  it("reads a spec that left the worktree as not having one", () => {
    const gone = showAgain({
      specs: [{ step_id: "implement", attempt: 1, spec: "specs/gone.ts", on_disk: false }],
    });

    expect(caseViewsOf(sampleDetail({ show_again: gone }))[0]?.has_spec).toBe(false);
  });

  it("owes no tasks and no groups, because nothing resolves them", () => {
    const view = caseViewsOf(sampleDetail({ show_again: showAgain() }))[0];

    expect(view?.tasks).toEqual([]);
    expect(view?.groups).toEqual([]);
  });

  it("carries the latest press as its last run, and none where nobody pressed", () => {
    const pressed = showAgain({ shown: [shown({ press: 1 }), shown({ press: 2 })] });
    const view = caseViewsOf(sampleDetail({ show_again: pressed }))[0];

    expect(view?.last_run?.id).toBe("press-2");
    expect(caseViewsOf(sampleDetail({ show_again: showAgain() }))[0]?.last_run).toBeUndefined();
  });

  it("matches a press to its case by the spec it ran, not by the step", () => {
    const other = showAgain({
      shown: [shown({ press: 1, spec: "specs/elsewhere.spec.ts" })],
    });

    expect(caseViewsOf(sampleDetail({ show_again: other }))[0]?.last_run).toBeUndefined();
  });
});

describe("what a case run says, and what it must never say", () => {
  const runs = caseRunsOf(sampleDetail({ show_again: showAgain({ shown: [shown()] }) }));

  it("carries no verdict and no evidence id", () => {
    const run = runs[0];

    expect(run && "verdict" in run).toBe(false);
    expect(run && "evidence_id" in run).toBe(false);
    expect(run && "outcome" in run).toBe(true);
  });

  it("is a person pressing show again, against the branch", () => {
    expect(runs[0]?.actor).toBe("person");
    expect(runs[0]?.purpose).toBe("show_again");
    expect(runs[0]?.tree).toBe("branch");
  });

  it("ran, because a press that captured nothing keeps no set", () => {
    expect(runs[0]?.outcome).toBe("ran");
    expect(runs[0]?.not_run_reason).toBeUndefined();
  });

  it("counts frames rather than carrying them", () => {
    expect(runs[0]?.frames).toBe(1);
  });

  it("sits at the step and run that named the spec", () => {
    expect(runs[0]?.coord).toEqual({ step: "implement", step_attempt: 1 });
  });

  it("draws no baseline: there is one tree and it is the branch", () => {
    expect(runs.every((run) => run.tree === "branch")).toBe(true);
  });
});

describe("a scope revision", () => {
  it("is none where a step recorded the plan", () => {
    const detail = sampleDetail({ work_plan: samplePlan([sampleTask()]) });

    expect(scopeRevisionsOf(detail)).toEqual([]);
  });

  it("is none where no plan was recorded at all", () => {
    expect(scopeRevisionsOf(sampleDetail())).toEqual([]);
  });

  it("is one where a person recorded the plan whole", () => {
    const detail = sampleDetail({
      work_plan: samplePlan([sampleTask()], { recorded_by: { by: "person" } }),
    });
    const [revision] = scopeRevisionsOf(detail);

    expect(revision?.by).toBe("person");
    expect(revision?.at).toBe("2026-09-22T09:04:00Z");
  });

  it("names no paths, because the wire keeps no before and after of them", () => {
    const detail = sampleDetail({
      work_plan: samplePlan([sampleTask()], { recorded_by: { by: "person" } }),
    });
    const [revision] = scopeRevisionsOf(detail);

    expect(revision?.paths_added).toEqual([]);
    expect(revision?.paths_removed).toEqual([]);
    expect(revision?.cases_dropped).toEqual([]);
  });
});
