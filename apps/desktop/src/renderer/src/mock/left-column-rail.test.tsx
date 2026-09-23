// The left column's rail as a person's choice — #1591, through `App`, because
// the claim is about the window's width and what survives a remount, and a
// story sized by a `div` can see neither.
//
// The owner's words, 22 Sep 2026: *"I should be able to collapse it to a rail
// at any point or at the breakpoint."*

import { afterEach, beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp } from "./mount";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** What `left-collapsed.ts` writes. Cleared either side, so no test inherits a choice. */
const KEY = "armada.bridge.left-collapsed";

const RESTING = { width: 1440, height: 900 };
beforeEach(() => window.localStorage.removeItem(KEY));
afterEach(async () => {
  window.localStorage.removeItem(KEY);
  await page.viewport(RESTING.width, RESTING.height);
});

/** `--layout-breakpoint`, read rather than retyped — the one bound that overrides a choice. */
function breakpoint(): number {
  const value = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--layout-breakpoint"));
  if (!Number.isFinite(value)) throw new Error("--layout-breakpoint is not declared");
  return value;
}

const collapse = () => page.getByRole("button", { name: "Collapse the left column" });
const expand = () => page.getByRole("button", { name: "Expand the left column" });
/** A row only the expanded Fleet panel draws, so it answers which width the column is at. */
const panelRows = () => page.getByText("pid");

test("the column collapses to its rail on a press at a wide width, and comes back", async () => {
  await page.viewport(RESTING.width, RESTING.height);
  mount("every-state");
  await expect.element(panelRows()).toBeVisible();

  await collapse().click();
  expect(panelRows().query()).toBeNull();
  // The rail, not a second narrow state: Navigation's own glyphs are still there.
  await expect.element(page.getByRole("button", { name: "Job Board" }).first()).toBeInTheDocument();

  await expand().click();
  await expect.element(panelRows()).toBeVisible();
});

test("⌘\\ does the same thing from the keyboard", async () => {
  await page.viewport(RESTING.width, RESTING.height);
  mount("every-state");
  await expect.element(panelRows()).toBeVisible();

  await userEvent.keyboard("{Meta>}\\{/Meta}");
  await expect.element(expand()).toBeVisible();

  await userEvent.keyboard("{Meta>}\\{/Meta}");
  await expect.element(panelRows()).toBeVisible();
});

test("the choice is remembered, so the window opens at the rail next time", async () => {
  await page.viewport(RESTING.width, RESTING.height);
  // Its own host and its own teardown: this test reopens the window, and
  // `unmountAfterEach` would take the first root down a second time.
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const first = mountApp("every-state", host);
  await collapse().click();
  expect(window.localStorage.getItem(KEY)).toBe("true");

  // A remount is the window reopening: the state is gone and only what was
  // written survives. A collapsed column that forgets itself on every reload
  // is worse than no control at all, which is what the issue says.
  first.unmount();
  host.remove();

  mount("every-state");
  await expect.element(expand()).toBeVisible();
  expect(panelRows().query()).toBeNull();
});

test("below the breakpoint the column is a rail whatever was chosen, and returns to the choice above it", async () => {
  await page.viewport(RESTING.width, RESTING.height);
  mount("every-state");
  await expect.element(panelRows()).toBeVisible();

  await page.viewport(breakpoint() - 100, RESTING.height);
  // No control: there is one width here and no choice to offer, so the press
  // that could not expand is not drawn rather than drawn and inert.
  await expect.poll(() => panelRows().query()).toBeNull();
  expect(collapse().query()).toBeNull();
  expect(expand().query()).toBeNull();
  expect(window.localStorage.getItem(KEY)).toBeNull();

  await page.viewport(RESTING.width, RESTING.height);
  await expect.element(panelRows()).toBeVisible();
});
