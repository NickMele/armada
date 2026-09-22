// What a revision came to, tested where a hundred cases cost what one costs.
// Its rendering is claimed through `App`, in `arc.test.tsx`. `#1552`.

import { describe, expect, test } from "vitest";

import type { StepDetail } from "@armada/protocol";

import { arcCases, arcGroups, withTask } from "../fixtures/build/arc-plan";
import type { CaseView, ScopeRevisionView } from "./cases";
import type { CriterionView } from "./criterion";
import { planRevisionsOf, revisedTaskOf, revisionSaid } from "./revision";

const AT = "2026-09-22T09:34:00Z";

/** The narrowing the arc holds: `c-panel` let go by `T5` alone. */
function narrowed(): ScopeRevisionView {
  return {
    at: AT,
    paths_added: [],
    paths_removed: ["packages/screens/src/running-rows.tsx"],
    cases_dropped: ["c-panel"],
    cases_added: [],
    by: "person",
    outcome: "refused",
  };
}

/** The cases as they read after the narrowing — `c-panel` dropped. */
function droppedCases(): CaseView[] {
  return arcCases().map((one) =>
    one.id === "c-panel"
      ? { ...one, state: "dropped" as const, dropped_by: { dropped: "scope_revision" as const, at: AT } }
      : one,
  );
}

const CRITERIA: CriterionView[] = [
  { criterion_id: "a2", text: "The plan names what it will touch", verified_by: "judge", origin: { origin: "prompt" } },
];

/** A plan step run twice, the second run answered however the caller says. */
function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "plan",
    label: "Plan the change",
    ordinal: 1,
    state: "awaiting_human",
    checks: [],
    check_runs: [],
    judge_checks: [],
    judged: [],
    flagged: [],
    overridden: false,
    attempts: [
      { attempt: 1, outcome: "advanced", started_at: AT, ended_at: AT },
      { attempt: 2, outcome: "awaiting_human", started_at: AT },
    ],
    verdicts: [],
    entered_at: AT,
    updated_at: AT,
    ...over,
  };
}

describe("which task a revision touched", () => {
  test("the task that no longer claims the dropped case is the one that was narrowed", () => {
    const groups = withTask(arcGroups(), "T5", { cases: [] });
    expect(revisedTaskOf(narrowed(), droppedCases(), groups)).toBe("T5");
  });

  test("a case every task still claims names nobody, rather than guessing", () => {
    expect(revisedTaskOf(narrowed(), droppedCases(), arcGroups())).toBeUndefined();
  });

  test("two tasks letting one case go names neither — a revision nobody can check is not one", () => {
    const groups = withTask(withTask(arcGroups(), "T5", { cases: [] }), "T6", { cases: [] });
    expect(revisedTaskOf(narrowed(), droppedCases(), groups)).toBeUndefined();
  });

  test("a revision that dropped no case names nobody", () => {
    const none = { ...narrowed(), cases_dropped: [] };
    expect(revisedTaskOf(none, droppedCases(), arcGroups())).toBeUndefined();
  });
});

describe("what was asked, in words", () => {
  test("paths taken out read as a narrowing of the task that lost them", () => {
    expect(revisionSaid(narrowed(), "T5")).toBe(
      "Take packages/screens/src/running-rows.tsx out of T5.",
    );
  });

  test("paths put in read the other way", () => {
    const added = { ...narrowed(), paths_removed: [], paths_added: ["a.ts"] };
    expect(revisionSaid(added, "T5")).toBe("Put a.ts into T5.");
  });

  test("both halves are one sentence", () => {
    const both = { ...narrowed(), paths_added: ["a.ts"] };
    expect(revisionSaid(both, "T5")).toBe(
      "Take packages/screens/src/running-rows.tsx out of T5 and put a.ts in.",
    );
  });

  test("a revision naming no task speaks of the plan", () => {
    expect(revisionSaid({ ...narrowed(), paths_removed: [] }, undefined)).toBe(
      "Rewrite the plan.",
    );
  });
});

describe("the answer to an ask", () => {
  const groups = withTask(arcGroups(), "T5", { cases: [] });
  const cases = droppedCases();

  test("a not_met verdict on the run the ask made is a refusal, with the criterion in its own words", () => {
    const refused = step({
      judged: [
        {
          attempt: 2,
          criterion_id: "a2",
          verdict: "not_met",
          expected: "the plan names every file",
          produced: "no task claims running-rows.tsx",
        },
      ],
    });
    const [one] = planRevisionsOf(refused, [narrowed()], cases, groups, CRITERIA);
    expect(one?.answer).toBe("refused");
    expect(one?.task).toBe("T5");
    expect(one?.refusal).toEqual({
      criterion: "The plan names what it will touch",
      expected: "the plan names every file",
      produced: "no task claims running-rows.tsx",
    });
    expect(one?.dropped).toEqual([
      { id: "c-panel", spec: "packages/screens/src/Running.test.tsx" },
    ]);
  });

  test("a run that happened and refused nothing is an ask the Drone took", () => {
    const [one] = planRevisionsOf(step(), [narrowed()], cases, groups, CRITERIA);
    expect(one?.answer).toBe("taken");
  });

  test("a step that has not been run again is an ask still out", () => {
    const once = step({ attempts: [{ attempt: 1, outcome: "advanced", started_at: AT, ended_at: AT }] });
    const [one] = planRevisionsOf(once, [narrowed()], cases, groups, CRITERIA);
    expect(one?.answer).toBe("asked");
  });

  test("a Job whose plan step has not arrived reads every ask as still out", () => {
    const [one] = planRevisionsOf(undefined, [narrowed()], cases, groups, CRITERIA);
    expect(one?.answer).toBe("asked");
  });

  test("the first run recorded the plan, so the second revision is answered on the third run", () => {
    const third = step({
      attempts: [
        { attempt: 1, outcome: "advanced", started_at: AT, ended_at: AT },
        { attempt: 2, outcome: "advanced", started_at: AT, ended_at: AT },
        { attempt: 3, outcome: "awaiting_human", started_at: AT },
      ],
      judged: [{ attempt: 3, criterion_id: "a2", verdict: "not_met" }],
    });
    const two = planRevisionsOf(third, [narrowed(), narrowed()], cases, groups, CRITERIA);
    expect(two.map((one) => one.answer)).toEqual(["taken", "refused"]);
  });

  test("a criterion the Job does not carry leaves the refusal without one, rather than an id", () => {
    const refused = step({
      judged: [{ attempt: 2, criterion_id: "nothing", verdict: "not_met", produced: "no" }],
    });
    const [one] = planRevisionsOf(refused, [narrowed()], cases, groups, CRITERIA);
    expect(one?.refusal).toEqual({ produced: "no" });
  });

  test("no revisions is no reading, never an empty one drawn", () => {
    expect(planRevisionsOf(step(), [], cases, groups, CRITERIA)).toEqual([]);
  });
});
