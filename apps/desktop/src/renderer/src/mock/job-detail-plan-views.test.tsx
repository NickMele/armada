// The Plan tab's two views through `App`: which one it opens in, that the
// toggle moves between them, and that the choice is this viewer's.
//
// The owner's second note of 25 Sep 2026: *it would be easier to see this if
// the plan had its own different views — Graph, List (this view), some kind of
// diagram that represents the repo.* Graph and List are built; the diagram is
// not designed, so there is no third tab and nothing standing in for one.

import { beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

// The arrangement is remembered per viewer, so one test's press would otherwise
// be the next test's default.
beforeEach(() => window.localStorage.removeItem("armada.bridge.plan-view"));

/** The arc's Job mid-implement, with its Plan tab open. The owner's own scenario. */
async function plan(): Promise<void> {
  mount("arc/executing-concurrent");
  await page.getByRole("tab", { name: /^Plan/ }).last().click();
}

/** The graph's own group card. Not a control: there is no group destination to press into. */
const groupNode = (ordinal: number) => page.getByRole("group", { name: new RegExp(`^Group ${ordinal}, `) }).last();

/** The list's own group heading, which the graph draws no copy of. */
const listGroups = () => page.getByRole("list", { name: "Groups, in the order they run" }).last();

test("the Plan tab opens on the graph, with a node per group and its tasks beside it", async () => {
  await page.viewport(2000, 900);
  await plan();
  await expect.element(page.getByRole("tab", { name: "Graph", selected: true }).last()).toBeVisible();
  await expect.element(groupNode(1)).toBeVisible();
  // A task hangs off its own group, and the edge says so to a reader who
  // cannot see the line.
  await expect.element(page.getByLabelText(/^Group 1 holds /).last()).toBeInTheDocument();
  // The list is the other view, not a second copy under this one.
  expect(await listGroups().elements()).toHaveLength(0);
});

test("the toggle moves to the list and back, and neither view draws the other", async () => {
  await page.viewport(2000, 900);
  await plan();
  await expect.element(groupNode(1)).toBeVisible();

  await page.getByRole("tab", { name: "List" }).last().click();
  await expect.element(listGroups()).toBeVisible();
  expect(await page.getByRole("group", { name: /^Group 1, / }).elements()).toHaveLength(0);

  await page.getByRole("tab", { name: "Graph" }).last().click();
  await expect.element(groupNode(1)).toBeVisible();
  expect(await listGroups().elements()).toHaveLength(0);
});

test("the choice is this viewer's, and survives a remount", async () => {
  await page.viewport(2000, 900);
  await plan();
  await page.getByRole("tab", { name: "List" }).last().click();
  await expect.element(listGroups()).toBeVisible();

  // A second window, as a relaunch is: the arrangement is this viewer's and
  // not this Job's, so it is what opens.
  await plan();
  await expect.element(page.getByRole("tab", { name: "List", selected: true }).first()).toBeVisible();
  await expect.element(listGroups().first()).toBeVisible();
});

test("a task on the graph opens the sheet the list's own row opens", async () => {
  await page.viewport(2000, 900);
  await plan();
  await expect.element(groupNode(1)).toBeVisible();
  // One press for one task, whichever view a person is reading in.
  await page.getByRole("button", { name: /^Serve one read of everything running, / }).last().click();
  const sheet = page.getByRole("dialog", { name: /Serve one read of everything running/ }).last();
  await expect.element(sheet).toBeVisible();
});
