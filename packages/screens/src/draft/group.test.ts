// One task per group, in plan order, while the wire has no groups.

import { describe, expect, it } from "vitest";

import { derivedGroupId } from "./coord";
import { taskGroupsOf } from "./group";
import { sampleDetail, samplePlan, sampleStep, sampleTask } from "./sample";

function withTasks(ids: string[], step = sampleStep()) {
  return sampleDetail({
    steps: [step],
    work_plan: samplePlan(ids.map((id) => sampleTask({ id }))),
    job: { ...sampleDetail().job, current_step_id: step.step_id },
  });
}

describe("what the wire's silence about groups derives into", () => {
  it("is one group per task, in plan order, counted from one", () => {
    const groups = taskGroupsOf(withTasks(["T1", "T2", "T3"]));

    expect(groups).toHaveLength(3);
    expect(groups.map((group) => group.ordinal)).toEqual([1, 2, 3]);
    expect(groups.map((group) => group.id)).toEqual([
      derivedGroupId("T1"),
      derivedGroupId("T2"),
      derivedGroupId("T3"),
    ]);
  });

  it("holds exactly its one task and nothing concurrent", () => {
    const [group] = taskGroupsOf(withTasks(["T1"]));

    expect(group?.tasks.map((task) => task.id)).toEqual(["T1"]);
    expect(group?.concurrent).toBe(false);
  });

  it("is empty where no plan was recorded", () => {
    expect(taskGroupsOf(sampleDetail())).toEqual([]);
  });
});

describe("the scope and Checks a group takes", () => {
  it("takes its task's declared paths as its own", () => {
    const detail = withTasks([]);
    detail.work_plan = samplePlan([
      sampleTask({ id: "T1", scope: ["crates/ipc/src/work_plan.rs"] }),
    ]);

    expect(taskGroupsOf(detail)[0]?.scope).toEqual(["crates/ipc/src/work_plan.rs"]);
  });

  it("selects the step's declared Checks, by name", () => {
    const step = sampleStep({
      checks: [
        { kind: "manifest_check", name: "typecheck" },
        { kind: "manifest_check", name: "test" },
      ],
    });

    expect(taskGroupsOf(withTasks(["T1"], step))[0]?.checks_selected).toEqual([
      "typecheck",
      "test",
    ]);
  });

  it("stands the kind in for a Check that names no Manifest Check", () => {
    const step = sampleStep({ checks: [{ kind: "diff_nonempty" }] });

    expect(taskGroupsOf(withTasks(["T1"], step))[0]?.checks_selected).toEqual([
      "diff_nonempty",
    ]);
  });

  it("runs no cases at its boundary, because nothing resolves them", () => {
    expect(taskGroupsOf(withTasks(["T1"]))[0]?.cases_at_boundary).toEqual([]);
  });
});

describe("where a group is", () => {
  it("is checking while the gate is running the step's Checks", () => {
    const step = sampleStep({
      checking: { attempt: 1, checks: [] },
    });

    expect(taskGroupsOf(withTasks(["T1"], step))[0]?.state).toBe("checking");
  });

  it("reads a stopped step as failed and an advanced one as passed", () => {
    expect(taskGroupsOf(withTasks(["T1"], sampleStep({ state: "stopped" })))[0]?.state).toBe(
      "failed",
    );
    expect(taskGroupsOf(withTasks(["T1"], sampleStep({ state: "advanced" })))[0]?.state).toBe(
      "passed",
    );
  });

  it("reads a step state it does not know as pending, never as running", () => {
    expect(
      taskGroupsOf(withTasks(["T1"], sampleStep({ state: "not_started" })))[0]?.state,
    ).toBe("pending");
  });

  it("is passed where the task itself is done, whatever the step is doing", () => {
    const detail = withTasks([]);
    detail.work_plan = samplePlan([sampleTask({ id: "T1", state: "done" })]);

    expect(taskGroupsOf(detail)[0]?.state).toBe("passed");
  });
});

describe("retries, counted from the step's own runs", () => {
  it("is nought on a step nothing has entered", () => {
    expect(taskGroupsOf(withTasks(["T1"]))[0]?.retry_count).toBe(0);
  });

  it("is one fewer than the runs of the step holding it", () => {
    const step = sampleStep({
      attempts: [
        { attempt: 1, outcome: "refused", started_at: "2026-09-22T09:05:00Z" },
        { attempt: 2, outcome: "running", started_at: "2026-09-22T09:20:00Z" },
      ],
    });

    expect(taskGroupsOf(withTasks(["T1"], step))[0]?.retry_count).toBe(1);
  });
});
