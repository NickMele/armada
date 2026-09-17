// The Studios surface, through `App`, on a Fleet that keeps Studios — #1287's definition of done,
// and #1341's: every scenario keeps Studios, so the surface opens wherever it is reached.
//
// #1287's is:
// open Studios from the rail, start one, drag a node, close Bridge, reopen it read-only with the
// node where it was left, press Continue, and accept a proposed relation, with Helm's footer naming
// the Studio. `crates/acceptance/tests/studio.rs` lists the read-only reopen as proved here.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { mountApp, type Mounted } from "./mount";
import { studying } from "./studio-fleet";
import { entered } from "./testing";

const windows: { app: Mounted; host: HTMLElement }[] = [];

/** Open a Bridge window on `fleet`, sharing its `window.armada` where one was open before. */
function open(scenario: Parameters<typeof mountApp>[0], shared?: Mounted["api"]): Mounted {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const app = mountApp(scenario, host, shared);
  windows.push({ app, host });
  return app;
}

/** Close the window. Fleet, and every Studio it keeps, outlives it. */
function close(): void {
  for (const one of windows.splice(0)) {
    one.app.unmount();
    one.host.remove();
  }
}

afterEach(close);

const node = (name: RegExp) => page.getByRole("group", { name });
/** The acts on what is selected, and the dialog whichever one opened. */
const acts = () => page.getByRole("group", { name: "Acts on what is selected" });
const asked = (name: string) => page.getByRole("dialog").getByRole("button", { name, exact: true });
const centre = (element: Element) => {
  const box = element.getBoundingClientRect();
  return { x: box.left + box.width / 2, y: box.top + box.height / 2 };
};

/** A pointer drag, the way React Flow hears one: down on the node, moves and up on the window. */
function drag(element: Element, dx: number, dy: number): void {
  const from = centre(element);
  const at = (x: number, y: number) => ({ clientX: from.x + x, clientY: from.y + y, button: 0, bubbles: true, view: window });
  element.dispatchEvent(new MouseEvent("mousedown", at(0, 0)));
  for (const step of [0.1, 0.5, 1]) window.dispatchEvent(new MouseEvent("mousemove", at(dx * step, dy * step)));
  window.dispatchEvent(new MouseEvent("mouseup", at(dx, dy)));
}

test("a Studio started, laid out, closed, reopened read-only, continued, and a relation accepted", async () => {
  const fleet = studying([]);
  const first = open(fleet.scenario);

  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await expect.element(page.getByText("No Studios yet.", { exact: false })).toBeVisible();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();
  // Started, so the person's own: no Continue to press.
  expect(page.getByRole("button", { name: "Continue" }).query()).toBeNull();

  const [studio] = fleet.studios();
  fleet.helmProposes(studio!.id);
  await expect.element(node(/^Note: Drag me somewhere/)).toBeVisible();
  const before = fleet.studios()[0]!.nodes.find((one) => one.kind === "note")!.position;

  drag(node(/^Note: Drag me somewhere/).element(), 160, 120);
  await expect.poll(() => fleet.studios()[0]!.nodes.find((one) => one.kind === "note")!.position.x).toBeGreaterThan(before.x + 60);
  const left = fleet.studios()[0]!.nodes.find((one) => one.kind === "note")!.position;
  expect(left.y).toBeGreaterThan(before.y + 30);

  close();
  open(fleet.scenario, first.api);

  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "Untitled Studio" }).click();
  await expect.element(page.getByText("Read-only", { exact: true })).toBeVisible();
  await expect.element(node(/^Note: Drag me somewhere/)).toBeVisible();
  expect(fleet.studios()[0]!.nodes.find((one) => one.kind === "note")!.position).toEqual(left);

  // Read-only means read-only: a drag saves nothing, and no act on a relation is offered.
  drag(node(/^Note: Drag me somewhere/).element(), 200, 0);
  await new Promise((resolve) => setTimeout(resolve, 200));
  expect(fleet.studios()[0]!.nodes.find((one) => one.kind === "note")!.position).toEqual(left);
  expect(page.getByRole("button", { name: /^Accept: / }).query()).toBeNull();

  // Helm's footer names the Studio, and the node selected on it.
  await expect.element(page.getByText("Studios · Untitled Studio", { exact: true })).toBeVisible();
  node(/^Note: Drag me somewhere/).element().focus();
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByText("Studios · Untitled Studio · Note Drag me somewhere selected")).toBeVisible();

  await page.getByRole("button", { name: "Continue" }).click();
  expect(page.getByText("Read-only", { exact: true }).query()).toBeNull();
  await page.getByRole("button", { name: /^Accept: Finding What does the note point at\? answers Note Drag me somewhere/ }).click();
  await expect.poll(() => fleet.studios()[0]!.edges.map((edge) => edge.standing)).toEqual(["accepted"]);
  await expect.poll(() => page.getByRole("button", { name: /^Accept: / }).query()).toBeNull();
});

