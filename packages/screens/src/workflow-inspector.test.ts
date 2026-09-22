// What the inspector reads for the step or group a person has open.

import { describe, expect, it } from "vitest";

import { ARC_MOMENTS } from "./fixtures/build/arc";
import { KIND_FIXTURES } from "./fixtures/build/kinds";
import { taskGroupsOf } from "./draft/group";
import { groupNodeId, stepNodeId, stepTheGroupsHangUnder } from "./workflow-canvas";
import { NO_CASES_SERVED, workflowReadingOf } from "./workflow-inspector";

const executing = ARC_MOMENTS.find((moment) => moment.name === "executingSequential")!;
const opened = executing.fixtures.find((one) => one.job.id === executing.opens)!;
const watched = opened.watched;
if (watched.state !== "read") throw new Error("the executing moment has no detail");
const whole = watched.detail;
const groups = taskGroupsOf(whole);
const groupsUnder = stepTheGroupsHangUnder(whole);

describe("nothing open", () => {
  it("reads nothing for no selection, and nothing for a node this Job has not", () => {
    expect(workflowReadingOf({ whole, groups, selected: null })).toBeUndefined();
    expect(workflowReadingOf({ whole, groups, selected: "step:not-a-step" })).toBeUndefined();
  });
});

describe("a step", () => {
  const reading = workflowReadingOf({
    whole,
    groups,
    groupsUnder,
    selected: stepNodeId(whole.steps[1]!.step_id),
  })!;

  it("names the step, what it is doing, and how many groups it opens into", () => {
    expect(reading.kind).toBe("step");
    expect(reading.name).toBe(whole.steps[1]!.label);
    expect(reading.doing).toContain(`${groups.length} group`);
  });

  it("draws every Check the step declares, once each, whichever glob selected it", () => {
    const declared = (whole.steps[1]!.checks ?? []).map((check) => check.name ?? check.kind);
    // `implement` declares `test` twice — once for Rust and once for Bridge —
    // and one command is one row.
    expect(new Set(declared).size).toBeLessThan(declared.length);
    expect(reading.checks?.map((check) => check.name)).toEqual([...new Set(declared)]);
  });

  it("draws the tests apart from the Checks, and says why there are none", () => {
    expect(reading.tests).toEqual([]);
    expect(reading.testsAbsent).toBe(NO_CASES_SERVED);
  });
});

describe("a group", () => {
  const group = groups[0]!;
  const reading = workflowReadingOf({ whole, groups, groupsUnder, selected: groupNodeId(group.id) })!;

  it("holds the tasks that group holds and nothing beside them", () => {
    expect(reading.kind).toBe("group");
    expect(reading.tasks?.map((task) => task.id)).toEqual(group.tasks.map((task) => task.id));
  });

  it("draws only the Checks selected at this group's boundary", () => {
    for (const check of reading.checks ?? []) {
      expect(group.checks_selected).toContain(check.name);
    }
  });

  it("names the Drone a redirect from here would reach", () => {
    // One Drone per Job today, so a group's redirect reaches the Job's own.
    const expected = whole.job.assigned_drone === undefined ? 0 : 1;
    expect(reading.drones.length).toBeGreaterThanOrEqual(expected);
  });
});

describe("a Job with no plan", () => {
  it("says no plan was recorded rather than drawing an empty list", () => {
    const bare = KIND_FIXTURES.map((fixture) => fixture.watched)
      .filter((one) => one.state === "read")
      .map((one) => one.detail)
      .find((detail) => detail.work_plan === undefined);
    if (bare === undefined) return;
    const reading = workflowReadingOf({
      whole: bare,
      groups: [],
      selected: stepNodeId(bare.steps[0]!.step_id),
    })!;
    expect(reading.tasks).toEqual([]);
    expect(reading.tasksAbsent).toContain("No plan has been recorded");
  });
});
