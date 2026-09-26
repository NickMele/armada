// The Workflow tab through `App`: which arrangement it opens in, that the
// toggle is remembered, and what a step opens into. `#1539`.
//
// **Canvas by default, at every width** (owner, 21 and 22 Sep, #1530). The
// revised Narrow board drops the toggle; the toggle stays, so the narrow claim
// below is the one that would have been lost.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { executingSequential } from "@armada/screens/src/fixtures/build/arc";
import { GUIDE_GROUP_ORDER, GUIDES, RETIRED_GUIDE_NUMBERS } from "@armada/components";

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
  const run = page.getByRole("list", { name: /as its workflow's run$/ }).last();
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

/**
 * The moment with three groups worked and one not, so the claim below has both
 * halves in one picture. `arc/group-failed` is the note's own scenario.
 */
async function failed(): Promise<void> {
  mount("arc/group-failed");
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
}

/** An edge, by what it says to somebody who cannot see the line. */
const edge = (says: string) => page.getByLabelText(says).last();

test("a group comes off the step that wrote it, and a worked one off the step that worked it", async () => {
  await page.viewport(2000, 900);
  await failed();
  // One node per group, with the plan step's edge into each of the four.
  for (const ordinal of [1, 2, 3, 4]) {
    await expect.element(edge(`Plan the change made Group ${ordinal}`)).toBeInTheDocument();
  }
  // Three have been worked and carry the second edge into that same node.
  for (const ordinal of [1, 2, 3]) {
    await expect.element(edge(`Implement worked Group ${ordinal}`)).toBeInTheDocument();
  }
  // The fourth has not, so it has one edge and no other.
  expect(await page.getByLabelText("Implement worked Group 4").elements()).toHaveLength(0);
  // A task hangs off its own group, never off the step.
  await expect.element(edge("Group 3 holds Draw what is running, in four lists")).toBeInTheDocument();
  expect(await page.getByLabelText(/^Implement holds /).elements()).toHaveLength(0);
});

test("Stacked says what the graph says, in words", async () => {
  await failed();
  await page.getByRole("tab", { name: "Stacked" }).last().click();
  // Every group and every task is a row, under the step that wrote them.
  for (const ordinal of [1, 2, 3, 4]) {
    await expect.element(card(`Group ${ordinal}`)).toBeVisible();
  }
  await expect.element(card("Draw what is running, in four lists")).toBeVisible();
  // A column has no second edge to draw, so a worked group says which step
  // worked it — three of the four, and the pending one says nothing.
  expect(await page.getByText("worked at Implement").elements()).toHaveLength(3);
});

test("a press on a task opens that task, and a press on its group takes the panel back", async () => {
  await page.viewport(2000, 900);
  await failed();
  await card("Open a Drone's Job from its row").click();
  await expect.element(page.getByRole("region", { name: "T6 · Open a Drone's Job from its row, task" }).last()).toBeVisible();
  await card("Group 3").click();
  await expect.element(page.getByRole("region", { name: "Group 3, group" }).last()).toBeVisible();
});

// # What this destination no longer says, and the mark that carries it
//
// The implement board's order line stood over the groups and is guide 4 now.
// It is true of a Job that had never run, which is #1602's test.

/** One guide's `?`, by the name `GuideMark` gives it. */
const markFor = (guide: { number: number; title: string }) =>
  page.getByRole("button", { name: `Open guide ${guide.number}, ${guide.title}` });

test("the implement board heads its groups with the noun, and the mark hangs on that", async () => {
  await workflow();
  // The head first: an assertion that something is absent passes against a
  // window that has not drawn yet, so a negative stands behind a positive.
  const head = page.getByRole("heading", { name: "Groups", exact: true }).last();
  await expect.element(head).toBeVisible();
  // The noun and nothing else. *The groups, in the order they run* would be
  // the sentence that came off wearing a hat.
  await expect.element(head).toHaveTextContent(/^Groups$/);
  await expect.element(markFor(GUIDE_GROUP_ORDER).last()).toBeVisible();
  // *One group at a time. No task of the next group starts…* stood over the
  // groups and was the only evidence of the rule. It is guide 4 now.
  await expect
    .element(page.getByText(/No task of the next group starts while this one is being checked/))
    .not.toBeInTheDocument();
  // The board still reports: the groups are there, in the order they run.
  await expect
    .element(page.getByRole("list", { name: "The groups of this step, in the order they run" }).last())
    .toBeVisible();
});

test("the canvas head carries no mark, because the guide that hung there is retired", async () => {
  await workflow();
  // Guide 11 explained how a group gets its second edge. The plan's graph
  // moved to the Plan tab on 25 September 2026, so it was retired rather than
  // rewritten, and 11 is a number nothing may take again.
  expect(RETIRED_GUIDE_NUMBERS).toContain(11);
  expect(GUIDES.map((guide) => guide.number)).not.toContain(11);
  expect(page.getByRole("button", { name: /^Open guide 11,/ }).elements()).toHaveLength(0);

  await page.getByRole("tab", { name: "Stacked" }).last().click();
  await expect.element(page.getByRole("list", { name: /as its workflow's run$/ }).last()).toBeVisible();
});
