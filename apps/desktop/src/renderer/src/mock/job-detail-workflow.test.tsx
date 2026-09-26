// The Workflow tab through `App`: which arrangement it opens in, that the
// toggle is remembered, and what a step opens into. `#1539`.
//
// **Canvas by default, at every width** (owner, 21 and 22 Sep, #1530). The
// revised Narrow board drops the toggle; the toggle stays, so the narrow claim
// below is the one that would have been lost.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { executingSequential } from "@armada/screens/src/fixtures/build/arc";
import { GUIDE_GROUP_EDGES, GUIDE_GROUP_ORDER } from "@armada/components";

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

// # The inspector is a layer over the canvas, and nothing is open until a press
//
// The owner's note of 25 Sep: *this panel should overlay the canvas. By default
// it is not open, so the canvas spans the width of the page* — and it read as
// part of the canvas, because it sat on the canvas's own ground. The claim has
// three halves, and the geometry is the half a story cannot see: the canvas is
// measured either side of the press, so a panel that went back to taking a grid
// track would fail here rather than only look wrong.

/** The canvas's frame, by the box it draws in. */
function canvasBox(): DOMRect {
  const frame = document.querySelectorAll<HTMLElement>(".armada-workflow-tab__canvas");
  const last = frame[frame.length - 1];
  if (last === undefined) throw new Error("the canvas is not drawn");
  return last.getBoundingClientRect();
}

/** The inspector's own box, and `null` where no press has opened it. */
function panelBox(): DOMRect | null {
  const panels = document.querySelectorAll<HTMLElement>(".armada-wf-inspector");
  const last = panels[panels.length - 1];
  return last === undefined ? null : last.getBoundingClientRect();
}

test("nothing is open until a press, and the canvas spans the width of the tab", async () => {
  await page.viewport(2000, 900);
  await workflow();
  // The run is drawn, so the measurement below is of a laid-out canvas.
  await expect.element(card("Implement")).toBeVisible();
  // No panel, and no empty column holding a panel's place either: the reading
  // that used to land on the running step is what would have drawn one.
  expect(await page.getByRole("region", { name: "Implement, step" }).elements()).toHaveLength(0);
  expect(panelBox()).toBeNull();
  // The canvas reaches the trailing edge of the tab it is drawn in.
  const tab = document.querySelectorAll<HTMLElement>(".armada-workflow-tab");
  const frame = tab[tab.length - 1]!.getBoundingClientRect();
  expect(canvasBox().right).toBeCloseTo(frame.right, 0);
});

test("a press opens the panel over the canvas, and the canvas does not reflow", async () => {
  await page.viewport(2000, 900);
  await workflow();
  await expect.element(card("Implement")).toBeVisible();
  const shut = canvasBox();

  await card("Plan the change").click();
  const panel = page.getByRole("region", { name: "Plan the change, step" }).last();
  await expect.element(panel).toBeVisible();

  // **Over it, not beside it.** One element, one window, either side of one
  // press: bit-for-bit, so a tolerance cannot admit the 332px a column would
  // have taken. `job-detail-width.test.tsx` carries the reasoning.
  const open = canvasBox();
  expect(open.width).toBe(shut.width);
  expect(open.left).toBe(shut.left);

  // And the panel is inside the canvas's own frame rather than after it.
  const box = panelBox()!;
  expect(box.left).toBeGreaterThan(open.left);
  expect(box.right).toBeLessThanOrEqual(open.right + 0.5);
});

test("Close takes the panel off, and the canvas is back at full width with nothing open", async () => {
  await page.viewport(2000, 900);
  await workflow();
  await expect.element(card("Implement")).toBeVisible();
  const shut = canvasBox();

  await card("Plan the change").click();
  await expect.element(page.getByRole("region", { name: "Plan the change, step" }).last()).toBeVisible();
  // Closed the way Helm's dock is closed: a Close in the panel's own head.
  await page.getByRole("button", { name: "Close" }).last().click();

  expect(await page.getByRole("region", { name: "Plan the change, step" }).elements()).toHaveLength(0);
  expect(panelBox()).toBeNull();
  expect(canvasBox().width).toBe(shut.width);
});

test("at the owner's own window the panel takes a ceiling, and the canvas is still read beside it", async () => {
  // 1512 × 817 is the window the note was written in, where the panel measured
  // 1347px tall. **The ceiling is the window and the regions scroll inside it**,
  // so the panel is bounded by something other than how much this step has to
  // say. What is above the destination — the title row, the Job's head and the
  // tab strip — is not the panel's to subtract: `--layout` carries no token for
  // it, and the panel is positioned against a destination that scrolls under
  // all three. That was the rule before this change and it is unchanged by it.
  await page.viewport(1512, 817);
  await workflow();
  await card("Implement").click();
  await expect.element(page.getByRole("region", { name: "Implement, step" }).last()).toBeVisible();

  const box = panelBox()!;
  expect(box.height).toBeLessThanOrEqual(window.innerHeight);
  // The regions are what scroll, not the panel: its head stays whatever a
  // person reads, which is the dock's arrangement and the reason for the split.
  const body = document.querySelectorAll<HTMLElement>(".armada-wf-inspector__body");
  const last = body[body.length - 1]!;
  expect(last.scrollHeight).toBeGreaterThan(last.clientHeight);

  // And it is a panel over one side of the canvas, not a second canvas-wide
  // surface: the run stays readable while somebody reads a node.
  const canvas = canvasBox();
  expect(box.width).toBeLessThan(canvas.width / 2);
  expect(box.left).toBeGreaterThan(canvas.left + box.width);
});

