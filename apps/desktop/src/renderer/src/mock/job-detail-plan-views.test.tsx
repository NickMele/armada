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

/** One window, by the host `mount` appended for it. */
type Window = ReturnType<typeof page.elementLocator>;

/**
 * The arc's Job mid-implement, with its Plan tab open — the owner's own
 * scenario.
 *
 * **It answers with that window's own locator**, because the relaunch claim
 * below mounts a second `App` beside the first and every window on the page
 * draws a tab strip. `.last()` is not enough there: the first window is still
 * on whatever was pressed in it, so a claim that matched it would pass against
 * a second window that read nothing back at all.
 */
async function plan(): Promise<Window> {
  mount("arc/executing-concurrent");
  const hosts = document.querySelectorAll<HTMLElement>("#root");
  const within = page.elementLocator(hosts[hosts.length - 1]!);
  await within.getByRole("tab", { name: /^Plan/ }).click();
  return within;
}

/** The graph's own group card. Not a control: there is no group destination to press into. */
const groupNode = (within: Window, ordinal: number) =>
  within.getByRole("group", { name: new RegExp(`^Group ${ordinal}, `) });

/** The list's own group list, which the graph draws no copy of. */
const listGroups = (within: Window) => within.getByRole("list", { name: "Groups, in the order they run" });

test("the Plan tab opens on the graph, with a node per group and its tasks beside it", async () => {
  await page.viewport(2000, 900);
  const within = await plan();
  await expect.element(within.getByRole("tab", { name: "Graph", selected: true })).toBeVisible();
  await expect.element(groupNode(within, 1)).toBeVisible();
  // A task hangs off its own group, and the edge says so to a reader who
  // cannot see the line. Named in full: group one holds two tasks, and a
  // prefix would match both.
  await expect
    .element(within.getByLabelText("Group 1 holds Serve one read of everything running"))
    .toBeInTheDocument();
  // The list is the other view, not a second copy under this one.
  expect(await listGroups(within).elements()).toHaveLength(0);
});

test("the toggle moves to the list and back, and neither view draws the other", async () => {
  await page.viewport(2000, 900);
  const within = await plan();
  await expect.element(groupNode(within, 1)).toBeVisible();

  await within.getByRole("tab", { name: "List" }).click();
  await expect.element(listGroups(within)).toBeVisible();
  expect(await groupNode(within, 1).elements()).toHaveLength(0);

  await within.getByRole("tab", { name: "Graph" }).click();
  await expect.element(groupNode(within, 1)).toBeVisible();
  expect(await listGroups(within).elements()).toHaveLength(0);
});

test("the choice is this viewer's, and survives a remount", async () => {
  await page.viewport(2000, 900);
  const first = await plan();
  await first.getByRole("tab", { name: "List" }).click();
  await expect.element(listGroups(first)).toBeVisible();

  // A second window, as a relaunch is: the arrangement is this viewer's and
  // not this Job's, so it is what opens — scoped to that window and to no
  // other, which is what makes this a claim about what was read back.
  const second = await plan();
  await expect.element(second.getByRole("tab", { name: "List", selected: true })).toBeVisible();
  await expect.element(listGroups(second)).toBeVisible();
  expect(await groupNode(second, 1).elements()).toHaveLength(0);
});

test("a task on the graph opens the sheet the list's own row opens", async () => {
  await page.viewport(2000, 900);
  const within = await plan();
  await expect.element(groupNode(within, 1)).toBeVisible();
  // One press for one task, whichever view a person is reading in.
  await within.getByRole("button", { name: /^Serve one read of everything running, / }).click();
  await expect
    .element(page.getByRole("dialog", { name: /Serve one read of everything running/ }).last())
    .toBeVisible();
});
