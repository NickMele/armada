// The run's placement and its edges, off the eight shipped workflows and the
// arc's own moments. Arithmetic, so it is unit-tested here rather than in a
// story paying a browser's price.

import { describe, expect, it } from "vitest";

import { ARC_MOMENTS } from "./fixtures/build/arc";
import { KIND_FIXTURES, KIND_NAMES } from "./fixtures/build/kinds";
import { taskGroupsOf } from "./draft/group";
import { stepNodeId, workflowRunOf } from "./workflow-canvas";

/** The Job whole, from a fixture's own `GET /jobs/:job_id` answer. */
function wholeOf(fixture: (typeof KIND_FIXTURES)[number]) {
  const watched = fixture.watched;
  if (watched.state !== "read") throw new Error(`${fixture.job.handle} has no detail`);
  return watched.detail;
}

describe("every shipped workflow draws", () => {
  it("covers each of the eight workflow files and no ninth", () => {
    expect(KIND_FIXTURES).toHaveLength(KIND_NAMES.length);
    expect(KIND_NAMES).not.toContain("verify-and-ship");
  });

  for (const fixture of KIND_FIXTURES) {
    it(`draws ${fixture.job.handle} on the steps its own file declares`, () => {
      const whole = wholeOf(fixture);
      const run = workflowRunOf({ whole, groups: taskGroupsOf(whole) });
      const steps = whole.steps.map((step) => stepNodeId(step.step_id));
      // Every step is a node, in the workflow's own order, and nothing else is.
      expect(run.nodes.filter((node) => node.id.startsWith("step:")).map((node) => node.id)).toEqual(steps);
      // The same run, stacked, holds the same nodes in the same order.
      expect(run.rows.map((row) => row.id)).toEqual(expect.arrayContaining(steps));
    });

    it(`places ${fixture.job.handle}'s steps left to right, evenly and in order`, () => {
      const whole = wholeOf(fixture);
      const run = workflowRunOf({ whole, groups: taskGroupsOf(whole) });
      const spine = run.nodes.filter((node) => node.id.startsWith("step:"));
      const xs = spine.map((node) => node.position.x);
      expect([...xs].sort((a, b) => a - b)).toEqual(xs);
      expect(new Set(xs).size).toBe(xs.length);
      expect(spine.every((node) => node.position.y === 0)).toBe(true);
    });

    it(`joins ${fixture.job.handle}'s steps one to the next, and no further`, () => {
      const whole = wholeOf(fixture);
      const run = workflowRunOf({ whole, groups: taskGroupsOf(whole) });
      const spine = run.edges.filter((edge) => edge.source.startsWith("step:") && edge.target.startsWith("step:"));
      expect(spine).toHaveLength(Math.max(0, whole.steps.length - 1));
      // No shipped workflow declares a loop, so none is drawn for one.
      expect(spine.filter((edge) => edge.returning === true)).toHaveLength(0);
    });
  }
});

describe("the implement step opens into its groups", () => {
  const executing = ARC_MOMENTS.find((moment) => moment.name === "executingSequential");
  const opened = executing?.fixtures.find((one) => one.job.id === executing.opens);

  it("hangs every group under one step, and never on the spine", () => {
    const watched = opened?.watched;
    if (watched?.state !== "read") throw new Error("the executing moment has no detail");
    const whole = watched.detail;
    const groups = taskGroupsOf(whole);
    expect(groups.length).toBeGreaterThan(0);
    const run = workflowRunOf({ whole, groups });
    const under = run.nodes.filter((node) => node.id.startsWith("group:"));
    expect(under).toHaveLength(groups.length);
    expect(under.every((node) => node.position.y > 0)).toBe(true);
    expect(new Set(under.map((node) => node.position.x)).size).toBe(1);
    expect(run.rows.filter((row) => row.under !== undefined)).toHaveLength(groups.length);
  });

  it("names the step the Job is on, for staying on it", () => {
    const watched = opened?.watched;
    if (watched?.state !== "read") throw new Error("the executing moment has no detail");
    const whole = watched.detail;
    const run = workflowRunOf({ whole, groups: taskGroupsOf(whole) });
    expect(run.running).toBe(stepNodeId(whole.job.current_step_id ?? ""));
    // Narrow opens on where you are, which is never the whole run of four.
    expect(run.opensOn.length).toBeLessThan(whole.steps.length);
    expect(run.opensOn).toContain(run.running);
  });
});

describe("a step that loops draws a returning edge", () => {
  it("returns to the step it names, dashed, with its cap", () => {
    const whole = wholeOf(KIND_FIXTURES[1]!);
    const looping = {
      ...whole,
      steps: whole.steps.map((step, at) =>
        at === 1 ? { ...step, verdict_routing_target: whole.steps[0]!.step_id, pass: { of: 5, taken: 1 } } : step,
      ),
    };
    const run = workflowRunOf({ whole: looping, groups: [] });
    const returning = run.edges.filter((edge) => edge.returning === true);
    expect(returning).toHaveLength(1);
    expect(returning[0]?.target).toBe(stepNodeId(whole.steps[0]!.step_id));
    expect(returning[0]?.label).toBe("up to 5 passes");
    // Stacked has nothing to arc over, so the row says where it goes back to.
    expect(run.rows[1]?.returns?.toName).toBe(whole.steps[0]!.label);
  });
});
