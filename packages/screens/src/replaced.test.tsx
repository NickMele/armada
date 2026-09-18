// The callout on a job a redispatch replaced. #1439.
//
// **What is pinned is the dead end closing**: the name of the job that took
// over is on the control, so reading it and reaching it are one thing. The
// absences matter as much — a job nothing replaced draws nothing, and a surface
// with nowhere to navigate offers no press rather than a dead one.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { replacedCallout } from "./replaced";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const REPLACEMENT = { job_id: "01M2C1TJ8G0016YK5S0JXBXDQ5", handle: "124-cache-the-manifest-read" };

test("a job nothing replaced draws nothing", () => {
  expect(replacedCallout(undefined, () => {})).toBeUndefined();
});

test("the callout says the job was redispatched and names the job that took over", async () => {
  mount(<>{replacedCallout(REPLACEMENT, () => {})}</>);
  await expect.element(page.getByText("This job was redispatched")).toBeVisible();
  await expect
    .element(page.getByRole("button", { name: "Open 124-cache-the-manifest-read" }))
    .toBeVisible();
});

test("the press opens the job that replaced it, by id", async () => {
  const open = vi.fn();
  mount(<>{replacedCallout(REPLACEMENT, open)}</>);
  await page.getByRole("button", { name: "Open 124-cache-the-manifest-read" }).click();
  expect(open).toHaveBeenCalledWith(REPLACEMENT.job_id);
});

test("with nowhere to navigate the callout still says so, and offers no press", async () => {
  mount(<>{replacedCallout(REPLACEMENT, undefined)}</>);
  await expect.element(page.getByText("This job was redispatched")).toBeVisible();
  expect(page.getByRole("button").query()).toBeNull();
});
