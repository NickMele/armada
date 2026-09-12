// The diff a person opens is a reading taken for the press, not for the Job.
//
// # Why this is a browser test
//
// The call is made by an effect inside `JobDetail`, off the sheet's open state
// and off the live footprint — three things a function test cannot see. What
// this pins is what a person did: they opened a Job, waited while its Drone
// wrote, and pressed `Open the diff`.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { JobDetail } from "./JobDetail";
import { running } from "./fixtures/build";
import { propsFor } from "./fixtures/props";
import type { JobFixture } from "./fixtures/fixture";
import { mount, rerender, unmount } from "./mounted";

afterEach(unmount);

/** The same fixture with one more reading of what the Drone has written. */
function alsoWrote(fixture: JobFixture, paths: string[]): JobFixture {
  const observed = fixture.observed;
  if (!("turns" in observed)) throw new Error("the fixture is not watching");
  const wrote = {
    ts: "2026-09-10T14:31:00Z",
    seq: 9_000,
    step: "fix",
    by: "fleet" as const,
    saw: {
      event: "produced" as const,
      files: paths.map((path) => ({ path, change: "modified" as const, outside_plan: false })),
    },
  };
  return {
    ...fixture,
    observed: { ...observed, turns: { ...observed.turns, rows: [...observed.turns.rows, wrote] } },
  };
}

test("the press that opens the diff takes the reading again, and so does the file list moving", async () => {
  const fixture = running();
  const onReadDiff = vi.fn<(jobId: string | null) => void>();
  mount(<JobDetail {...propsFor(fixture)} onReadDiff={onReadDiff} />);

  // Opening the Job reads it once, which is what it has always done.
  await expect.element(page.getByRole("button", { name: "Open the diff" }).first()).toBeVisible();
  expect(onReadDiff.mock.calls).toEqual([[fixture.job.id]]);

  // **The press.** Ten minutes of a Drone writing sit between this and the read
  // above, and what the sheet drew was the worktree as it was before any of it.
  onReadDiff.mockClear();
  await userEvent.click(page.getByRole("button", { name: "Open the diff" }).first());
  await expect.element(page.getByText("Job diff")).toBeVisible();
  expect(onReadDiff).toHaveBeenCalledWith(fixture.job.id);

  // And the Drone writes again while the sheet is open, which is the Produced
  // chapter moving under it — one worktree, and now one answer.
  onReadDiff.mockClear();
  rerender(
    <JobDetail {...propsFor(alsoWrote(fixture, ["src/late.ts"]))} onReadDiff={onReadDiff} />,
  );
  await vi.waitFor(() => expect(onReadDiff).toHaveBeenCalledWith(fixture.job.id));
});
