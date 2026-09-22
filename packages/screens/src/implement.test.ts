// What the implement board says, over the four moments inside the step.
// `#1536`. Arithmetic and copy are proved here so the browser tests are left
// with what only a rendering can show.

import { describe, expect, test } from "vitest";

import {
  doneTouched,
  executingConcurrent,
  executingSequential,
  groupFailed,
} from "./fixtures/build/arc";
import type { ArcMoment } from "./fixtures/build/arc-base";
import type { GroupView } from "./draft/group";
import {
  boundaryOf,
  groupsThatOpen,
  implementBoardOf,
  shapeSaid,
  stopsSaid,
  tierSaid,
  toldNextOf,
  verdictSaid,
} from "./implement";

/** The Job whole behind a moment, and the step its groups hang under. */
function reading(moment: ArcMoment) {
  const fixture = moment.fixtures[0]!;
  const held = fixture.watched.state === "read" ? fixture.watched.detail : null;
  const groups = [...moment.draft.groups!];
  const step = held?.steps.find((one) => one.step_id === "implement");
  return { whole: held, groups, step, cases: [...(moment.draft.cases ?? [])] };
}

/** The board a moment draws, with nothing opened by hand. */
function board(moment: ArcMoment) {
  const read = reading(moment);
  return implementBoardOf({
    ...read,
    openGroups: groupsThatOpen(read.groups),
    onOpenGroup: () => undefined,
    onOpenTask: () => undefined,
  });
}

const groupAt = (moment: ArcMoment, ordinal: number) =>
  board(moment)!.groups.find((one) => one.ordinal === ordinal)!;

const taskIn = (moment: ArcMoment, ordinal: number, id: string) =>
  groupAt(moment, ordinal).tasks.find((one) => one.id === id)!;

describe("one group at a time", () => {
  test("the board is the step's own label, and every group is drawn in order", () => {
    const drawn = board(executingSequential())!;
    expect(drawn.stepName).toBe("Implement");
    expect(drawn.groups.map((one) => one.ordinal)).toEqual([1, 2, 3, 4]);
  });

  test("groups one and two read passed with the commit each left", () => {
    expect(groupAt(executingSequential(), 1).says).toBe("passed");
    expect(groupAt(executingSequential(), 1).commit).toBe("4c1b9d2");
    expect(groupAt(executingSequential(), 2).commit).toBe("7a2f0c5");
  });

  test("group four has not started, and its Checks read as not run", () => {
    const four = groupAt(executingSequential(), 4);
    expect(four.says).toBe("not started");
    expect(four.boundary.says).toBe("7 checks will run at this boundary");
    expect(new Set(four.boundary.checks.map((one) => one.reads))).toEqual(new Set(["not run"]));
    expect(four.boundary.verdictSays).toBeUndefined();
  });

  // `groupsThatOpen` is the whole of what a person sees first, and it is the
  // difference between reading a run and opening four groups in turn.
  test("the group that is moving opens itself, and a run with none open falls to the last", () => {
    const moving = reading(executingSequential()).groups;
    expect(groupsThatOpen(moving)).toEqual(["g3"]);
    const over: GroupView[] = moving.map((one) => ({ ...one, state: "passed" }));
    expect(groupsThatOpen(over)).toEqual(["g4"]);
    expect(groupsThatOpen([])).toEqual([]);
  });
});

describe("a task carries only its own agent", () => {
  test("a working task shows turns and no cost, because its agent has not stopped", () => {
    const five = taskIn(executingSequential(), 3, "T5");
    expect(five.spentSays).toBe("14 turns");
    expect(five.spentSays).not.toContain("$");
  });

  test("a finished task shows what it cost, in a group that has already passed", () => {
    expect(taskIn(executingSequential(), 1, "T1").spentSays).toBe("34 turns · ~$2.40");
  });

  test("the tier, the model it resolved to and how it is run are one line", () => {
    expect(taskIn(executingSequential(), 1, "T1").says).toBe("difficult · opus · its own agent");
    expect(taskIn(executingSequential(), 2, "T3").says).toBe("easy · haiku · its own agent");
  });

  test("a cost is on a task the moment its agent stopped, before its group is checked", () => {
    const five = taskIn(executingConcurrent(), 3, "T5");
    const six = taskIn(executingConcurrent(), 3, "T6");
    expect(five.spentSays).toBe("27 turns · ~$1.90");
    expect(six.spentSays).toBe("15 turns · ~$0.72");
    expect(groupAt(executingConcurrent(), 3).boundary.verdictSays).toBeUndefined();
  });
});

