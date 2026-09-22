// What a task draws from today's wire, and what it refuses to invent.

import { describe, expect, it } from "vitest";

import { sampleDetail, samplePlan, sampleTask } from "./sample";
import { taskViewOf, taskViewsOf } from "./task";

describe("the wire fields a task carries through", () => {
  it("keeps the planner's scope, expectation and the work's own claim", () => {
    const detail = sampleDetail();
    const view = taskViewOf(
      detail,
      sampleTask({
        scope: ["packages/screens/src/draft/task.ts"],
        expects: "a unit test",
        shown: "fourteen of them",
        note: "not the place paths go",
      }),
    );

    expect(view.scope).toEqual(["packages/screens/src/draft/task.ts"]);
    expect(view.expects).toBe("a unit test");
    expect(view.shown).toBe("fourteen of them");
    expect(view.note).toBe("not the place paths go");
  });

  it("leaves a field the wire omitted omitted, rather than blank", () => {
    const view = taskViewOf(sampleDetail(), sampleTask());

    expect("note" in view).toBe(false);
    expect("expects" in view).toBe(false);
    expect("reason" in view).toBe(false);
    expect(view.scope).toEqual([]);
  });

  it("carries a drop's reason, which is on a dropped task and nothing else", () => {
    const view = taskViewOf(
      sampleDetail(),
      sampleTask({ state: "dropped", reason: "the boards do not need it" }),
    );

    expect(view.state).toBe("dropped");
    expect(view.reason).toBe("the boards do not need it");
  });
});

describe("what the derivation fills in where the wire says nothing", () => {
  it("marks nothing as touched after done", () => {
    const view = taskViewOf(sampleDetail(), sampleTask({ state: "done" }));

    expect(view.touched_after_done).toBe(false);
  });

  it("runs every task on the step's Drone, because a Job has one", () => {
    const view = taskViewOf(sampleDetail(), sampleTask());

    expect(view.treatment).toBe("step_drone");
    expect(view.concurrent_with).toEqual([]);
  });

  it("takes the model from the Job, since no tier map is served", () => {
    const detail = sampleDetail();
    detail.job.model = "opus";

    expect(taskViewOf(detail, sampleTask()).model).toBe("opus");
    expect(taskViewOf(detail, sampleTask()).tier).toBe("medium");
  });

  it("names the Job's Drone only on the task that is working", () => {
    const detail = sampleDetail();
    detail.job.assigned_drone = "01DRONE";

    expect(taskViewOf(detail, sampleTask({ state: "working" })).drone_id).toBe("01DRONE");
    expect(taskViewOf(detail, sampleTask({ state: "open" })).drone_id).toBeUndefined();
  });

  it("gives no cost and no turns, which arrive on a session's last line", () => {
    const view = taskViewOf(sampleDetail(), sampleTask({ state: "done" }));

    expect(view.cost_micros).toBeUndefined();
    expect(view.turns).toBeUndefined();
  });

  it("owes no cases, because nothing resolves them yet", () => {
    expect(taskViewOf(sampleDetail(), sampleTask()).cases).toEqual([]);
  });
});

describe("a state the wire cannot produce", () => {
  it("reads an unrecognised spelling as open rather than guessing", () => {
    const view = taskViewOf(sampleDetail(), sampleTask({ state: "blocked" }));

    expect(view.state).toBe("open");
  });

  it("carries failed through where something does send it", () => {
    const view = taskViewOf(sampleDetail(), sampleTask({ state: "failed" }));

    expect(view.state).toBe("failed");
  });
});

describe("a Job's tasks, in plan order", () => {
  it("is empty where no plan was recorded, which is not an error", () => {
    expect(taskViewsOf(sampleDetail())).toEqual([]);
  });

  it("keeps the plan's own order", () => {
    const detail = sampleDetail({
      work_plan: samplePlan([
        sampleTask({ id: "T1" }),
        sampleTask({ id: "T2" }),
        sampleTask({ id: "T3" }),
      ]),
    });

    expect(taskViewsOf(detail).map((task) => task.id)).toEqual(["T1", "T2", "T3"]);
  });
});
