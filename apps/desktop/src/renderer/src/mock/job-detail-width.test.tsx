// Job detail's two columns, measured through `App` at the window widths the
// owner hit — the defect being that the run column held its 380px, Helm's dock
// held its 380px, and the step panel took what was left, which at 1280 was
// 192px of one-letter-a-line log.
//
// **A geometry test, so it lives here and not in a story.** The three widths
// that matter are decided by the whole window — the left column, the dock's own
// rules and the `--layout-breakpoint` the dock folds at — and a story drawing
// one composition in a sized `div` cannot see any of them. `Layer.test.tsx` is
// the precedent for driving the viewport in this project.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { FLEET_DOT_TONE } from "@armada/components";
import type { Connection } from "@armada/protocol";
import { SHORT_LABEL } from "@armada/shell";
import { workingAPlan } from "@armada/screens/src/fixtures/build/index";

import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

// The project's own size, back after every test: the viewport is the one piece
// of state here that outlives an unmount.
const RESTING = { width: 1440, height: 900 };
afterEach(async () => {
  await page.viewport(RESTING.width, RESTING.height);
});

/** The floors, as `spacing.css` declares them. Read, never retyped. */
function floor(
  token: "--w-run-column-min" | "--w-step-panel-min" | "--window-fold-left" | "--sidebar-rail",
): number {
  const value = parseFloat(getComputedStyle(document.documentElement).getPropertyValue(token));
  if (!Number.isFinite(value)) throw new Error(`${token} is not declared`);
  return value;
}

const boxOf = (selector: string): DOMRect => {
  const element = document.querySelector<HTMLElement>(selector);
  if (element === null) throw new Error(`${selector} is not drawn`);
  return element.getBoundingClientRect();
};

/**
 * The worst text in the panel: the node drawn on the most lines per word it
 * has. **A run of text never needs more lines than it has words** — one line
 * each is the worst honest wrapping — so a ratio above 1 is a word that was
 * broken down the middle, which is the defect this file exists for. Measured
 * with a range over each text node, `WorkflowDiagram.stories.tsx`'s `linesOf`
 * at the scale of a whole panel.
 */
function brokenWord(): { text: string; words: number; lines: number } | null {
  const panel = document.querySelector<HTMLElement>(".armada-inside__panel");
  if (panel === null) throw new Error("the step panel is not drawn");
  const walker = document.createTreeWalker(panel, NodeFilter.SHOW_TEXT);
  let worst: { text: string; words: number; lines: number } | null = null;
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    const text = (node.textContent ?? "").trim();
    if (text === "") continue;
    const range = document.createRange();
    range.selectNodeContents(node);
    const lines = new Set(
      [...range.getClientRects()].filter((rect) => rect.width > 0 && rect.height > 0).map((rect) => Math.round(rect.top)),
    ).size;
    if (lines === 0) continue;
    const words = text.split(/\s+/).length;
    if (lines <= words) continue;
    if (worst === null || lines - words > worst.lines - worst.words) worst = { text, words, lines };
  }
  return worst;
}

/** App on the Job the owner had open, at `width`, with the layout settled. */
async function atWidth(width: number): Promise<void> {
  await page.viewport(width, 860);
  mount(onJob(workingAPlan()));
  await expect.element(page.getByText("THE RUN").first()).toBeVisible();
  await expect.poll(() => document.querySelector(".armada-inside__panel") !== null).toBe(true);
}

// 1280 is where the owner found it: wide enough for Helm's dock to sit beside
// the content, narrow enough that what is left cannot hold both columns at
// their drawn widths. The run column gives up 140px here and the panel keeps
// its floor exactly.
test("at 1280 with Helm's dock open, the step panel keeps its floor and the run column yields", async () => {
  await atWidth(1280);
  await expect.element(page.getByRole("button", { name: "Close" }).first()).toBeVisible();
  // First, because it is the thing the owner photographed: with the panel at
  // 192px this reported `Extract selectColumnOrder into its own module`, six
  // words on forty lines.
  expect(brokenWord()).toBe(null);
  expect(boxOf(".armada-inside__panel").width).toBeGreaterThanOrEqual(floor("--w-step-panel-min"));
  expect(boxOf(".armada-inside__run").width).toBeGreaterThanOrEqual(floor("--w-run-column-min"));
  // The crush did not move across the gutter: the run column gave width up, it
  // did not take the panel's place as the sliver.
  expect(boxOf(".armada-inside__run").width).toBeLessThan(380);
});

