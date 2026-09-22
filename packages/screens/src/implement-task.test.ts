// One task, read for the inspector beside the run. `#1536`.
//
// **Every absence here is named.** A Job nothing is watching, a task nothing
// has written for and a planner that recorded no words are three different
// facts, and a blank region would read as the same one.

import { describe, expect, test } from "vitest";

import { executingConcurrent, executingSequential, groupFailed } from "./fixtures/build/arc";
import type { TaskView } from "./draft/task";
import {
  NO_BRIEF,
  NO_EDIT_READ,
  NO_TURNS_WATCHED,
  briefOfTask,
  doingOfTask,
  taskReadingOf,
} from "./implement-task";

/** The Job whole behind a moment, and its groups. */
function reading(moment: ReturnType<typeof executingSequential>) {
  const fixture = moment.fixtures[0]!;
  return {
    whole: fixture.watched.state === "read" ? fixture.watched.detail : null,
    groups: [...moment.draft.groups!],
  };
}

const inspector = (moment: ReturnType<typeof executingSequential>, taskId: string) =>
  taskReadingOf({ ...reading(moment), taskId, turns: [], watching: false, stepId: "implement" });

const taskNamed = (moment: ReturnType<typeof executingSequential>, id: string): TaskView =>
  reading(moment)
    .groups.flatMap((one) => one.tasks)
    .find((one) => one.id === id)!;

describe("what the panel says a task is doing", () => {
  test("a working task names its turns and says when a cost will read", () => {
    const said = doingOfTask(taskNamed(executingSequential(), "T5"));
    expect(said).toContain("14 turns");
    expect(said).toContain("once that agent stops");
    expect(said).not.toContain("$");
  });

  test("a finished task names what its own agent spent", () => {
    expect(doingOfTask(taskNamed(executingConcurrent(), "T5"))).toBe(
      "Its agent stopped after 27 turns · ~$1.90.",
    );
  });

  test("a failed task says why its agent stopped, in the words the run recorded", () => {
    expect(doingOfTask(taskNamed(groupFailed(), "T6"))).toBe(
      "The row's press opened the Board rather than the Job",
    );
  });

  test("a task nothing has run yet says so rather than reading as idle", () => {
    expect(doingOfTask(taskNamed(executingSequential(), "T8"))).toContain("Nothing has been dispatched");
  });
});

describe("what its Drone was told", () => {
  test("the planner's own words are carried, never a paraphrase", () => {
    expect(briefOfTask(taskNamed(executingSequential(), "T5"))).toContain(
      "The panel lists Drones, Checks, Judge calls and proposer calls",
    );
  });

  test("a task the planner said nothing about names the absence", () => {
    const bare = { ...taskNamed(executingSequential(), "T5") };
    delete bare.note;
    delete bare.expects;
    expect(briefOfTask(bare)).toBeUndefined();
    expect(inspector(executingSequential(), "T5")).toBeDefined();
  });
});

describe("the whole reading", () => {
  test("it is the task's id and title, drawn as a task", () => {
    const read = inspector(executingSequential(), "T5")!;
    expect(read.kind).toBe("task");
    expect(read.name).toBe("T5 · Draw what is running, in four lists");
  });

  test("it carries what the task claims and what it runs beside", () => {
    const read = inspector(executingConcurrent(), "T5")!;
    expect(read.scope).toEqual(["packages/screens/src/Running.tsx"]);
    expect(read.beside).toEqual(["T6"]);
  });

  test("the redirect is addressed to that task's own Drone, not the Job's", () => {
    const read = inspector(executingConcurrent(), "T6")!;
    expect(read.drones).toEqual([{ id: "01M2D5HKQP001DRONE0000T6", label: "Drone on T6" }]);
  });

  // The two absences are different facts: nothing was read, and nothing is
  // reading. A panel that drew one blank region for both would say neither.
  test("with nothing watching, the log and the last edit each name their own absence", () => {
    const read = inspector(executingSequential(), "T5")!;
    expect(read.log).toEqual([]);
    expect(read.logAbsent).toBe(NO_TURNS_WATCHED);
    expect(read.lastEditAbsent).toBe(NO_EDIT_READ);
    expect(NO_BRIEF).toContain("no words of its own");
  });

  test("a task the plan does not hold reads as nothing, rather than as an empty task", () => {
    expect(
      taskReadingOf({ ...reading(executingSequential()), taskId: "T99", turns: [], watching: false }),
    ).toBeUndefined();
  });
});

// One Drone per Job today, so a fallback labelled "Drone on T5" would be a
// claim the wire does not make.
test("a task with no Drone of its own says the box reaches the Job's Drone", () => {
  const read = reading(executingSequential());
  const groups = read.groups.map((group) => ({
    ...group,
    tasks: group.tasks.map((task) => {
      const bare = { ...task };
      delete bare.drone_id;
      return bare;
    }),
  }));
  const said = taskReadingOf({ ...read, groups, taskId: "T5", turns: [], watching: false })!;
  expect(said.drones).toEqual([{ id: read.whole!.job.assigned_drone, label: "This Job's Drone" }]);
});
