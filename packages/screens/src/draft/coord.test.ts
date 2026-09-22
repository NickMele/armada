// What a coordinate says while the wire has no groups.

import { describe, expect, it } from "vitest";

import { coordOfStep, coordOfTask, derivedGroupId, stepAttemptOf } from "./coord";
import { sampleDetail, sampleStep, sampleTask } from "./sample";

describe("the group a task derives into", () => {
  it("is one group per task, named from the task's own stable id", () => {
    const detail = sampleDetail();
    const first = coordOfTask(detail, sampleTask({ id: "T1" }));
    const second = coordOfTask(detail, sampleTask({ id: "T2" }));

    expect(first.group).toBe(derivedGroupId("T1"));
    expect(second.group).toBe(derivedGroupId("T2"));
    expect(first.group).not.toBe(second.group);
  });

  it("is never the task id itself, so a join cannot confuse the two", () => {
    expect(derivedGroupId("T1")).not.toBe("T1");
  });

  it("names the step the Job is on", () => {
    const detail = sampleDetail({
      job: { ...sampleDetail().job, current_step_id: "tests" },
      steps: [sampleStep({ step_id: "implement" }), sampleStep({ step_id: "tests" })],
    });

    expect(coordOfTask(detail, sampleTask()).step).toBe("tests");
  });

  it("falls back to the last step where the Job names none", () => {
    const detail = sampleDetail({
      job: sampleDetail().job,
      steps: [sampleStep({ step_id: "implement" }), sampleStep({ step_id: "handoff" })],
    });
    delete detail.job.current_step_id;

    expect(coordOfTask(detail, sampleTask()).step).toBe("handoff");
  });
});

describe("which run of a step a coordinate names", () => {
  it("is one on a step nothing has entered, never zero", () => {
    expect(stepAttemptOf(sampleStep({ attempts: [] }))).toBe(1);
    expect(stepAttemptOf(undefined)).toBe(1);
  });

  it("is the latest run where the step has been run more than once", () => {
    const step = sampleStep({
      attempts: [
        { attempt: 1, outcome: "refused", started_at: "2026-09-22T09:05:00Z" },
        { attempt: 2, outcome: "running", started_at: "2026-09-22T09:20:00Z" },
      ],
    });

    expect(stepAttemptOf(step)).toBe(2);
  });
});

describe("a coordinate about a step and no task", () => {
  it("names no group and no task at all", () => {
    const coord = coordOfStep(sampleStep({ step_id: "plan" }));

    expect(coord).toEqual({ step: "plan", step_attempt: 1 });
    expect(coord.group).toBeUndefined();
    expect(coord.task).toBeUndefined();
  });
});