test("the panel reads as a layer over the canvas rather than part of it", async () => {
  await page.viewport(2000, 900);
  await workflow();
  await card("Implement").click();
  const panels = document.querySelectorAll<HTMLElement>(".armada-wf-inspector");
  const panel = panels[panels.length - 1]!;
  const drawn = getComputedStyle(panel);
  // The defect in the note's second paragraph: the panel had the canvas's own
  // background and no edge, so it blended in. `armada-glass` is what lifts it —
  // the card treatment's edge and --shadow-card, neither of which the canvas has.
  // The edge is measured against the token that declares it, never a typed `1px`.
  const hairline = getComputedStyle(document.documentElement).getPropertyValue("--border-width").trim();
  expect(hairline).not.toBe("");
  expect(drawn.borderTopWidth).toBe(hairline);
  expect(drawn.boxShadow).not.toBe("none");
  // The glass is a layer on `::before`, so the panel's own background stays off.
  expect(getComputedStyle(panel, "::before").backgroundImage).not.toBe("none");
  // Over the canvas, on the dock's own stacking token.
  const layer = panel.closest<HTMLElement>(".armada-workflow-tab__inspector-layer");
  expect(layer).not.toBeNull();
  expect(getComputedStyle(layer!).position).toBe("absolute");
  expect(getComputedStyle(layer!).zIndex).not.toBe("auto");
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
 * The moment with three groups worked and one not — four groups and eight
 * tasks, which is what the one Plan node below has to summarise without
 * drawing any of them. `arc/group-failed` is the note's own scenario.
 */
async function failed(): Promise<void> {
  mount("arc/group-failed");
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
}

/** An edge, by what it says to somebody who cannot see the line. */
const edge = (says: string) => page.getByLabelText(says).last();

// # The plan is one node, and pressing it is the way to the whole of it
//
// The owner's note of 25 Sep: *instead of the workflow tab showing the entire
// plan, it should just show a single "Plan" node summary. When clicked it takes
// you to the "Plan" tab.* The cost is the second edge, which had nowhere left
// to arrive — stated to him and taken.

test("Workflow draws the steps and one Plan node, and no group or task node", async () => {
  await page.viewport(2000, 900);
  await failed();
  // The plan comes off the step that recorded it, and off nothing else.
  await expect.element(edge("Plan the change made Plan")).toBeInTheDocument();
  const node = page.getByRole("button", { name: /^Plan, / }).last();
  await expect.element(node).toBeVisible();
  // It says what the plan is made of — the design board's own summary.
  await expect.element(node).toHaveTextContent("4 groups");
  await expect.element(node).toHaveTextContent("8 tasks");
  // And nothing of what is inside it: no group node, no task node, no second edge.
  expect(await page.getByRole("button", { name: /^Group \d, / }).elements()).toHaveLength(0);
  expect(await page.getByRole("button", { name: /^Draw what is running, in four lists, / }).elements()).toHaveLength(0);
  expect(await page.getByLabelText(/^Implement worked /).elements()).toHaveLength(0);
});

test("pressing the Plan node lands on the Plan tab", async () => {
  await page.viewport(2000, 900);
  await failed();
  await page.getByRole("button", { name: /^Plan, / }).last().click();
  await expect.element(page.getByRole("tab", { name: /^Plan/, selected: true }).last()).toBeVisible();
  await expect.element(page.getByRole("tabpanel", { name: "Plan" }).last()).toBeVisible();
});

test("Stacked says what the graph says, in words", async () => {
  await failed();
  await page.getByRole("tab", { name: "Stacked" }).last().click();
  // The steps, and the plan as one row under the step that recorded it.
  await expect.element(card("Plan the change")).toBeVisible();
  await expect.element(card("Implement")).toBeVisible();
  await expect.element(card("Plan")).toBeVisible();
  // A column that still listed every group would be the toggle changing subject.
  expect(await page.getByRole("button", { name: /^Group \d, / }).elements()).toHaveLength(0);
  expect(await page.getByText("worked at Implement").elements()).toHaveLength(0);
});

test("a press on a task in the step's board opens that task, and a press on its group takes the panel back", async () => {
  await page.viewport(2000, 900);
  await failed();
  // The board under the run is what a task is opened from now: the canvas
  // above draws steps and the plan, and the board draws what is inside the
  // step that is moving.
  const task = page.getByRole("listitem", { name: "T6 Open a Drone's Job from its row" }).last();
  await task.getByRole("button").first().click();
  await expect.element(page.getByRole("region", { name: "T6 · Open a Drone's Job from its row, task" }).last()).toBeVisible();
  await page.getByRole("button", { name: /^Group 3\b/ }).last().click();
  await expect.element(page.getByRole("region", { name: "Group 3, group" }).last()).toBeVisible();
});

// # What this destination no longer says, and the two marks that carry it
//
// Two standing sentences: the implement board's order line, and — never on the
// screen, because the mark was owed since #1607 — how a group gets its second
// edge. Both are true of a Job that had never run, which is #1602's test.

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

test("the canvas head carries the mark for how a group is drawn, and Stacked does not", async () => {
  await workflow();
  await expect.element(markFor(GUIDE_GROUP_EDGES).last()).toBeVisible();

  // Off on Stacked. A column draws no edges, so a mark about reading them
  // would explain a picture that is not on screen — and mounting it is what
  // would spend a person's one first contact on it.
  await page.getByRole("tab", { name: "Stacked" }).last().click();
  await expect.element(page.getByRole("list", { name: /as its workflow's run$/ }).last()).toBeVisible();
  expect(markFor(GUIDE_GROUP_EDGES).elements()).toHaveLength(0);
});
