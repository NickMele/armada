// The timeline's arithmetic, case by case.
//
// **Every case is a claim `timeline.ts` makes.** An attempt is the spine, turns
// fall into the attempt whose window holds them, and what an attempt wrote
// rides on its own working row. The turns are built with the fixtures' own
// constructors, so a change to the wire's shape breaks this rather than
// drifting past it.
import { describe, expect, it } from "vitest";

import type { CheckRun, StepDetail } from "@armada/protocol";

import { freshStep, instructed, producedTurn, said } from "./fixtures/build/base";
import { timelineOf } from "./timeline";

const NOW = Date.parse("2026-09-10T14:30:00Z");

/** A step on its second attempt, the first handed back by a Check. */
function handedBack(over: Partial<StepDetail> = {}): StepDetail {
  return {
    ...freshStep("regression_verify", "Regression check", 4),
    state: "running",
    entered_at: "2026-09-10T14:22:18Z",
    updated_at: "2026-09-10T14:26:00Z",
    attempts: [
      {
        attempt: 1,
        outcome: "retrying",
        why: "gate_failure",
        started_at: "2026-09-10T14:22:18Z",
        ended_at: "2026-09-10T14:24:40Z",
      },
      { attempt: 2, outcome: "running", started_at: "2026-09-10T14:24:40Z" },
    ],
    ...over,
  };
}

function failedRun(attempt: number): CheckRun {
  return { attempt, name: "cargo_nextest", outcome: "failed", produced: "exit 101" };
}

describe("an attempt is the spine", () => {
  it("draws one section per attempt, oldest first, the last one current", () => {
    const drawn = timelineOf(handedBack(), [], NOW);
    expect(drawn.map((one) => one.attempt)).toEqual([1, 2]);
    expect(drawn.map((one) => one.current)).toEqual([false, true]);
  });

  it("draws one attempt for a step Fleet has recorded none for", () => {
    const step = { ...freshStep("fix", "Fix", 3), attempts: [] };
    const drawn = timelineOf(step, [], NOW);
    expect(drawn).toHaveLength(1);
    expect(drawn[0]?.outcome).toBe(step.state);
  });

  it("carries the wire's own outcome and why", () => {
    const [first] = timelineOf(handedBack(), [], NOW);
    expect(first?.outcome).toBe("retrying");
    expect(first?.why).toBe("gate_failure");
  });
});

describe("turns fall into the attempt that was running", () => {
  const turns = [
    instructed("regression_verify", "2026-09-10T14:22:20Z", 4, "the suite passes", "Regression check"),
    said("regression_verify", "2026-09-10T14:23:02Z", "Reading the failure."),
    said("regression_verify", "2026-09-10T14:25:10Z", "Attempt 2: adjusting the memo key."),
  ];

  it("puts each turn under the attempt whose window holds it", () => {
    const [first, second] = timelineOf(handedBack(), turns, NOW);
    expect(first?.rows.find((row) => row.phase === "working")?.turns).toHaveLength(2);
    expect(second?.rows.find((row) => row.phase === "working")?.turns).toHaveLength(1);
  });

  it("counts them, and the time the attempt ran, on the row itself", () => {
    const [first] = timelineOf(handedBack(), turns, NOW);
    expect(first?.rows.find((row) => row.phase === "working")?.meta).toBe("2 turns · 2m 22s");
  });

  it("never draws a finished attempt as running, on a step that is", () => {
    const [first, second] = timelineOf(handedBack(), turns, NOW);
    expect(first?.rows.find((row) => row.phase === "working")?.mark).toBe("advanced");
    expect(second?.rows.find((row) => row.phase === "working")?.mark).toBe("running");
  });
});

describe("what an attempt wrote", () => {
  const wrote = producedTurn("regression_verify", "2026-09-10T14:23:30Z", [
    { path: "crates/settings/src/reducer.rs", change: "modified" },
  ]);

  it("rides on the working row of the attempt that wrote it", () => {
    const [first, second] = timelineOf(handedBack(), [wrote], NOW);
    expect(first?.rows.find((row) => row.phase === "working")?.produced).toHaveLength(1);
    expect(second?.rows.find((row) => row.phase === "working")?.produced).toBeUndefined();
  });

  it("is counted in what the row says folded", () => {
    const [first] = timelineOf(handedBack(), [wrote], NOW);
    expect(first?.rows.find((row) => row.phase === "working")?.meta).toContain("1 file");
  });
});

describe("the gate's rows", () => {
  it("marks the Checks of the attempt that failed them", () => {
    const step = handedBack({ check_runs: [failedRun(1)], checks: [] });
    const [first, second] = timelineOf(step, [], NOW);
    expect(first?.rows.find((row) => row.phase === "checks")?.mark).toBe("failed");
    expect(second?.rows.find((row) => row.phase === "checks")?.mark).toBe("not_started");
  });

  it("says a Check was not reached rather than that it was skipped", () => {
    const step = handedBack({ checks: [{ kind: "command", name: "cargo_nextest" }] });
    const [, second] = timelineOf(step, [], NOW);
    expect(second?.rows.find((row) => row.phase === "checks")?.meta).toBe("not reached");
  });

  it("says how many criteria a Judge was never asked", () => {
    const step = handedBack({ judge_checks: [{ criteria: 2, gaming_check: false }] });
    const [, second] = timelineOf(step, [], NOW);
    expect(second?.rows.find((row) => row.phase === "judge")?.meta).toBe("2 criteria, not asked");
  });
});
