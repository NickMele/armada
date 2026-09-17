// The mock's own claim, through `App` and nothing else: on `every-state` the
// Board lists every Job the scenario holds, and each row opens that Job.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp } from "./mount";
import type { Mounted } from "./mount";

let mounted: { app: Mounted; host: HTMLElement } | null = null;

afterEach(() => {
  mounted?.app.unmount();
  mounted?.host.remove();
  mounted = null;
});

/** Mount `App` on a scenario, in a host the app's stylesheet sizes as its window. */
function mount(name: string): Mounted {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const app = mountApp(name, host);
  mounted = { app, host };
  return app;
}

const rows = () => [...document.querySelectorAll<HTMLElement>("[data-job-id]")];

/** The Board, with its collapsed Done group opened so every row is drawn. */
async function everyRowDrawn(): Promise<void> {
  await expect.poll(() => rows().length).toBeGreaterThan(0);
  const done = page.getByRole("button", { name: /^Done/ });
  if (done.query()?.getAttribute("aria-expanded") === "false") await done.click();
}

test("every-state lists a row per Job, and every row opens its own detail", async () => {
  const { scenario } = mount("every-state");
  const jobs = scenario.state.jobs;
  await page.getByRole("button", { name: "Job Board" }).first().click();
  await everyRowDrawn();
  await expect.poll(() => rows().map((row) => row.dataset.jobId).sort()).toEqual(jobs.map((job) => job.id).sort());

  for (const job of jobs) {
    await everyRowDrawn();
    const row = rows().find((one) => one.dataset.jobId === job.id);
    expect(row, job.handle).toBeDefined();
    await userEvent.click(row!);
    // The Board is gone and the header names this Job by its handle.
    await expect.poll(() => rows().length, { message: job.handle }).toBe(0);
    await expect.element(page.getByText(job.handle, { exact: true }).first(), { message: job.handle }).toBeVisible();
    // Where the scenario holds the Job's detail, it is that detail drawn: its first step, by label.
    const watched = scenario.reads[job.id]?.watched;
    const step = watched?.state === "read" ? watched.detail.steps[0]?.label : undefined;
    if (step !== undefined) {
      await expect.element(page.getByText(step, { exact: true }).first(), { message: job.handle }).toBeVisible();
    }
    await userEvent.keyboard("{Escape}");
  }
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
