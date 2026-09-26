// The run's placement and its edges, off the eight shipped workflows and the
// arc's own moments. Arithmetic, so it is unit-tested here rather than in a
// story paying a browser's price.

import { describe, expect, it } from "vitest";

import { ARC_MOMENTS } from "./fixtures/build/arc";
import { KIND_FIXTURES, KIND_NAMES } from "./fixtures/build/kinds";
import { taskGroupsOf } from "./draft/group";
import {
  groupNodeId,
  stepNodeId,
  stepTheGroupsWereMadeAt,
  stepThatWorksTheGroups,
  taskNodeId,
  worked,
  workflowRunOf,
} from "./workflow-canvas";

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
      // One edge per pair, and one more for each step that sends the run back.
      // **`epic.json` is the one shipped file that does** — `verdict_routing`
      // on `roll_up`, capped at five passes — and the returning edge is what
      // the wave's iteration count is drawn from (`#1544`).
      const loops = whole.steps.filter(
        (step) => step.pass !== undefined && step.verdict_routing_target !== undefined,
      ).length;
      expect(spine).toHaveLength(Math.max(0, whole.steps.length - 1) + loops);
      expect(spine.filter((edge) => edge.kind === "returns")).toHaveLength(loops);
    });

    it(`draws every group of ${fixture.job.handle} once, whatever its plan holds`, () => {
      const whole = wholeOf(fixture);
      const groups = taskGroupsOf(whole);
      const run = workflowRunOf({ whole, groups });
      const drawn = run.nodes.filter((node) => node.id.startsWith("group:")).map((node) => node.id);
      // A plan no step recorded has no node to come off, so its groups are
      // left undrawn rather than hung somewhere they were not made.
      const expected =
        stepTheGroupsWereMadeAt(whole) === undefined ? [] : groups.map((group) => groupNodeId(group.id));
      expect(drawn).toEqual(expected);
      expect(new Set(drawn).size).toBe(drawn.length);
    });
  }
});

/** The arc moment by name, with the Job it opens. */
function arc(name: string) {
  const moment = ARC_MOMENTS.find((one) => one.name === name);
  const opened = moment?.fixtures.find((one) => one.job.id === moment.opens);
  const watched = opened?.watched;
  if (watched?.state !== "read") throw new Error(`the ${name} moment has no detail`);
  return { whole: watched.detail, groups: moment?.draft.groups ?? [] };
}

/**
 * The density case the note names: twelve steps and four groups of three. No
 * shipped workflow is that long, so it is built here rather than photographed.
 */
function dense() {
  const { whole, groups } = arc("groupFailed");
  const one = whole.steps[0]!;
  const steps = Array.from({ length: 12 }, (_, at) => ({
    ...one,
    step_id: `s${at + 1}`,
    label: `Step ${at + 1}`,
    ordinal: at + 1,
  }));
  const spare = groups.flatMap((group) => group.tasks)[0]!;
  const four = groups.slice(0, 4).map((group, at) => ({
    ...group,
    tasks: [0, 1, 2].map((k) => ({ ...spare, id: `D${at * 3 + k + 1}`, group: group.id })),
  }));
  return {
    whole: {
      ...whole,
      steps,
      job: { ...whole.job, current_step_id: "s7" },
      work_plan: { ...whole.work_plan!, recorded_by: { by: "step" as const, step_id: "s1", attempt: 1 } },
    },
    groups: four,
  };
}

describe("twelve steps and four groups of three", () => {
  it("puts every node somewhere of its own, with each group's tasks between it and the next", () => {
    const { whole, groups } = dense();
    const run = workflowRunOf({ whole, groups });
    expect(run.nodes).toHaveLength(12 + 4 + 12);
    const where = run.nodes.map((node) => `${node.position.x},${node.position.y}`);
    expect(new Set(where).size).toBe(where.length);

    const at = (id: string) => run.nodes.find((node) => node.id === id)!.position;
    const columns = { group: new Set<number>(), task: new Set<number>() };
    for (const [j, group] of groups.entries()) {
      const mine = at(groupNodeId(group.id));
      columns.group.add(mine.x);
      const below = groups[j + 1];
      const floor = below === undefined ? Infinity : at(groupNodeId(below.id)).y;
      for (const task of group.tasks) {
        const node = at(taskNodeId(task.id));
        columns.task.add(node.x);
        expect(node.y).toBeGreaterThanOrEqual(mine.y);
        expect(node.y).toBeLessThan(floor);
      }
    }
    // One column of groups and one of tasks, the tasks to the right.
    expect(columns.group.size).toBe(1);
    expect(columns.task.size).toBe(1);
    expect([...columns.task][0]!).toBeGreaterThan([...columns.group][0]!);
  });

  it("opens on the step you are on and its neighbours, with no plan hanging off them", () => {
    const { whole, groups } = dense();
    const run = workflowRunOf({ whole, groups });
    // `s7` neither wrote the plan nor works it, so what opens is three steps.
    expect(run.opensOn[0]).toEqual(["s6", "s7", "s8"].map(stepNodeId));
    expect(run.opensOn[run.opensOn.length - 1]).toEqual([stepNodeId("s7")]);
  });
});

