// The fifth arrangement's own data, tested as the answer it is.
//
// **The fixture is the real Job the drawing used**, `01M27918MN0011N9KZEV9ZWHY3`
// — dispatched, restarted once, its `tests` step overruled, approved at
// `handoff`, and merged as #630. `slotFor` builds the two shapes that Job took
// on the way: `overriddenStep(false)` is any ordinary approve, and `true` is
// what the run tree shows a reader who opens the `tests` step.

import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";
import type { VerdictSheetProps } from "@armada/components";
import type { Diff, Evidence, JobDetail as JobWhole, JobSummary, Noted, Remarks, StepDetail } from "@armada/protocol";

import { humanGateStepOf, overruleReasonOf, verdictSlotAfterAnswer } from "./verdict-answered";

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
const NOW = Date.parse("2026-09-11T05:45:00Z");

function overruleNote(over: Partial<Noted> = {}): Noted {
  return {
    seq: 1,
    at: "2026-09-11T05:16:03.465Z",
    by: "fleet",
    level: "warn",
    msg: "a person overruled the gate and the step advanced",
    step: "tests",
    fields: [
      { name: "overruled", value: "gate_failure" },
      { name: "said", value: "Reasonable, and typed by a person." },
    ],
    ...over,
  };
}

describe("the step a person answers at", () => {
  it("finds the step gated human_always", () => {
    const gate = step({ step_id: "handoff", advance_gate: "human_always" });
    expect(humanGateStepOf([step({ advance_gate: "auto_if_judge_passes" }), gate])).toBe(gate);
  });

  it("is undefined where no step in the frozen workflow ever asks a person", () => {
    expect(humanGateStepOf([step({ advance_gate: "auto" }), step({ advance_gate: "auto" })])).toBeUndefined();
  });
});

describe("the reason a person gave for an overrule", () => {
  it("reads the said field off the note for that step", () => {
    expect(overruleReasonOf([overruleNote()], "tests")).toBe("Reasonable, and typed by a person.");
  });

  it("is undefined for a step the note is not about", () => {
    expect(overruleReasonOf([overruleNote()], "implement")).toBeUndefined();
  });

  it("is undefined where the log has no note about an overrule at all", () => {
    expect(overruleReasonOf([], "tests")).toBeUndefined();
  });

  it("is undefined where a person overruled and typed nothing", () => {
    const said = overruleNote({ fields: [{ name: "overruled", value: "gate_failure" }] });
    expect(overruleReasonOf([said], "tests")).toBeUndefined();
  });

  it("takes the most recent note where a step was overruled more than once", () => {
    const first = overruleNote({ seq: 1, fields: [{ name: "said", value: "first" }] });
    const second = overruleNote({ seq: 2, fields: [{ name: "said", value: "second" }] });
    expect(overruleReasonOf([first, second], "tests")).toBe("second");
  });
});

describe("the verdict sheet once a Job is done and a person answered it", () => {
  function job(over: Partial<JobSummary> = {}): JobSummary {
    return {
      id: "01M27918MN0011N9KZEV9ZWHY3",
      handle: "2-a-job",
      title: "Refuse a merge press whose chosen comments won't fit the brief",
      status: "completed_success",
      workflow_id: "feature",
      owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
      origin: "manual",
      urgency: "normal",
      atomic: true,
      model: "sonnet",
      created_at: "2026-09-11T03:43:58.613Z",
      branch: "armada/2-refuse-a-merge-press-whose-chosen-comments",
      current_step_id: "handoff",
      ...over,
    };
  }

  const NO_DIFF: Diff = { state: "none" };
  const NO_EVIDENCE: Evidence = { state: "none" };
  const NO_REMARKS: Remarks = { state: "none" };

  function slotFor(overriddenStep: boolean): ReactElement<VerdictSheetProps> {
    const testsStep = step({
      step_id: "tests",
      advance_gate: "auto_if_judge_passes",
      overridden: overriddenStep,
      judge_checks: [{ criteria: 1, gaming_check: true }],
      judged: overriddenStep
        ? [{ attempt: 1, criterion_id: "c1", verdict: "not_met", expected: "Stay in scope." }]
        : [],
    });
    const gateStep = step({
      step_id: "handoff",
      advance_gate: "human_always",
      delivers: true,
      state: "advanced",
      updated_at: "2026-09-11T05:29:16.834Z",
    });
    const whole: JobWhole = {
      job: job(),
      created_at: "2026-09-11T03:43:58.613Z",
      steps: [testsStep, gateStep],
      acceptance_criteria: [],
      dependencies: [],
      spend: {
        cost_micros: 3_891_121,
        cost_cap_micros: 10_000_000,
        turns: 126,
        turn_cap: 300,
        ran_ms: 5_569_564,
        drones: 5,
      },
      delivery: { pull_request: "https://forge.invalid/armada/armada/pull/630", landed: "merged" },
    };
    return verdictSlotAfterAnswer({
      job: job(),
      whole,
      open: testsStep,
      render: "finished",
      recorded: { diff: NO_DIFF, evidence: NO_EVIDENCE, remarks: NO_REMARKS },
      opensRecords: { jobId: job().id, open: async () => ({ ok: true }), onSaid: () => {} },
      now: NOW,
      claimed: undefined,
      notes: overriddenStep
        ? [
            {
              seq: 1,
              at: "2026-09-11T05:16:03.465Z",
              by: "fleet",
              level: "warn",
              msg: "a person overruled the gate and the step advanced",
              step: "tests",
              fields: [{ name: "said", value: "The note is correct and needed." }],
            },
          ]
        : [],
    }) as ReactElement<VerdictSheetProps>;
  }

  it("carries the header off the human gate step, and the settled pull request", () => {
    const slot = slotFor(false);
    expect(slot.props.header).toEqual({ done: "Done", when: expect.stringContaining("approved") });
    expect(slot.props.pullRequest).toBeDefined();
    expect(slot.props.actions).toBeUndefined();
    expect(slot.props.recordNote).toBeDefined();
  });

  it("adds how many Drones ran to the figures", () => {
    const slot = slotFor(false);
    expect(slot.props.figures.some((figure) => figure.label === "Drones" && figure.value === "5")).toBe(true);
  });

  it("carries the overrule reason through to the open step's overruled row, where the open step is the one overruled", () => {
    const slot = slotFor(true);
    const provesIt = slot.props.provesIt as ReactElement<{ rows: { detail?: string }[] }>;
    const overruled = provesIt.props.rows.find((row) => "detail" in row && row.detail?.includes("You:"));
    expect(overruled?.detail).toMatch(/You: “The note is correct and needed\.”/);
  });
});
