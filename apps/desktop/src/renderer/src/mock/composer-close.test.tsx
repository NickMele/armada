// The composer's way out, through `App`. The owner's note of 2026-09-17, on
// the Cancel this replaced: it "lingers in the middle of nowhere" — its own
// line above the card, left-aligned, belonging to nothing, which is what was
// left of the page head #1090 ended everywhere but Job detail.
//
// **The way out sits on the head of the thing it closes**, in every state the
// composer draws, and carries the key that does the same act. Job settings and
// Helm's dock already draw theirs that way; this is the same control, so the
// assertions below read the word and the key a person reads, never a class.
//
// The ask has no card header to sit in — it is an alert — so there the control
// takes the alert's own trailing-edge slot. Which is why the place is asserted
// against the state's own frame rather than against one shape for all three.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The control, by what a person reads on it: the word, then the key beside it. */
const WAY_OUT = "Close Esc";

/** The ask, which is the composer's first state while the Board is on All repositories. */
const ASK = "Pick the repository this Job is for";

/**
 * How far inside its frame's trailing edge the control may sit. A card header
 * is flush, so it lands on zero; the ask's alert pads its own contents, and
 * the padding is what this allows.
 */
const INSET = 16;

/** The composer, opened the way a person opens it: the title row's Dispatch. */
async function composing(): Promise<void> {
  mount("every-state");
  await page.getByRole("button", { name: "Dispatch", exact: true }).first().click();
  await expect.element(page.getByText(ASK)).toBeVisible();
}

/** Answer the ask, which is what draws the composer itself. */
async function answered(): Promise<void> {
  const select = page.getByLabelText("Repository", { exact: true }).element() as HTMLSelectElement;
  const offered = [...select.options].find((one) => !one.disabled && one.value !== "");
  await userEvent.selectOptions(select, offered!.value);
  await expect.element(page.getByRole("heading", { name: "Dispatch a job" })).toBeVisible();
}

/** What a state is once it is on screen: its own frame, and the title at the head of it. */
type Drawn = { frame: Element; title: Element };

/** Every state `Composing` draws, and how each is reached and read. */
const STATES: { what: string; open: () => Promise<Drawn> }[] = [
  {
    what: "The repository ask",
    async open() {
      await composing();
      const title = page.getByText(ASK).element();
      return { frame: title.closest('[role="status"]')!, title };
    },
  },
  {
    what: "The composer",
    async open() {
      await composing();
      await answered();
      const title = page.getByRole("heading", { name: "Dispatch a job" }).element();
      return { frame: title.parentElement!, title };
    },
  },
  {
    what: "Hand entry",
    async open() {
      await composing();
      await answered();
      await page.getByRole("button", { name: "Enter by hand" }).click();
      const title = page.getByRole("heading", { name: "Propose a job" }).element();
      return { frame: title.parentElement!, title };
    },
  },
];

/** The surface the composer was opened from, back again. `App` opens on Overview. */
async function backToOverview(): Promise<void> {
  await expect.poll(() => page.getByRole("button", { name: WAY_OUT }).query()).toBe(null);
  await expect.element(page.getByRole("heading", { name: "Running" }).first()).toBeVisible();
}

test.for(STATES)("$what carries the way out on its own head, and it closes the composer", async (state) => {
  const { frame, title } = await state.open();
  const control = page.getByRole("button", { name: WAY_OUT });
  await expect.element(control).toBeVisible();
  const close = control.element();

  expect(frame.contains(close), `${state.what}: the way out is not on the head of what it closes`).toBe(true);
  const box = close.getBoundingClientRect();
  const head = frame.getBoundingClientRect();
  const named = title.getBoundingClientRect();
  expect(box.left, `${state.what}: the way out is not past its title`).toBeGreaterThan(named.right);
  expect(head.right - box.right, `${state.what}: the way out is not at the trailing edge`).toBeLessThanOrEqual(INSET);
  expect(box.top, `${state.what}: the way out sits above what it closes rather than on it`).toBeGreaterThanOrEqual(
    head.top,
  );

  await control.click();
  await backToOverview();
});

test.for(STATES)("$what: Escape closes the composer, which is what the control says", async (state) => {
  await state.open();
  await userEvent.keyboard("{Escape}");
  await backToOverview();
});
