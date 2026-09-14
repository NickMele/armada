// What the verdict rests on, read off the step at the gate. Arithmetic over the wire, so it runs
// in node; the section's drawing is `ConfidenceSheet`'s stories.

import { describe, expect, it } from "vitest";
import type { Criterion, StepDetail, Submitted, Work } from "@armada/protocol";

import { NO_FRAMES } from "./frames";
import { capturedOf, checksRowOf, evidenceRowOf, judgeRowOf, planRowOf } from "./grounds";

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "implement",
    label: "Implement",
    ordinal: 2,
    state: "stopped",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-14T09:30:00Z",
    updated_at: "2026-09-14T09:41:00Z",
    ...over,
  };
}

const CRITERIA: Criterion[] = [
  { criterion_id: "c1", text: "A saved limit applies at the next turn.", source: "brief" },
  { criterion_id: "c2", text: "Nothing already running stops.", source: "brief" },
];

const CLAIM: Submitted = {
  step_id: "implement",
  evidence_type: "diff",
  claimed: "Limits save from Bridge.",
  shown_by: "the diff",
};

function work(over: Partial<Work>): Work {
  return { files: [], plan_declared: true, ...over } as Work;
}

describe("checksRowOf", () => {
  it("counts the Checks and names them", () => {
    const row = checksRowOf(
      step({
        checks: [{ kind: "manifest_check", name: "build" }, { kind: "manifest_check", name: "tests" }],
        check_runs: [
          { attempt: 1, name: "build", outcome: "passed" },
          { attempt: 1, name: "tests", outcome: "passed" },
        ],
      }),
    );
    expect(row).toEqual({ result: "2 of 2 passed", tone: "met", detail: "build, tests" });
  });

  it("draws no row for a step that gates on nothing", () => {
    expect(checksRowOf(step())).toBeUndefined();
  });
});

describe("judgeRowOf", () => {
  it("counts every criterion met", () => {
    const row = judgeRowOf(
      step({
        judged: [
          { attempt: 1, criterion_id: "c1", verdict: "met" },
          { attempt: 1, criterion_id: "c2", verdict: "met" },
        ],
      }),
      CRITERIA,
    );
    expect(row).toEqual({ result: "2 of 2 criteria met", tone: "met" });
  });

  it("leads with what was refused, naming the criterion", () => {
    const row = judgeRowOf(
      step({
        judged: [
          { attempt: 1, criterion_id: "c1", verdict: "met" },
          { attempt: 1, criterion_id: "c2", verdict: "not_met" },
        ],
      }),
      CRITERIA,
    );
    expect(row?.result).toBe("1 of 2 criteria not met");
    expect(row?.tone).toBe("not_met");
    expect(row?.detail).toBe("Nothing already running stops.");
  });
});

describe("evidenceRowOf", () => {
  it("says the claim stayed in scope where the scope Check wrote no failure", () => {
    expect(evidenceRowOf(step(), CLAIM)?.result).toBe("Within scope");
  });

  it("says where it went outside, in the Check's own words", () => {
    const row = evidenceRowOf(
      step({
        check_runs: [{ attempt: 1, name: "evidence_scope", outcome: "failed", produced: "`secrets.env` changed" }],
      }),
      CLAIM,
    );
    expect(row).toEqual({ result: "Outside its scope", tone: "not_met", detail: "`secrets.env` changed" });
  });

  it("draws no row before the Drone has claimed anything", () => {
    expect(evidenceRowOf(step(), undefined)).toBeUndefined();
  });
});

describe("planRowOf", () => {
  it("names the files outside the plan, quietly, because drift does not fail a step", () => {
    const row = planRowOf(
      work({
        files: [
          { path: "setup.rs", change: "modified", outside_plan: true },
          { path: "limits.rs", change: "modified" },
        ],
      }),
    );
    expect(row).toEqual({ result: "1 file outside the plan", tone: "quiet", detail: "`setup.rs`" });
  });

  it("says inside the plan where nothing strayed, and nothing where no plan was declared", () => {
    expect(planRowOf(work({ files: [{ path: "limits.rs", change: "modified" }] }))?.result).toBe(
      "Inside the plan",
    );
    expect(planRowOf(work({ plan_declared: false }))).toBeUndefined();
    expect(planRowOf(undefined)).toBeUndefined();
  });
});

describe("capturedOf", () => {
  it("draws nothing where there are no frames and no claim", () => {
    expect(capturedOf(step(), undefined, NO_FRAMES)).toBeUndefined();
  });

  it("keeps the current attempt's frames, and the claim", () => {
    const captured = capturedOf(
      step({
        frames: [
          { attempt: 1, name: "old.png", kept: "k1", bytes: 10 },
          { attempt: 2, name: "new.png", kept: "k2", bytes: 10 },
        ] as StepDetail["frames"],
      }),
      CLAIM,
      NO_FRAMES,
    );
    expect(captured?.frames.map((frame) => frame.name)).toEqual(["new.png"]);
    expect(captured?.claim).toEqual({ claimed: "Limits save from Bridge.", shownBy: "the diff" });
  });
});