describe("a group comes off where it was made and where it was worked", () => {
  it("hangs every group off the step that recorded the plan, and never on the spine", () => {
    const { whole, groups } = arc("executingSequential");
    expect(groups.length).toBeGreaterThan(0);
    const run = workflowRunOf({ whole, groups });
    const madeAt = stepTheGroupsWereMadeAt(whole);
    const under = run.nodes.filter((node) => node.id.startsWith("group:"));
    expect(under).toHaveLength(groups.length);
    expect(under.every((node) => node.position.y > 0)).toBe(true);
    expect(new Set(under.map((node) => node.position.x)).size).toBe(1);
    // Every group's one `made` edge leaves the step that recorded the plan.
    const made = run.edges.filter((edge) => edge.kind === "made");
    expect(made).toHaveLength(groups.length);
    expect(new Set(made.map((edge) => edge.source))).toEqual(new Set([stepNodeId(madeAt!)]));
    expect(run.rows.filter((row) => row.depth === 1).map((row) => row.under)).toEqual(
      groups.map(() => stepNodeId(madeAt!)),
    );
  });

  it("draws a second edge into a worked group and none into an unworked one", () => {
    const { whole, groups } = arc("groupFailed");
    const run = workflowRunOf({ whole, groups });
    const worksAt = stepNodeId(stepThatWorksTheGroups(whole)!);
    const worked = run.edges.filter((edge) => edge.kind === "worked");
    // The moment has three groups off `pending` and one still waiting.
    const moving = groups.filter((group) => group.state !== "pending");
    expect(moving.length).toBeGreaterThan(0);
    expect(moving.length).toBeLessThan(groups.length);
    expect(worked.map((edge) => edge.target).sort()).toEqual(moving.map((one) => groupNodeId(one.id)).sort());
    expect(new Set(worked.map((edge) => edge.source))).toEqual(new Set([worksAt]));
    // Two edges into the worked node, one into the unworked — one node either way.
    for (const group of groups) {
      const into = run.edges.filter((edge) => edge.target === groupNodeId(group.id));
      expect(into).toHaveLength(group.state === "pending" ? 1 : 2);
    }
    // The stacked run says the same, in words.
    const said = run.rows.filter((row) => row.depth === 1);
    expect(said.filter((row) => row.worked !== undefined)).toHaveLength(moving.length);
    expect(new Set(said.map((row) => row.worked).filter(Boolean))).toEqual(new Set(["Implement"]));
  });

  it("hangs every task off its own group, in plan order", () => {
    const { whole, groups } = arc("groupFailed");
    const run = workflowRunOf({ whole, groups });
    for (const group of groups) {
      const groupId = groupNodeId(group.id);
      const holds = run.edges.filter((edge) => edge.kind === "holds" && edge.source === groupId);
      expect(holds.map((edge) => edge.target)).toEqual(group.tasks.map((task) => taskNodeId(task.id)));
      // A task sits right of its group and never above it.
      const at = run.nodes.find((node) => node.id === groupId)!;
      for (const task of group.tasks) {
        const node = run.nodes.find((one) => one.id === taskNodeId(task.id))!;
        expect(node.position.x).toBeGreaterThan(at.position.x);
        expect(node.position.y).toBeGreaterThanOrEqual(at.position.y);
      }
      // Stacked, a task hangs under its group rather than under the step.
      const rows = run.rows.filter((row) => row.under === groupId);
      expect(rows.map((row) => row.id)).toEqual(group.tasks.map((task) => taskNodeId(task.id)));
      expect(rows.every((row) => row.depth === 2)).toBe(true);
    }
  });

  it("names the step the Job is on, and narrows onto it rather than shrinking", () => {
    const { whole, groups } = arc("executingSequential");
    const run = workflowRunOf({ whole, groups });
    expect(run.running).toBe(stepNodeId(whole.job.current_step_id ?? ""));
    const widest = run.opensOn[0]!;
    // Narrow opens on where you are, which is never the whole run.
    expect(widest.filter((id) => id.startsWith("step:")).length).toBeLessThan(whole.steps.length);
    // The step the plan was recorded at is a neighbour here, so its groups come too.
    expect(widest).toContain(groupNodeId(groups[0]!.id));
    // Each choice is narrower than the one before it, and each holds the step
    // a person is on.
    for (const [at, choice] of run.opensOn.entries()) {
      expect(choice).toContain(run.running);
      // A task is the finest grain and is left out of every choice.
      expect(choice.some((id) => id.startsWith("task:"))).toBe(false);
      const before = run.opensOn[at - 1];
      if (before !== undefined) expect(before.length).toBeGreaterThan(choice.length);
    }
    // The narrowest keeps the whole plan the step works, the groups whose turn
    // has not come included — a frame too small for them draws a clipped plan
    // rather than one card in an empty pane.
    const last = run.opensOn[run.opensOn.length - 1]!;
    expect(last.filter((id) => id.startsWith("group:"))).toHaveLength(groups.length);
    expect(groups.some((group) => !worked(group))).toBe(true);
  });

  it("marks the card a person has open and no other", () => {
    const { whole, groups } = arc("groupFailed");
    const open = groupNodeId(groups[0]!.id);
    const run = workflowRunOf({ whole, groups, selected: open });
    expect(run.nodes.filter((node) => node.card.selected === true).map((node) => node.id)).toEqual([open]);
  });
});

describe("a step that loops draws a returning edge", () => {
  it("returns to the step it names, dashed, with its cap", () => {
    const whole = wholeOf(KIND_FIXTURES[1]!);
    const looping = {
      ...whole,
      steps: whole.steps.map((step, at) =>
        at === 1 ? { ...step, verdict_routing_target: whole.steps[0]!.step_id, pass: { number: 1, of: 5 } } : step,
      ),
    };
    const run = workflowRunOf({ whole: looping, groups: [] });
    const returning = run.edges.filter((edge) => edge.kind === "returns");
    expect(returning).toHaveLength(1);
    expect(returning[0]?.target).toBe(stepNodeId(whole.steps[0]!.step_id));
    expect(returning[0]?.label).toBe("up to 5 passes");
    // Stacked has nothing to arc over, so the row says where it goes back to.
    expect(run.rows[1]?.returns?.toName).toBe(whole.steps[0]!.label);
  });
});
