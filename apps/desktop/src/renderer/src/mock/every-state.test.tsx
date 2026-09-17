// The mock's own claim, through `App` and nothing else: on `every-state` the
// Board lists every Job the scenario holds, and each row opens that Job.
//
// **A Job is a test, not a turn of a loop.** Opening every Job inside one test
// spent one 5000ms budget on the whole board — 4628ms of it on an idle machine
// — so on a busy one the budget ran out mid-press and Playwright reported a
// click with 36ms, then one with `-8ms`, left to it. That reads as the app
// ignoring a press rather than as a test out of time, which is what it was
// (#1339). `test.for` gives each Job its own budget, the Done fold included,
// and the roster the loop opened with is a test of its own.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { scenarioNamed } from "./scenario";
import { mount, openBoard, rows, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * Every Job `every-state` holds, read while the file is collected so each one
 * is its own test. The same scenario `mount` will hand each test back.
 */
const JOBS = scenarioNamed("every-state")!.state.jobs;

/** The Board, with its collapsed Done group opened so every row is drawn. */
async function everyRowDrawn(): Promise<void> {
  await expect.poll(() => rows().length).toBeGreaterThan(0);
  const done = page.getByRole("button", { name: /^Done/ });
  if (done.query()?.getAttribute("aria-expanded") === "false") await done.click();
}

test("every-state lists a row per Job", async () => {
  mount("every-state");
  await openBoard();
  await everyRowDrawn();
  await expect.poll(() => rows().map((row) => row.dataset.jobId).sort()).toEqual(JOBS.map((job) => job.id).sort());
});

test.for(JOBS)("$handle's row opens its own detail", async (job) => {
  const { scenario } = mount("every-state");
  await openBoard();
  await everyRowDrawn();
  // The fold's rows arrive with it, so wait for this Job's row rather than reading the board once.
  const row = (): HTMLElement | undefined => rows().find((one) => one.dataset.jobId === job.id);
  await expect.poll(row).toBeDefined();
  await userEvent.click(row()!);
  // The Board is gone and the header names this Job by its handle.
  await expect.poll(() => rows().length).toBe(0);
  await expect.element(page.getByText(job.handle, { exact: true }).first()).toBeVisible();
  // Where the scenario holds the Job's detail, it is that detail drawn: its first step, by label.
  const watched = scenario.reads[job.id]?.watched;
  const step = watched?.state === "read" ? watched.detail.steps[0]?.label : undefined;
  if (step !== undefined) {
    await expect.element(page.getByText(step, { exact: true }).first()).toBeVisible();
  }
  // Escape puts the Board back, which the loop proved on every Job it went on from.
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => rows().length).toBeGreaterThan(0);
});

test("fleet-not-running draws what Bridge draws with no Fleet", async () => {
  mount("fleet-not-running");
  await expect.element(page.getByText("Fleet is not running").first()).toBeVisible();
});

test("a job scenario opens its Job on start", async () => {
  const { scenario } = mount("job/review");
  const job = scenario.state.jobs[0]!;
  await expect.element(page.getByText(job.handle, { exact: true }).first()).toBeVisible();
  expect(rows()).toHaveLength(0);
});
