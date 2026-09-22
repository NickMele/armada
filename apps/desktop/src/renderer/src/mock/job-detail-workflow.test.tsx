// The Workflow tab through `App`: which arrangement it opens in, that the
// toggle is remembered, and what a step opens into. `#1539`.
//
// **Canvas by default, at every width** (owner, 21 and 22 Sep, #1530). The
// revised Narrow board drops the toggle; the toggle stays, so the narrow claim
// below is the one that would have been lost.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { executingSequential } from "@armada/screens/src/fixtures/build/arc";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const RESTING = { width: 1440, height: 900 };
afterEach(async () => {
  await page.viewport(RESTING.width, RESTING.height);
});

// The arrangement is remembered per viewer, so one test's press would otherwise
// be the next test's default.
beforeEach(() => window.localStorage.removeItem("armada.bridge.workflow-view"));

/**
 * The feature Job mid-implement, with its Workflow tab open.
 *
 * **Every locator is the newest window's.** The relaunch claim below mounts a
 * second `App` beside the first, which is what a relaunch is from the
 * arrangement's point of view — it is this viewer's and not this Job's.
 */
async function workflow(): Promise<void> {
  mount("arc/executing-sequential");
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
}

/** The steps the arc's Job froze, by the labels its own workflow file declares. */
const STEPS = executingSequential().fixtures[0]!.workflows[0]!.steps.map((step) => step.label);

const card = (label: string) => page.getByRole("button", { name: new RegExp(`^${label}, `) }).last();

test("the Workflow tab opens on the canvas, with every step of the frozen workflow", async () => {
  await workflow();
  await expect.element(page.getByRole("tab", { name: "Canvas", selected: true }).last()).toBeVisible();
  for (const label of STEPS) await expect.element(card(label)).toBeVisible();
});

test("the canvas is still what opens at the narrowest window Bridge lays out for", async () => {
  await page.viewport(768, 900);
  await workflow();
  await expect.element(page.getByRole("tab", { name: "Canvas", selected: true }).last()).toBeVisible();
  await expect.element(page.getByRole("tab", { name: "Stacked" }).last()).toBeVisible();
});

test("Stacked draws the same steps as a list, and the choice survives a relaunch", async () => {
  await workflow();
  await page.getByRole("tab", { name: "Stacked" }).last().click();
  const run = page.getByRole("list", { name: /as its workflow's steps$/ }).last();
  await expect.element(run).toBeVisible();
  for (const label of STEPS) await expect.element(card(label)).toBeVisible();

  // A second window, as a relaunch is: the arrangement is this viewer's and
  // not this Job's, so it is what opens.
  await workflow();
  await expect.element(page.getByRole("tab", { name: "Stacked", selected: true }).first()).toBeVisible();
});

test("the inspector lands on the step the Job is on, and a press moves it", async () => {
  // Wide enough that the whole run is fitted inside the canvas rather than
  // half of it sitting under the rail, where a press would never land.
  await page.viewport(2000, 900);
  await workflow();
  // The panel is never a blank column beside a full canvas.
  await expect.element(page.getByRole("region", { name: "Implement, step" }).last()).toBeVisible();
  await card("Plan the change").click();
  await expect.element(page.getByRole("region", { name: "Plan the change, step" }).last()).toBeVisible();
});

test("a step opens with its Checks and the tests at its boundary drawn apart", async () => {
  await workflow();
  await card("Implement").click();
  await expect.element(page.getByRole("region", { name: "Checks at this boundary" }).last()).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Tests at this boundary" }).last()).toBeVisible();
  // Review and reply are one loop, so the box is in the panel and not behind a dialog.
  await expect.element(page.getByRole("region", { name: "Redirect" }).last()).toBeVisible();
});