// Above the band the dock is affordable and nothing has to yield at all — the
// run column is still at its drawn width, which is what stops this fix reading
// as "the run column got narrower".
test("at 1512 the run column is still at its drawn width", async () => {
  await atWidth(1512);
  expect(boxOf(".armada-inside__run").width).toBe(380);
  expect(boxOf(".armada-inside__panel").width).toBeGreaterThanOrEqual(floor("--w-step-panel-min"));
  expect(brokenWord()).toBe(null);
});

// Below `--layout-breakpoint` the dock folds away and the panel was never the
// problem — but the window floor was, with no dock involved at all: 768px drew
// a 196px panel before this. Both ends of the band, and the floor itself.
test.each([1100, 1000, 900, 768])("at %i the step panel is at or above its floor", async (width) => {
  await atWidth(width);
  expect(boxOf(".armada-inside__panel").width).toBeGreaterThanOrEqual(floor("--w-step-panel-min"));
  expect(brokenWord()).toBe(null);
});

// Inside the band the window cannot pay for both floors, and the rule that
// yields there is the panel's `min()`. What it must never do is overflow the
// card and clip the log against its edge, which two hard minimums would.
test("at 1150 the log is not clipped against the card's edge", async () => {
  await atWidth(1150);
  const panel = boxOf(".armada-inside__panel");
  const inside = boxOf(".armada-inside");
  expect(panel.right).toBeLessThanOrEqual(Math.ceil(inside.right));
  expect(boxOf(".armada-inside__run").width).toBeGreaterThanOrEqual(floor("--w-run-column-min"));
  expect(brokenWord()).toBe(null);
});

// ---- the band, and what the left column does about it ---------------------
//
// The rule above kept the panel from clipping in the band but could not put
// pixels in it: at 1150 the panel was 202px, degrading smoothly and far too
// narrow to read. The pixels come from the left column folding away, and these
// measure the three things that claim rests on — that it folds inside the band,
// that both floors are met once it has, and that nothing outside the band moved.

const leftColumn = (): Element | null => document.querySelector(".armada-shell__left");

/**
 * The left column's own width — its computed one, not its box. It is
 * `content-box` with `--space-4` of padding either side, so its rect is 32px
 * wider than the width every other rule in the shell means by it.
 */
function leftColumnWidth(): number {
  const element = leftColumn();
  if (element === null) throw new Error("the left column is not drawn");
  return parseFloat(getComputedStyle(element).width);
}

/** Both floors met, at a width where they could not both be met before. */
function bothFloorsHold(): void {
  expect(boxOf(".armada-inside__panel").width).toBeGreaterThanOrEqual(floor("--w-step-panel-min"));
  expect(boxOf(".armada-inside__run").width).toBeGreaterThanOrEqual(floor("--w-run-column-min"));
  expect(brokenWord()).toBe(null);
}

// Both ends of the band and the width the owner photographed. 1101 is the first
// width above `--layout-breakpoint`, where the dock comes back beside the
// content and the band opens; 1279 is the last width under
// `--window-fold-left`, where it closes.
test.each([1101, 1150, 1250, 1279])(
  "at %i the left column is not drawn and both of job detail's columns hold their floors",
  async (width) => {
    await atWidth(width);
    await expect.element(page.getByRole("button", { name: "Close" }).first()).toBeVisible();
    await expect.poll(leftColumn).toBe(null);
    bothFloorsHold();
  },
);

// The fold is the window's arithmetic, so it is on every surface and not only
// the one that needs it — the same way `--layout-breakpoint` collapses the rail
// everywhere. Navigation, Stats and Fleet go together; only Fleet's own
// liveness stands in, as the title row's dot (#1437) — the panel itself, with
// its pid, port, protocol and uptime, is gone with the rest.
test("while folded, Navigation, Stats and Fleet are absent rather than hidden", async () => {
  await atWidth(1150);
  await expect.poll(leftColumn).toBe(null);
  expect(document.querySelector(".armada-shell__left-handle")).toBe(null);
  expect(document.querySelector(".armada-sidebar")).toBe(null);
  // Helm's dock is what the band is paying for, and it is untouched.
  await expect.element(page.getByLabelText("Helm").first()).toBeVisible();
});