test("Delete node confirms, with Cancel first, and takes the node's edges with it", async () => {
  const fleet = studying([]);
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();
  fleet.helmProposes(fleet.studios()[0]!.id);
  await expect.element(node(/^Finding: /)).toBeVisible();

  node(/^Finding: /).element().focus();
  await userEvent.keyboard("{Enter}");
  await page.getByRole("button", { name: "Delete node" }).click();
  // Scaling up, so the press inside it waits for it to land — #1323.
  const confirm = page.getByRole("dialog");
  await entered(confirm);
  await expect.element(page.getByRole("button", { name: "Cancel" })).toHaveFocus();
  await confirm.getByRole("button", { name: "Delete node" }).click();

  await expect.poll(() => fleet.studios()[0]!.nodes.map((one) => one.kind)).toEqual(["note"]);
  expect(fleet.studios()[0]!.edges).toEqual([]);
});

test("on All repositories the surface asks for one, since a Studio belongs to one", async () => {
  const fleet = studying();
  open({ ...fleet.scenario, state: { ...fleet.scenario.state, repository: null } });
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await expect.element(page.getByText("Pick a repository to open its Studios")).toBeVisible();
});

/** Studios from the rail, answering the surface's own ask for a repository. */
async function openStudios(): Promise<void> {
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await userEvent.selectOptions(page.getByLabelText("Repository", { exact: true }), repository().root);
}

const FAILED = "This repository's Studios could not be read";

test("every-state keeps Studios, so the surface opens on a list and not a read failure", async () => {
  open("every-state");
  await openStudios();

  await expect.element(page.getByRole("button", { name: "Every kind of node and edge" })).toBeVisible();
  // Untitled, so the list draws what an unnamed Studio is called.
  await expect.element(page.getByRole("button", { name: "Untitled Studio" })).toBeVisible();
  expect(page.getByText(FAILED).query()).toBeNull();

  await page.getByRole("button", { name: "Every kind of node and edge" }).click();
  await expect.element(node(/^Note: The legend under the step bar is unreadable/)).toBeVisible();
  // A Job node reads its state off the Board row this window already holds.
  await expect.element(node(/^Job: /)).toBeVisible();
  // Reopened read-only, so its proposed relations are listed and nothing acts on them.
  await expect.element(page.getByText("Continue to accept or reject.")).toBeVisible();
});

test("a scenario keeping no Studios draws the empty state, not a read failure", async () => {
  open("empty-store");
  await openStudios();

  await expect.element(page.getByText("No Studios yet.", { exact: false })).toBeVisible();
  expect(page.getByText(FAILED).query()).toBeNull();
});

/**
 * #1352's own: the picture a Note kept is drawn on the node, and opened full
 * size beside it. What this holds is the whole path — Fleet's bytes, over the
 * preload, into a `blob:` this window made and an `img` drew — since every
 * piece of it is fine on its own and the frame was invisible all the same.
 */
test("a Note draws the frame it kept, opens it full size, and a Note without one draws no plate", async () => {
  // The `studios` scenario has this repository picked already, and keeps the legend Studio.
  open(studying().scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "The Board's legend" }).click();

  const kept = node(/^Note: The legend under the step bar is unreadable/);
  await expect.element(kept).toBeVisible();
  // One picture on the board: the second Note kept none, and draws no box at all.
  const drawn = kept.getByRole("img", { name: /captured from/ });
  await expect.element(drawn).toBeVisible();
  await expect.poll(() => drawn.element().getAttribute("src")).toMatch(/^blob:/);
  expect(node(/^Note: It wraps at 720 wide/).getByRole("img").query()).toBeNull();

  // Opened from the node's own acts, and read-only is no reason not to look.
  kept.element().focus();
  await userEvent.keyboard("{Enter}");
  await page.getByRole("button", { name: "Open frame" }).click();
  const sheet = page.getByRole("dialog", { name: "Note" });
  await expect.element(sheet).toBeVisible();
  await expect.element(sheet.getByText("The legend under the step bar is unreadable")).toBeVisible();
  await expect.element(sheet.getByRole("img", { name: /captured from/ })).toBeVisible();

  // A Note that kept none is offered nothing to open.
  await userEvent.keyboard("{Escape}");
  node(/^Note: It wraps at 720 wide/).element().focus();
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => page.getByRole("button", { name: "Open frame" }).query()).toBeNull();
});

