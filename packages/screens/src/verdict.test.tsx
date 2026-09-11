// The verdict sheet's own data, tested as the answer it is.
//
// **Three-way logic, so it is tested at three values.** `neverDelivers` and
// `neverAsksAPerson` each answer `true`, `false` or `undefined`, and a case
// that only checked the two-value shortcut would have missed the one the
// wire actually calls "cannot say".

import { describe, expect, it } from "vitest";
import type { StepDetail, Submitted } from "@armada/protocol";

import {
  cameBackOf,
  leftAloneOf,
  neverAsksAPerson,
  neverDelivers,
  provesItNoteOf,
  provesItOf,
} from "./verdict";

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "land",
    label: "Land",
    ordinal: 3,
    state: "awaiting_human",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-09T09:30:00Z",
    updated_at: "2026-09-09T09:41:00Z",
    ...over,
  };
}

/** A wall clock reading, for the one row that elapses while it runs. */
const NOW = Date.parse("2026-09-09T09:45:00Z");

describe("whether a workflow ever delivers", () => {
  it("is true where every step says it does not deliver", () => {
    expect(neverDelivers([step({ delivers: false }), step({ delivers: false })])).toBe(true);
  });

  it("is false where any step delivers", () => {
    expect(neverDelivers([step({ delivers: false }), step({ delivers: true })])).toBe(false);
  });

  it("is undefined where a step cannot say and none delivers", () => {
    expect(neverDelivers([step({ delivers: false }), step({ delivers: undefined })])).toBeUndefined();
  });

  it("is undefined on a Job with no steps", () => {
    expect(neverDelivers([])).toBeUndefined();
  });
});

describe("whether a workflow ever stops for a person", () => {
  it("is false where any step gates on a person", () => {
    expect(
      neverAsksAPerson([step({ advance_gate: "auto" }), step({ advance_gate: "human_always" })]),
    ).toBe(false);
  });

  it("is true where every step's gate is known and none is human", () => {
    expect(
      neverAsksAPerson([step({ advance_gate: "auto" }), step({ advance_gate: "auto_if_judge_passes" })]),
    ).toBe(true);
  });

  it("is undefined where a step's gate cannot be said and none is human", () => {
    expect(neverAsksAPerson([step({ advance_gate: "auto" }), step({ advance_gate: undefined })])).toBeUndefined();
  });
});

describe("what proves it", () => {
  it("surfaces a mechanical check with no declared counterpart", () => {
    const rows = provesItOf(
      step({ check_runs: [{ attempt: 1, name: "artifact_exists", outcome: "passed" }] }),
      [],
      NOW,
    );
    expect(rows[0]?.identifier).toBe("artifact_exists");
    expect(rows[0]?.named).toBe("passed");
  });

  it("carries a declared Check and an undeclared one together", () => {
    const rows = provesItOf(
      step({
        checks: [{ kind: "manifest_check", name: "build" }],
        check_runs: [
          { attempt: 1, name: "build", outcome: "passed" },
          { attempt: 1, name: "artifact_exists", outcome: "passed" },
        ],
      }),
      [],
      NOW,
    );
    expect(rows.map((row) => row.identifier).sort()).toEqual(["artifact_exists", "build"]);
  });

  it("adds the 'nothing checked' row where the only tier is the deliverable's existence", () => {
    const rows = provesItOf(
      step({ check_runs: [{ attempt: 1, name: "artifact_exists", outcome: "passed" }] }),
      [],
      NOW,
    );
    const nothing = rows.find((row) => row.says === "Nothing checked what it says");
    expect(nothing?.identifier).toBe("no Judge · no Checks");
  });

  it("does not add the row where a Judge is declared", () => {
    const rows = provesItOf(
      step({
        check_runs: [{ attempt: 1, name: "artifact_exists", outcome: "passed" }],
        judge_checks: [{ criteria: 1, gaming_check: false }],
        judged: [{ attempt: 1, criterion_id: "c1", verdict: "met" }],
      }),
      [{ criterion_id: "c1", text: "The draft names a limit.", source: "brief" }],
      NOW,
    );
    expect(rows.some((row) => row.says === "Nothing checked what it says")).toBe(false);
  });

  it("does not add the row where a real Check is declared beside artifact_exists", () => {
    const rows = provesItOf(
      step({
        checks: [{ kind: "manifest_check", name: "build" }],
        check_runs: [
          { attempt: 1, name: "build", outcome: "passed" },
          { attempt: 1, name: "artifact_exists", outcome: "passed" },
        ],
      }),
      [],
      NOW,
    );
    expect(rows.some((row) => row.says === "Nothing checked what it says")).toBe(false);
  });

  it("does not add the row where the step has no checks at all", () => {
    const rows = provesItOf(step(), [], NOW);
    expect(rows.some((row) => row.says === "Nothing checked what it says")).toBe(false);
  });
});

describe("the line under the checklist", () => {
  it("names the missing workflow where nothing on the step can say", () => {
    expect(provesItNoteOf(step({ checks: undefined, judge_checks: undefined }), "reviewing")).toMatch(
      /cannot say what gates this step/,
    );
  });

  it("says the answer is the only verdict, at a gate with no Judge", () => {
    expect(
      provesItNoteOf(
        step({ check_runs: [{ attempt: 1, name: "artifact_exists", outcome: "passed" }] }),
        "reviewing",
      ),
    ).toMatch(/is the review/);
  });

  it("says nobody was asked, once the Job is over", () => {
    expect(
      provesItNoteOf(
        step({ check_runs: [{ attempt: 1, name: "artifact_exists", outcome: "passed" }] }),
        "finished",
      ),
    ).toMatch(/advanced on its own/);
  });

  it("says nothing where a Judge is declared", () => {
    expect(
      provesItNoteOf(step({ judge_checks: [{ criteria: 1, gaming_check: false }] }), "reviewing"),
    ).toBeUndefined();
  });
});

describe("what came back, and what it left alone", () => {
  const claim: Submitted = {
    step_id: "land",
    evidence_type: "diff",
    claimed: "The branch is pushed and the pull request is open.",
    shown_by: "pull request #4711",
    not_claimed: "Nothing about the after-merge Checks.",
  };

  it("reads the Drone's own claim", () => {
    expect(cameBackOf(claim)).toBe(claim.claimed);
  });

  it("says a step has not submitted yet, where there is no claim", () => {
    expect(cameBackOf(undefined)).toMatch(/has not submitted/);
  });

  it("reads not_claimed where the submission drew a boundary", () => {
    expect(leftAloneOf(claim)).toBe(claim.not_claimed);
  });

  it("names the absence where the submission drew none", () => {
    expect(leftAloneOf({ ...claim, not_claimed: undefined })).toMatch(/drew no boundary/);
  });
});