describe("fan out, then join", () => {
  test("a concurrent group says its tasks run at the same time, and names what each is beside", () => {
    expect(groupAt(executingConcurrent(), 3).shapeSays).toBe("2 tasks, at the same time");
    expect(taskIn(executingConcurrent(), 3, "T5").besideSays).toBe("runs beside T6");
    expect(taskIn(executingConcurrent(), 3, "T6").besideSays).toBe("runs beside T5");
  });

  test("a group whose tasks run in order says so, and a group of one says it runs alone", () => {
    const first = reading(executingConcurrent()).groups[0]!;
    expect(groupAt(executingConcurrent(), 1).shapeSays).toBe("2 tasks, one after another");
    expect(shapeSaid({ ...first, tasks: [first.tasks[0]!] })).toBe("1 task, on its own");
  });

  test("the group joining its work says so, which is not the same as checking", () => {
    expect(groupAt(executingConcurrent(), 3).says).toBe("joining its work");
  });
});

describe("a boundary that failed", () => {
  test("the Check that failed is named, the six that passed are drawn beside it", () => {
    const three = groupAt(groupFailed(), 3).boundary;
    expect(three.verdictSays).toBe("screens_test failed");
    expect(three.verdictNamed).toBe("failed");
    expect(three.checks.filter((one) => one.reads === "failed").map((one) => one.name)).toEqual([
      "screens_test",
    ]);
    expect(three.checks.filter((one) => one.reads === "passed")).toHaveLength(6);
  });

  test("it says this is its second run, and which group it is holding back", () => {
    const three = groupAt(groupFailed(), 3).boundary;
    expect(three.retrySays).toBe("second run");
    expect(three.stopsSays).toBe("No task of group 4 starts until this boundary passes.");
  });

  test("what the next Drone is told is the Check's own output, not a summary", () => {
    const three = groupAt(groupFailed(), 3).boundary;
    expect(three.toldNext).toContain("1 of 1384 failed");
    expect(three.toldNext).toContain("every test in the screens package passes");
  });

  // A step's `check_runs` is one list for every group in it, so a passed group
  // taking a later group's red is the defect this guards.
  test("a group that passed takes no red from another group's failed run", () => {
    const one = groupAt(groupFailed(), 1).boundary;
    expect(one.verdictSays).toBe("all 4 passed");
    expect(one.checks.some((check) => check.reads === "failed")).toBe(false);
    expect(one.toldNext).toBeUndefined();
    expect(one.stopsSays).toBeUndefined();
  });

  test("a task whose own agent stopped without finishing carries its reason", () => {
    expect(taskIn(groupFailed(), 3, "T6").failedReason).toBe(
      "The row's press opened the Board rather than the Job",
    );
  });
});

describe("a done task a later task edited", () => {
  test("it stays done and is flagged with the task that did it", () => {
    const six = taskIn(doneTouched(), 3, "T6");
    expect(six.mark).toBe("done");
    expect(six.touchedSays).toBe("touched later · T7");
  });

  test("the task that did it is still working, with turns and no cost", () => {
    expect(taskIn(doneTouched(), 4, "T7").spentSays).toBe("6 turns");
  });
});

describe("what the board refuses to draw", () => {
  test("a Job with no group draws no board at all, rather than an empty one", () => {
    const read = reading(executingSequential());
    expect(
      implementBoardOf({
        ...read,
        groups: [],
        openGroups: [],
        onOpenGroup: () => undefined,
        onOpenTask: () => undefined,
      }),
    ).toBeUndefined();
  });

  test("a plan whose step cannot be found draws none either", () => {
    const read = reading(executingSequential());
    expect(
      implementBoardOf({
        ...read,
        step: undefined,
        openGroups: [],
        onOpenGroup: () => undefined,
        onOpenTask: () => undefined,
      }),
    ).toBeUndefined();
  });

  test("a boundary with no Check says so rather than drawing an empty bar", () => {
    const read = reading(executingSequential());
    const bare = { ...read.groups[3]!, checks_selected: [] };
    const drawn = boundaryOf(bare, undefined, [], read.whole, read.step);
    expect(drawn.checks).toEqual([]);
    expect(drawn.checksAbsent).toBe("No Check runs at this group's end.");
  });

  // Fleet serves no cases, and an empty list there would read as "none owed",
  // which is the confusion the two regions were separated for.
  test("a boundary with no case names what is missing rather than reading as none owed", () => {
    const three = groupAt(executingSequential(), 3).boundary;
    expect(three.testsAbsent).toContain("does not serve the cases");
  });

  test("a boundary nothing has reached says nothing about a verdict", () => {
    const read = reading(executingSequential());
    expect(verdictSaid(read.groups[3]!, [])).toBeUndefined();
    expect(stopsSaid(read.groups[3]!, read.groups[3])).toBeUndefined();
    expect(toldNextOf(read.step, [])).toBeUndefined();
  });

  test("the tier line never invents a model the planner did not choose a tier for", () => {
    const task = reading(executingSequential()).groups[0]!.tasks[0]!;
    expect(tierSaid({ ...task, treatment: "step_drone" })).toContain("the step's Drone");
    expect(tierSaid({ ...task, treatment: "job" })).toContain("a Job of its own");
  });
});
