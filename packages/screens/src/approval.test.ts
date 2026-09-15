// The workflow overview a Job at `awaiting_approval` draws — one box per
// step, the loop it names, and the facts beside it. #1149.

import { describe, expect, it } from "vitest";

import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";
import { approvalOverviewOf } from "./approval";
import { detail, job, spend } from "./fixtures/build/base";

/** `bug.json`'s three steps, before any of them has run. */
function bugSteps(): StepDetail[] {
  return [
    {
      step_id: "plan",
      label: "Plan the change",
      ordinal: 1,
      state: "not_started",
      checks: [{ kind: "plan_recorded" }],
      check_runs: [],
      judge_checks: [{ criteria: 2, gaming_check: false }],
      judged: [],
      flagged: [],
      advance_gate: "auto_if_judge_passes",
      delivers: false,
      overridden: false,
      attempts: [],
      verdicts: [],
      entered_at: "2026-09-15T09:00:00Z",
      updated_at: "2026-09-15T09:00:00Z",
    },
    {
      step_id: "implement",
      label: "Implement",
      ordinal: 2,
      state: "not_started",
      checks: [{ kind: "diff_nonempty" }],
      check_runs: [],
      judge_checks: [{ criteria: 3, gaming_check: true }],
      judged: [],
      flagged: [],
      advance_gate: "auto_if_judge_passes",
      delivers: false,
      overridden: false,
      attempts: [],
      verdicts: [],
      entered_at: "2026-09-15T09:00:00Z",
      updated_at: "2026-09-15T09:00:00Z",
    },
    {
      step_id: "handoff",
      label: "Review the change",
      ordinal: 3,
      state: "not_started",
      checks: [],
      check_runs: [],
      judge_checks: [],
      judged: [],
      flagged: [],
      advance_gate: "human_always",
      delivers: true,
      overridden: false,
      attempts: [],
      verdicts: [],
      entered_at: "2026-09-15T09:00:00Z",
      updated_at: "2026-09-15T09:00:00Z",
    },
  ];
}

/** `epic.json`'s three steps, `roll_up` routing back to `plan` capped at 5. */
function epicSteps(): StepDetail[] {
  const base = {
    ordinal: 1,
    state: "not_started" as const,
    checks: [] as StepDetail["checks"],
    check_runs: [],
    judge_checks: [{ criteria: 2, gaming_check: false }],
    judged: [],
    flagged: [],
    delivers: false,
    overridden: false,
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-15T09:00:00Z",
    updated_at: "2026-09-15T09:00:00Z",
  };
  return [
    { ...base, step_id: "plan", label: "Plan the wave", advance_gate: "human_always" },
    { ...base, step_id: "dispatch", label: "Dispatch the wave", ordinal: 2, advance_gate: "auto", judge_checks: [] },
    {
      ...base,
      step_id: "roll_up",
      label: "Roll up the wave",
      ordinal: 3,
      judge_checks: [],
      advance_gate: "human_always",
      pass: { number: 1, of: 5 },
      verdict_routing_target: "plan",
    },
  ];
}

function wholeAt(status: string, steps: StepDetail[]): { job: ReturnType<typeof job>; whole: JobWhole } {
  const theJob = job(status);
  return { job: theJob, whole: detail(theJob, steps, { spend: spend({ cost_cap_micros: 20_000_000, turn_cap: 40 }) }) };
}

describe("the overview a Job waiting for approval draws", () => {
  it("is absent on every other status", () => {
    const { job: theJob, whole } = wholeAt("running", bugSteps());
    expect(approvalOverviewOf(theJob, whole)).toBeUndefined();
  });

  it("is absent where the Job's steps have not arrived yet", () => {
    expect(approvalOverviewOf(job("awaiting_approval"), null)).toBeUndefined();
  });

  it("draws one box per step, its own Checks and Judge, and never the gate as a row inside it", () => {
    const { job: theJob, whole } = wholeAt("awaiting_approval", bugSteps());
    const overview = approvalOverviewOf(theJob, whole);
    expect(overview?.diagram).toEqual([
      {
        id: "plan",
        label: "Plan the change",
        labelIsAnIdentifier: undefined,
        checks: [{ command: "plan_recorded", covers: undefined }],
        declarations: [{ label: "judge · 2 criteria" }],
        gate: "auto",
        gateLabel: "the checks decide, unless the Judge objects",
        loop: undefined,
      },
      {
        id: "implement",
        label: "Implement",
        labelIsAnIdentifier: undefined,
        checks: [{ command: "diff_nonempty", covers: undefined }],
        declarations: [{ label: "judge · 3 criteria · gaming check" }],
        gate: "auto",
        gateLabel: "the checks decide, unless the Judge objects",
        loop: undefined,
      },
      {
        id: "handoff",
        label: "Review the change",
        labelIsAnIdentifier: undefined,
        checks: undefined,
        declarations: undefined,
        gate: "person",
        gateLabel: "a person answers",
        loop: undefined,
      },
    ]);
  });

  it("names the step a loop returns to, and its cap — not guessed from `structure: loop` alone", () => {
    const { job: theJob, whole } = wholeAt("awaiting_approval", epicSteps());
    const overview = approvalOverviewOf(theJob, whole);
    const rollUp = overview?.diagram.find((step) => step.id === "roll_up");
    expect(rollUp?.loop).toEqual({ to: "plan", label: "up to 5 passes" });
  });

  it("lists every step that stops for a person, in the workflow's order", () => {
    const { job: theJob, whole } = wholeAt("awaiting_approval", epicSteps());
    const overview = approvalOverviewOf(theJob, whole);
    expect(overview?.facts).toEqual(
      expect.arrayContaining([{ label: "Stops for a person", value: "Plan the wave, Roll up the wave" }]),
    );
  });

  it("carries the cost cap, exactly, beside the model — never hedged, unlike a spend", () => {
    const { job: theJob, whole } = wholeAt("awaiting_approval", bugSteps());
    const overview = approvalOverviewOf(theJob, whole);
    expect(overview?.facts).toEqual(
      expect.arrayContaining([
        { label: "Cost cap", value: "$20.00", mono: true },
        { label: "Turn cap", value: "40", mono: true },
        { label: "Model", value: theJob.model, mono: true },
      ]),
    );
  });
});
