// A job that was killed and redispatched, through `App`. #1439.
//
// **The defect was navigational**, so the test is too: the owner landed on the
// job that had been replaced, read that it was killed, and had to find its
// successor by hand. What is pinned here is that the callout sits under the
// header and above the arrangement, and that one press lands on the other job.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { killed, running } from "@armada/screens/src/fixtures/build/index";
import { watchedRead } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import type { Watched } from "@armada/protocol";

import { onJob } from "./scenario";
import type { Scenario } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const RESTING = { width: 1440, height: 900 };
afterEach(async () => {
  await page.viewport(RESTING.width, RESTING.height);
});

const REPLACEMENT = {
  id: "01M2C1TJ8G0099REDISPATCHED",
  handle: "124-cache-the-manifest-read",
  title: "Cache the manifest read",
};

/** `running`, moved onto an id of its own — a replacement is a different job. */
function theJobThatTookOver(): JobFixture {
  const fixture = running();
  const job = { ...fixture.job, ...REPLACEMENT, redispatched_from: killed().job.id };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  // The backward read Fleet fills on open: the predecessor's handle, not the
  // ULID on the column. #1474. The read moves onto the new id with the job —
  // `asRow`'s rule — or the detail reads as some other job's.
  const replaces = { job_id: killed().job.id, handle: killed().job.handle };
  const detail = { ...fixture.watched.detail, job, replaces };
  const watched: Watched = { state: "read", jobId: job.id, detail };
  return { ...fixture, job, watched };
}

/** `killed`, naming the job that took the work on. */
function theJobThatWasReplaced(): JobFixture {
  const from = killed();
  if (from.watched.state !== "read") return from;
  const replaced_by = { job_id: REPLACEMENT.id, handle: REPLACEMENT.handle };
  return { ...from, watched: watchedRead({ ...from.watched.detail, replaced_by }) };
}

/** The killed job, its replacement, and the one link between them. */
function aRedispatch(): Scenario {
  return bothJobs(onJob(theJobThatWasReplaced()));
}

/** The same redispatch, landed on from the other end. */
function landedOnTheReplacement(): Scenario {
  return bothJobs(onJob(theJobThatTookOver()));
}

/** Both ends served, whichever of the two the scenario opens on. */
function bothJobs(base: Scenario): Scenario {
  const dead = theJobThatWasReplaced();
  const replacement = theJobThatTookOver();
  return {
    ...base,
    state: { ...base.state, jobs: [dead.job, replacement.job] },
    reads: { ...base.reads, [dead.job.id]: dead, [replacement.job.id]: replacement },
  };
}

test("the callout names the job that took over, under the header and above the run", async () => {
  mount(aRedispatch());
  const callout = page.getByText("This job was redispatched");
  await expect.element(callout).toBeVisible();

  const said = document.querySelector<HTMLElement>(".armada-alert");
  const header = document.querySelector<HTMLElement>(".armada-job-head");
  const inside = document.querySelector<HTMLElement>(".armada-inside");
  if (said === null || header === null || inside === null) {
    throw new Error("the header, the callout and the arrangement are all drawn");
  }
  expect(said.getBoundingClientRect().top).toBeGreaterThan(header.getBoundingClientRect().top);
  expect(said.getBoundingClientRect().top).toBeLessThan(inside.getBoundingClientRect().top);
});

test("one press opens the job that replaced it", async () => {
  mount(aRedispatch());
  await page.getByRole("button", { name: `Open ${REPLACEMENT.handle}` }).click();
  await expect.element(page.getByText(REPLACEMENT.handle, { exact: true }).first()).toBeVisible();
});

// The defect the owner hit: the header said `Redispatched from` and then a
// ULID. #1474.
test("the header names the job this one replaced by its handle, never its id", async () => {
  mount(landedOnTheReplacement());
  const said = page.getByRole("button", { name: killed().job.handle });
  await expect.element(said).toBeVisible();
  expect(page.getByText(killed().job.id, { exact: true }).query()).toBeNull();
});

test("one press on that fact opens the job it replaced", async () => {
  mount(landedOnTheReplacement());
  await page.getByRole("button", { name: killed().job.handle }).click();
  await expect.element(page.getByText("This job was redispatched")).toBeVisible();
});

test("a job killed and left alone says nothing extra", async () => {
  mount(onJob(killed()));
  await expect.element(page.getByText(killed().job.handle, { exact: true }).first()).toBeVisible();
  expect(page.getByText("This job was redispatched").query()).toBeNull();
});