// ---- Fleet's dot, the one thing the fold leaves behind -------------------
//
// The dot is drawn from the window's width, which is exactly what a story
// cannot see — so it is measured here, through the same `App` the fold is,
// and the two widths are the band and one above it.

/** The title row's Fleet dot, or `null`. Its own class, because absence is what the wide case asserts. */
const fleetDot = (): Element | null => document.querySelector(".armada-title-bar__fleet");

// The mock answers, so the reading is a connected Fleet's. Both words come
// from the producers Bridge itself uses — `shortLabelOf` for the panel's own
// word and `FLEET_DOT_TONE` for the hue — rather than the two strings retyped
// here, so a rename in either place fails this rather than passing a stale copy.
const RUNNING = "connected" satisfies Connection["state"];
const SAID = `Fleet — ${SHORT_LABEL[RUNNING]}`;

test("at 1150 the title row carries Fleet's state, named and toned", async () => {
  await atWidth(1150);
  await expect.poll(leftColumn).toBe(null);
  // Found by its accessible name, never its class: the name is the assertion,
  // and it is what a person who cannot see the hue is left with.
  const dot = page.getByRole("img", { name: SAID }).first();
  await expect.element(dot).toBeVisible();
  await expect.element(dot).toHaveAttribute("title", SAID);
  expect(fleetDot()?.firstElementChild?.getAttribute("data-tone")).toBe(FLEET_DOT_TONE.running);
  // Liveness only: what the panel carried is not smuggled into the title row.
  expect(fleetDot()?.textContent).toBe("");
});

test("at 1512 the panel has the dot and the title row draws none", async () => {
  await atWidth(1512);
  await expect.poll(leftColumn).not.toBe(null);
  await expect.poll(fleetDot).toBe(null);
  // And the reading it stands in for is back where it belongs, with the facts
  // a dot could not carry beside it.
  await expect.element(page.getByText("pid").first()).toBeVisible();
});

// `--window-fold-left` is the narrowest window that still fits, not the widest
// that does not: at exactly 1280 the column is drawn and the panel is at its
// floor to the pixel, which is the measurement #1428 landed and this must not
// move. One pixel narrower is the first fold. Two tests and not one, because
// `mount` puts a second App in the document rather than replacing the first.
test("at --window-fold-left the column still stands, on the floors exactly", async () => {
  await atWidth(floor("--window-fold-left"));
  await expect.poll(leftColumn).not.toBe(null);
  bothFloorsHold();
  expect(boxOf(".armada-inside__panel").width).toBe(floor("--w-step-panel-min"));
});

test("one pixel under --window-fold-left the column folds", async () => {
  await atWidth(floor("--window-fold-left") - 1);
  await expect.poll(leftColumn).toBe(null);
  bothFloorsHold();
});

// The dock is what takes the room, so a closed dock keeps the column: the
// surface does not disappear at 1150 unless something is using the pixels.
test("at 1150 with Helm's dock closed the left column is drawn", async () => {
  await atWidth(1150);
  await expect.poll(leftColumn).toBe(null);
  // ⌘J rather than the dock's own Close: the binding is what a person presses
  // to get the room back, and it is the one press that reaches the dock's state
  // from outside it.
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.poll(leftColumn).not.toBe(null);
  expect(leftColumnWidth()).toBeGreaterThan(floor("--sidebar-rail"));
});

// Above the band nothing moved, and below it nothing moved: the column is
// drawn at both ends, expanded at 1512 and collapsed to the rail at 1100.
test.each([1512, 1440])("at %i the left column is still expanded", async (width) => {
  await atWidth(width);
  await expect.poll(leftColumn).not.toBe(null);
  expect(leftColumnWidth()).toBeGreaterThan(floor("--sidebar-rail"));
});

test.each([1100, 900, 768])("at %i the left column is still the rail", async (width) => {
  await atWidth(width);
  await expect.poll(leftColumn).not.toBe(null);
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
});