test("two Notes clustered, the Cluster written up, the draft edited and dispatched to a Job node", async () => {
  const fleet = studying([]);
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();
  fleet.twoNotes(fleet.studios()[0]!.id);
  await expect.element(node(/^Note: The chip keeps its count/)).toBeVisible();

  // Picked together: the second joins the first rather than replacing it.
  await node(/^Note: The chip keeps its count/).click();
  await userEvent.keyboard("{Meta>}");
  await node(/^Note: Overview still says three/).click();
  await userEvent.keyboard("{/Meta}");
  await expect.element(acts().getByRole("button", { name: "Cluster Notes" })).toBeVisible();

  await acts().getByRole("button", { name: "Cluster Notes" }).click();
  await page.getByRole("textbox", { name: "Title" }).fill("Counts go stale");
  await asked("Cluster").click();
  await expect.element(node(/^Cluster: Counts go stale/)).toBeVisible();
  // Every Note it was made of keeps an edge to it, so the Cluster says where it came from.
  await expect.poll(() => fleet.studios()[0]!.edges.filter((edge) => edge.kind === "produced").length).toBe(2);

  await node(/^Cluster: Counts go stale/).click();
  await acts().getByRole("button", { name: "Write up" }).click();
  // The write-up opens on the Notes' own words rather than on an empty field.
  await expect
    .poll(() => (page.getByRole("textbox", { name: "Body" }).element() as HTMLTextAreaElement).value)
    .toContain("The chip keeps its count");
  await page.getByRole("textbox", { name: "Title" }).fill("Counts go stale after what they count changes");
  await asked("Write up").click();
  await expect.element(node(/^Issue draft: Counts go stale after what they count changes/)).toBeVisible();

  // Edited before it is sent: what is dispatched is what the person left.
  await node(/^Issue draft: Counts go stale after what they count changes/).click();
  await acts().getByRole("button", { name: "Edit draft" }).click();
  await page.getByRole("textbox", { name: "Body" }).fill("Both counts are read off a row that is stale.");
  await asked("Save draft").click();
  await expect
    .poll(() => fleet.studios()[0]!.nodes.find((one) => one.kind === "issue_draft"))
    .toMatchObject({ body: "Both counts are read off a row that is stale." });

  await node(/^Issue draft: Counts go stale after what they count changes/).click();
  await acts().getByRole("button", { name: "Dispatch" }).click();
  // What is sent is the draft's own text, title first, and a person reads it before pressing.
  await expect
    .poll(() => (page.getByRole("textbox", { name: "What is sent" }).element() as HTMLTextAreaElement).value)
    .toBe("Counts go stale after what they count changes\n\nBoth counts are read off a row that is stale.");
  await asked("Dispatch").click();

  await expect.poll(() => fleet.studios()[0]!.nodes.filter((one) => one.kind === "job").length).toBe(1);
  const studio = fleet.studios()[0]!;
  const job = studio.nodes.find((one) => one.kind === "job")!;
  const draft = studio.nodes.find((one) => one.kind === "issue_draft")!;
  expect(studio.edges.find((edge) => edge.to === job.id)).toMatchObject({
    from: draft.id,
    kind: "produced",
  });
});

test("a Contradiction is ended as Resolved here, with the answer kept on the node", async () => {
  const fleet = studying([]);
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();
  fleet.aContradiction(fleet.studios()[0]!.id);
  await expect.element(node(/^Contradiction: The chip reads the Board/)).toBeVisible();

  await node(/^Contradiction: The chip reads the Board/).click();
  // All four outcomes are offered on the node, and two of them are the rungs beside them.
  for (const offered of ["Write up", "Defer", "Not a problem", "Resolved here"]) {
    await expect.element(acts().getByRole("button", { name: offered, exact: true })).toBeVisible();
  }
  await acts().getByRole("button", { name: "Resolved here", exact: true }).click();
  await page.getByRole("textbox", { name: "Answer" }).fill("The Board wins; the row is stale");
  await asked("Resolve").click();

  await expect
    .poll(() => fleet.studios()[0]!.nodes[0])
    .toMatchObject({
      state: "resolved_here",
      answer: "The Board wins; the row is stale",
    });
  // Ended once: nothing offers a second outcome on it.
  await node(/^Contradiction: The chip reads the Board/).click();
  expect(acts().getByRole("button", { name: "Not a problem", exact: true }).query()).toBeNull();
});
