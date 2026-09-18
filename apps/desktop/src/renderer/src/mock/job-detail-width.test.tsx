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
import { page } from "vitest/browser";
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
function floor(token: "--w-run-column-min" | "--w-step-panel-min"): number {
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
