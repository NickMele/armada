// The plan's placement and its edges, off the arc's own moments. Arithmetic,
// so it is unit-tested here rather than in a story paying a browser's price.
//
// **These claims moved with the code they test** (owner, 25 Sep 2026): the
// group and task graph came off `workflow-canvas.ts` whole, so the placement
// tests came with it. What is gone rather than moved is the second edge —
// there are no step nodes here for one to leave.

import { describe, expect, it } from "vitest";

import { ARC_MOMENTS } from "./fixtures/build/arc";
import { groupNodeId, planActivityOf, planGraphOf, taskNodeId, taskOfNodeId } from "./plan-canvas";

/** The arc moment by name, with the plan it opens. */
function arc(name: string) {
  const moment = ARC_MOMENTS.find((one) => one.name === name);
  if (moment === undefined) throw new Error(`there is no ${name} moment`);
  return moment.draft.groups ?? [];
}

describe("a group is a root, and its tasks hang off it", () => {
  it("hangs every task off its own group, in plan order", () => {
    const groups = arc("groupFailed");
    expect(groups.length).toBeGreaterThan(0);
    const graph = planGraphOf({ groups });
    for (const group of groups) {
      const groupId = groupNodeId(group.id);
      const holds = graph.edges.filter((edge) => edge.kind === "holds" && edge.source === groupId);
      expect(holds.map((edge) => edge.target)).toEqual(group.tasks.map((task) => taskNodeId(task.id)));
      // A task sits right of its group and never above it.
      const at = graph.nodes.find((node) => node.id === groupId)!;
      for (const task of group.tasks) {
        const node = graph.nodes.find((one) => one.id === taskNodeId(task.id))!;
        expect(node.position.x).toBeGreaterThan(at.position.x);
        expect(node.position.y).toBeGreaterThanOrEqual(at.position.y);
      }
    }
    // Nothing joins one group to another: a plan is as many trees as it has groups.
    const roots = graph.nodes.filter((node) => node.id.startsWith("group:")).map((node) => node.id);
    expect(roots).toEqual(groups.map((group) => groupNodeId(group.id)));
    expect(graph.edges.filter((edge) => edge.target.startsWith("group:"))).toHaveLength(0);
  });

  it("puts every node somewhere of its own, with each group's tasks between it and the next", () => {
    const groups = arc("groupFailed");
    const graph = planGraphOf({ groups });
    const where = graph.nodes.map((node) => `${node.position.x},${node.position.y}`);
    expect(new Set(where).size).toBe(where.length);

    const at = (id: string) => graph.nodes.find((node) => node.id === id)!.position;
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

  it("opens on the groups when the whole plan will not read, and never on a task", () => {
    const groups = arc("groupFailed");
    const graph = planGraphOf({ groups });
    expect(graph.opensOn).toEqual([groups.map((group) => groupNodeId(group.id))]);
    for (const choice of graph.opensOn) {
      expect(choice.some((id) => id.startsWith("task:"))).toBe(false);
    }
  });
});

describe("a press opens a task and nothing else", () => {
  it("presses into the task's own id, and leaves a group a card rather than a control", () => {
    const groups = arc("groupFailed");
    const pressed: string[] = [];
    const graph = planGraphOf({ groups, onOpenTask: (id) => pressed.push(id) });
    const task = groups[0]!.tasks[0]!;
    graph.nodes.find((node) => node.id === taskNodeId(task.id))!.card.onOpen!();
    expect(pressed).toEqual([task.id]);
    expect(graph.nodes.find((node) => node.id === groupNodeId(groups[0]!.id))!.card.onOpen).toBeUndefined();
  });

  it("marks the task a person has open and no other", () => {
    const groups = arc("groupFailed");
    const task = groups[0]!.tasks[0]!;
    const graph = planGraphOf({ groups, openTask: task.id });
    expect(graph.nodes.filter((node) => node.card.selected === true).map((node) => node.id)).toEqual([
      taskNodeId(task.id),
    ]);
  });

  it("reads a task out of a node id, and nothing out of a group's", () => {
    expect(taskOfNodeId(taskNodeId("T3"))).toBe("T3");
    expect(taskOfNodeId(groupNodeId("g1"))).toBeUndefined();
  });
});

describe("the plan's own word, for the one node the run draws of it", () => {
  it("says what is wrong before what is moving, and what is moving before what is done", () => {
    const groups = arc("groupFailed");
    const one = groups[0]!;
    const failed = { ...one, state: "failed" as const };
    const running = { ...one, state: "running" as const };
    const passed = { ...one, state: "passed" as const };
    const pending = { ...one, state: "pending" as const };
    expect(planActivityOf([passed, running, failed]).said).toBe("failed");
    expect(planActivityOf([passed, running]).said).toBe("running");
    // A plan is not done while a group of it has not started.
    expect(planActivityOf([passed, pending]).said).toBe("pending");
    expect(planActivityOf([passed, passed]).said).toBe("passed");
    // And a plan with no group has nothing to have started.
    expect(planActivityOf([])).toEqual({ activity: "not_started", said: "pending" });
  });
});
