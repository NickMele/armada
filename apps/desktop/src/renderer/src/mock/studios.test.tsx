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
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Cancel" })).toHaveFocus();
  await page.getByRole("dialog").getByRole("button", { name: "Delete node" }).click();

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
