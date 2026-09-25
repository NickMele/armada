// The guidance system through `App` — #1602, #1603. Three of its claims are
// about the window rather than about a component, so nothing in Storybook can
// make them: what happens the first time a person meets a piece, what survives
// the window closing, and whether the switch is findable afterwards.
//
// The owner, 23 September 2026: *"Any hints or guides should be something I
// choose to see."* The one card that arrives unasked is paid for by the switch
// arriving with it.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { running } from "@armada/screens/src/fixtures/build/index";
import { GUIDES, GUIDE_STEP_BAR } from "@armada/components";

import { onJob } from "./scenario";
import { mountApp } from "./mount";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** What `guidance.tsx` writes. Cleared either side, so no test inherits a memory. */
const KEY = "armada.bridge.guides";

/** What `guide-list-width.ts` writes — the catalogue list's own remembered width. */
const WIDTH_KEY = "armada.bridge.guide-list-width";

function forget(): void {
  window.localStorage.removeItem(KEY);
  window.localStorage.removeItem(WIDTH_KEY);
}

beforeEach(forget);
afterEach(forget);

/** The card for one guide, wherever the provider drew it. */
const cardFor = (guide: { number: number; title: string }) =>
  page.getByRole("dialog", { name: `Guide ${guide.number}, ${guide.title}` });

/** The run's own band carries the first mark a person meets on a Job. */
const runMark = () =>
  page.getByRole("button", { name: `Open guide ${GUIDE_STEP_BAR.number}, ${GUIDE_STEP_BAR.title}` });

const card = () => cardFor(GUIDE_STEP_BAR);
const closeCard = () => card().getByRole("button", { name: "Close" }).click();

test("the first piece a person meets opens its card by itself, and the card carries the switch", async () => {
  mount(onJob(running()));
  await expect.element(card()).toBeVisible();
  await expect
    .element(card().getByRole("switch", { name: /Open a guide the first time/ }))
    .toBeChecked();
});

test("a piece opens itself once and never again, across the window reopening", async () => {
  // Its own host and its own teardown: this test reopens the window, and
  // `unmountAfterEach` would take the first root down a second time.
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const first = mountApp(onJob(running()), host);
  await expect.element(card()).toBeVisible();
  await closeCard();
  first.unmount();
  host.remove();

  // The memory is all that survives a reopen, and a card that came back would
  // be the uninvited thing arriving twice.
  mount(onJob(running()));
  await expect.element(runMark()).toBeVisible();
  expect(card().query()).toBeNull();
});

test("the mark still opens the card after the piece has been met", async () => {
  mount(onJob(running()));
  await closeCard();

  await runMark().click();
  await expect.element(card()).toBeVisible();
  // No switch: the offer was made once, and Settings is where it lives now.
  expect(card().getByRole("switch").query()).toBeNull();
});

test("the switch on the first card turns all of them off, and Settings is where it is found", async () => {
  mount(onJob(running()));
  // The label, not the input: the switch's own text sits over its checkbox,
  // and a label click is what a person's press is anyway.
  await card().getByText("Open a guide the first time I meet a piece").click();
  await closeCard();

  await page.getByRole("button", { name: "Settings", exact: true }).first().click();
  const setting = page.getByRole("switch", { name: /Open a guide the first time/ });
  await expect.element(setting).toBeVisible();
  await expect.element(setting).not.toBeChecked();
});

test("the rail reaches the catalogue, every guide is in the list, and one is open", async () => {
  mount(onJob(running()));
  await closeCard();

  await page.getByRole("button", { name: "Guides", exact: true }).first().click();

  // The catalogue draws a row per guide and opens one beside them, so a guide
  // is reachable rather than already on screen — the list is the claim.
  for (const guide of GUIDES) {
    await expect.element(page.getByRole("button", { name: guide.title })).toBeVisible();
  }

  // Arriving on an empty panel is the thing the list-and-panel arrangement
  // must never do, so the first guide is open before anything is pressed.
  const [first] = GUIDES;
  if (first === undefined) throw new Error("no guides to draw");
  await expect.element(page.getByRole("heading", { name: first.title })).toBeVisible();

  const last = GUIDES[GUIDES.length - 1];
  if (last === undefined) throw new Error("no guides to draw");
  await page.getByRole("button", { name: last.title }).first().click();
  await expect.element(page.getByRole("heading", { name: last.title })).toBeVisible();
});

/**
 * The owner, 25 September 2026: *"Increase the default width of this panel by
 * 10%. Also it would be nice if it was resizable."* The width the list rests
 * at is a token, so it is read off `--w-guide-list` rather than typed here — a
 * test restating the number would pass against its own copy of it.
 */
function resting(): number {
  const value = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--w-guide-list"));
  if (!Number.isFinite(value)) throw new Error("--w-guide-list is not declared");
  return value;
}

const handle = () => page.getByRole("separator", { name: "Resize the list of guides" });

/** The column a drag moves, which is what a reader watching this would see change. */
const listWidth = () =>
  Math.round(document.querySelector(".armada-guides__list")?.getBoundingClientRect().width ?? 0);

/**
 * The catalogue, from a window that has just opened. The uninvited card is
 * over the rail the first time the piece is met and never again, so `met` says
 * whether there is one to close on the way.
 */
const openCatalogue = async (met = false) => {
  if (!met) await closeCard();
  await page.getByRole("button", { name: "Guides", exact: true }).first().click();
  await expect.element(handle()).toBeVisible();
};

test("the list rests at its token's width, and its inner edge moves it", async () => {
  mount(onJob(running()));
  await openCatalogue();

  await expect.element(handle()).toHaveAttribute("aria-valuenow", String(resting()));
  expect(listWidth()).toBe(resting());

  await handle().click();
  await userEvent.keyboard("{ArrowRight}");
  await expect.poll(listWidth).toBeGreaterThan(resting());
  // Home is the floor, and the column goes there rather than to whatever the
  // keys had reached — the clamp is the same one a drag reads.
  await userEvent.keyboard("{Home}");
  await expect.poll(listWidth).toBe(Number(handle().element().getAttribute("aria-valuemin")));
});

test("the width is remembered, so the catalogue opens at it next time", async () => {
  // Its own host and its own teardown, `left-column-rail.test.tsx`'s pattern:
  // this test reopens the window, and `unmountAfterEach` would take the first
  // root down a second time.
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const first = mountApp(onJob(running()), host);
  await openCatalogue();
  await handle().click();
  await userEvent.keyboard("{End}");
  const widest = handle().element().getAttribute("aria-valuemax");
  expect(window.localStorage.getItem(WIDTH_KEY)).toBe(widest);

  // A remount is the window reopening: the state is gone and only what was
  // written survives. A column that forgets a drag on every launch is the
  // control not being worth having.
  first.unmount();
  host.remove();

  mount(onJob(running()));
  await openCatalogue(true);
  await expect.element(handle()).toHaveAttribute("aria-valuenow", widest);
});
