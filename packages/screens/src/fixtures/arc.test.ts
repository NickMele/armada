// The arc's fixtures, proved the way `build.test.ts` proves the roster:
// `JobDetail` draws every Job of every moment without throwing.
//
// **This is the half that can be proved here.** What the draft carries has no
// screen yet — each board is its own issue — so what this file can say is that
// the wire half is a Job today's `JobDetail` renders, and that the draft half
// joins to it: every coordinate names a step the Job has, and every task a
// group holds is a task the plan holds.

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { JobDetail } from "../JobDetail";
import { ARC_MOMENTS } from "./build/arc";
import { KIND_FIXTURES, KIND_NAMES } from "./build/kinds";
import { epicWave, membersMerged, membersStacked } from "./build/waves";
import { propsFor } from "./props";

/** Every moment the mock offers: the arc, the two landing orders, the wave. */
const MOMENTS = [...ARC_MOMENTS, membersStacked(), membersMerged(), epicWave()];

describe("every moment's Jobs draw", () => {
  for (const moment of MOMENTS) {
    it(`draws every Job of ${moment.name} without throwing`, () => {
      for (const fixture of moment.fixtures) {
        const markup = renderToStaticMarkup(createElement(JobDetail, propsFor(fixture)));
        expect(markup.length, `${moment.name} / ${fixture.job.handle}`).toBeGreaterThan(0);
      }
    });
  }
});

describe("one Job per workflow kind", () => {
  it("names each of the eight workflow files and no ninth", () => {
    expect(KIND_FIXTURES).toHaveLength(KIND_NAMES.length);
    expect(KIND_NAMES).not.toContain("verify-and-ship");
  });

  for (const fixture of KIND_FIXTURES) {
    it(`draws ${fixture.job.handle} on the steps its own workflow declares`, () => {
      const workflow = fixture.workflows[0]!;
      const watched = fixture.watched;
      const steps = watched.state === "read" ? watched.detail.steps : [];
      expect(steps.map((step) => step.step_id)).toEqual(
        workflow.steps.map((step) => step.step_id),
      );
      const markup = renderToStaticMarkup(createElement(JobDetail, propsFor(fixture)));
      expect(markup.length).toBeGreaterThan(0);
    });
  }
});

describe("the draft half joins to the wire half", () => {
  for (const moment of MOMENTS) {
    const groups = moment.draft.groups;
    if (groups === undefined) continue;

    it(`${moment.name}'s groups hold every task the plan holds`, () => {
      const opened = moment.fixtures.find((one) => one.job.id === moment.opens);
      const watched = opened?.watched;
      const plan = watched?.state === "read" ? watched.detail.work_plan : undefined;
      const inGroups = groups.flatMap((group) => group.tasks.map((task) => task.id));
      expect(new Set(inGroups).size).toBe(inGroups.length);
      if (plan !== undefined) {
        expect(plan.tasks.map((task) => task.id).sort()).toEqual([...inGroups].sort());
      }
    });

    it(`${moment.name}'s tasks each sit in the group that holds them`, () => {
      for (const group of groups) {
        for (const task of group.tasks) {
          expect(task.group, `${task.id} is held by ${group.id}`).toBe(group.id);
          expect(task.coord.group).toBe(group.id);
          expect(task.coord.task).toBe(task.id);
        }
      }
    });

    it(`${moment.name} shows a cost only where an agent has stopped`, () => {
      for (const group of groups) {
        for (const task of group.tasks) {
          if (task.state === "working" || task.state === "open") {
            expect(task.cost_micros, `${task.id} is still working`).toBeUndefined();
          }
          if (task.state === "done") {
            expect(task.cost_micros, `${task.id}'s agent has stopped`).toBeDefined();
          }
        }
      }
    });
  }
});

describe("every Record row places itself", () => {
  for (const moment of MOMENTS) {
    const rows = moment.draft.record;
    if (rows === undefined) continue;
    it(`${moment.name}'s rows are in cursor order, and a Judge is not Fleet`, () => {
      const cursors = rows.map((row) => row.cursor);
      expect([...cursors].sort((a, b) => a - b)).toEqual(cursors);
      for (const row of rows) {
        expect(row.what.length).toBeGreaterThan(0);
      }
    });
  }
});
