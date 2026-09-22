// What the classifying screen reads off a draft, and what a person's change
// puts back on it.

import { describe, expect, it } from "vitest";

import type { CriterionView } from "./draft/criterion";
import type { LandingRule } from "./draft/landing";
import type { GateView, ProposalView } from "./draft/proposal";
import { sampleDetail, sampleStep } from "./draft/sample";
import {
  completeChoices,
  criteriaRowsOf,
  criteriaWith,
  frozenAtOf,
  gateRowsOf,
  gatesWith,
  landingValueOf,
  landingWith,
  proposalEditsOf,
} from "./tab-proposal-read";

const GATES: GateView[] = [
  { step_id: "plan", checks: false, judge: true, you: false },
  { step_id: "handoff", checks: false, judge: false, you: false, repository_decides: "review_gate" },
];

const PROPOSAL: ProposalView = {
  status: "awaiting_approval",
  title: "Show what is running in the Drones stat",
  gates: GATES,
  fleet_always_looks: true,
  tiers: { difficult: "opus", medium: "sonnet", easy: null },
  drone_cap: 2,
  machine_cap: 4,
  from_ref: "main",
  pr_mode: "ready",
};

const LANDING: LandingRule = {
  target: "main",
  from_ref: "main",
  prs: "job",
  branching: "job",
  pr_mode: "ready",
  complete_when: "pr_merged",
  land_together: [],
};

describe("what the screen opens on", () => {
  it("is nothing at all where the Job is at no proposal", () => {
    expect(proposalEditsOf(undefined)).toBeUndefined();
    expect(proposalEditsOf({})).toBeUndefined();
  });

  // A Manifest that has not been read names no base, and `null` is how
  // `amending.ts` already spells that — never the default branch by another name.
  it("lands in nothing where the moment carries no landing rule", () => {
    const edits = proposalEditsOf({ proposal: PROPOSAL });

    expect(edits?.landing.target).toBeNull();
    expect(edits?.landing.from_ref).toBe("main");
    expect(edits?.criteria).toEqual([]);
  });
});

describe("one row per step", () => {
  it("takes the step's own label off the Job, and its id where Fleet has no record", () => {
    const whole = sampleDetail({
      steps: [sampleStep({ step_id: "plan", label: "Plan the change" })],
    });
    const rows = gateRowsOf(GATES, whole);

    expect(rows[0]?.label).toBe("Plan the change");
    expect(rows[1]?.label).toBe("handoff");
  });

  it("carries the reading of the combination onto the row", () => {
    const rows = gateRowsOf(GATES, null);

    expect(rows[0]?.advanceGate).toBe("auto_if_judge_passes");
    // A Judge with no Checks, which is the combination Fleet cannot act on.
    expect(rows[0]?.unmeant).toBe(true);
    expect(rows[1]?.repositoryDecides).toBe("review_gate");
  });

  it("moves one box on one step and leaves the others alone", () => {
    const moved = gatesWith(GATES, "plan", { checks: true });

    expect(moved[0]).toEqual({ ...GATES[0], checks: true });
    expect(moved[1]).toBe(GATES[1]);
  });
});

describe("how it lands", () => {
  it("draws a ref the Manifest does not name as empty rather than as a branch", () => {
    expect(landingValueOf({ ...LANDING, target: null }).target).toBe("");
  });

  it("puts an emptied field back as null, which is the Manifest naming none", () => {
    const back = landingWith(LANDING, { ...landingValueOf(LANDING), target: "" });

    expect(back.target).toBeNull();
    expect(back.from_ref).toBe("main");
  });

  // `COMPLETE_WHEN_SERVED` is read rather than restated, so an answer Fleet
  // learns to observe stops being flagged without this file being touched.
  it("says which answers nothing on a Job's record can tell yet", () => {
    const choices = completeChoices();

    expect(choices.map((one) => one.value)).toContain("pr_merged");
    expect(choices.find((one) => one.value === "pr_merged")?.served).toBe(false);
    expect(choices.find((one) => one.value === "delivered")?.served).toBe(true);
  });
});

describe("what the Job is held to", () => {
  const criteria: CriterionView[] = [
    {
      criterion_id: "a1",
      text: "The stat reads one running",
      verified_by: "check",
      origin: { origin: "issue", ref: "armada/1162" },
      origin_moved_at: "2026-09-22T10:02:00Z",
    },
    {
      text: "Pressing it lists the Drone",
      verified_by: "judge",
      origin: { origin: "prompt" },
    },
  ];

  it("says where each line's words came from, apart from how it is answered", () => {
    const rows = criteriaRowsOf(criteria);

    expect(rows[0]?.origin).toBe("from armada/1162");
    expect(rows[0]?.verifiedBy).toBe("check");
    expect(rows[1]?.origin).toBe("from the prompt");
  });

  // The Job keeps the words it froze and says the issue has moved since — the
  // instant is the issue's own edit, never the freeze.
  it("carries the issue having moved on the line it moved under", () => {
    const rows = criteriaRowsOf(criteria);

    expect(rows[0]?.movedSince).not.toBeUndefined();
    expect(rows[1]?.movedSince).toBeUndefined();
  });

  it("rewords one line and leaves the rest as they were", () => {
    const moved = criteriaWith(criteria, 1, "Pressing it lists the Drone and its step");

    expect(moved[0]).toBe(criteria[0]);
    expect(moved[1]?.text).toBe("Pressing it lists the Drone and its step");
  });
});

describe("when it froze", () => {
  it("is nothing at all while nobody has approved it", () => {
    expect(frozenAtOf(PROPOSAL)).toBeUndefined();
  });

  // A frozen proposal whose instant cannot be read is still frozen: drawing it
  // as editable would offer controls over values the Job is already running on.
  it("falls back to the instant itself where it cannot be written out", () => {
    expect(frozenAtOf({ ...PROPOSAL, approved_at: "not an instant" })).toBe("not an instant");
    expect(frozenAtOf({ ...PROPOSAL, approved_at: "2026-09-22T09:14:00Z" })).toContain("2026");
  });
});
