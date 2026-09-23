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
// The ask is an alert rather than a card, so its head is the alert's title row
// rather than a `CardHeader`. That is a difference in markup and not in what a
// person sees, which is why the place is asserted against each state's own
// frame and title — past the title, at the trailing edge, level with the
// title's first line — rather than against one shape for both.

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

/**
 * How far the control's own vertical centre may sit off the middle of the
 * title's first line. Level is the whole ask — the owner's note of 2026-09-17
 * on the ask's control, which the alert's centred slot had put two lines
 * below its title: "it just kind of sits in the middle". A head row centres
 * the two against each other, so the only slack wanted here is the rounding
 * a line box and a control of different heights leave behind.
 */
const LEVEL = 2;

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

/**
 * Every state `Composing` draws, and how each is reached and read. There were
 * three: hand entry went with the form on 2026-09-23.
 */
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
  // On the title's line, not somewhere down the body. A title that wrapped
  // would report its whole block, so the first line box is what is read.
  const line = title.getClientRects()[0] ?? named;
  expect(
    Math.abs((box.top + box.bottom) / 2 - (line.top + line.bottom) / 2),
    `${state.what}: the way out is not level with its title's first line`,
  ).toBeLessThanOrEqual(LEVEL);

  await control.click();
  await backToOverview();
});

test.for(STATES)("$what: Escape closes the composer, which is what the control says", async (state) => {
  await state.open();
  await userEvent.keyboard("{Escape}");
  await backToOverview();
});

// The guard on the key, his call of 2026-09-17: an untouched composer closes at
// once — the tests above — and one carrying anything typed asks first,
// the way killing a Job does. The control and the key are one act, so both ask.

/** The ask, by its own role and the title it is named by. */
const ASKED = () => page.getByRole("dialog", { name: "Discard what you typed?" });

/**
 * Every state something can be typed into, and what typing in it leaves
 * behind. The hand form was the second and is gone with it.
 */
const TYPED: { what: string; type: () => Promise<() => string> }[] = [
  {
    what: "The request",
    async type() {
      await composing();
      await answered();
      const field = page.getByLabelText("Request", { exact: true });
      await userEvent.type(field, "fix the flake in the board test");
      return () => (field.element() as HTMLTextAreaElement).value;
    },
  },
];

test.for(TYPED)("$what: the key asks before it throws anything away, and no is no", async (state) => {
  const written = await state.type();
  const before = written();

  await userEvent.keyboard("{Escape}");
  await expect.element(ASKED()).toBeVisible();
  // The press over the ask is the ask's to answer and never a second exit: it
  // cancels, and the composer is still there with what was typed in it.
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => ASKED().query()).toBe(null);
  await expect.element(page.getByRole("button", { name: WAY_OUT })).toBeVisible();
  expect(written(), `${state.what}: answering no lost what was typed`).toBe(before);

  // Asked again, answered yes: that is the way out.
  await userEvent.keyboard("{Escape}");
  await ASKED().getByRole("button", { name: "Discard" }).click();
  await backToOverview();
});

test.for(TYPED)("$what: the control asks under the same condition, and yes closes", async (state) => {
  const written = await state.type();
  const before = written();

  await page.getByRole("button", { name: WAY_OUT }).click();
  await expect.element(ASKED()).toBeVisible();
  await ASKED().getByRole("button", { name: "Cancel" }).click();
  await expect.poll(() => ASKED().query()).toBe(null);
  expect(written(), `${state.what}: answering no lost what was typed`).toBe(before);

  // And the same control again, answered yes this time, is the way out.
  await page.getByRole("button", { name: WAY_OUT }).click();
  await ASKED().getByRole("button", { name: "Discard" }).click();
  await backToOverview();
});
