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
import { FLEET_DOT_TONE, fleetSaid } from "@armada/components";
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
  token:
    | "--w-run-column-min"
    | "--w-step-panel-min"
    | "--window-fold-left"
    | "--sidebar-rail"
    | "--w-sheet",
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

/**
 * App on the Job the owner had open, at `width`, with the layout settled.
 *
 * **It waits on the run and not on the inspector.** Under
 * `--layout-breakpoint` the inspector is a sheet that opens on a press, so a
 * wait for the panel would hang at every width the fold covers — which is
 * three of the widths this file measures.
 */
async function atWidth(width: number): Promise<void> {
  await page.viewport(width, 860);
  mount(onJob(workingAPlan()));
  await expect.element(page.getByText("THE RUN").first()).toBeVisible();
  await expect.poll(() => document.querySelector(".armada-inside__run") !== null).toBe(true);
}

/** Press a step in the run, which is what opens the folded inspector. */
async function openInspector(): Promise<void> {
  await page.getByRole("button", { name: "Fix", exact: true }).first().click();
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

// ---- below `--layout-breakpoint`, where the inspector folds ---------------
//
// **This is where the two-floor claim moved to, not where it was deleted.**
// Until #1534 the panel was a column at every width and these three widths
// asserted it held `--w-step-panel-min`; 768px drew a 196px panel before #1428
// and the floor is what fixed that. Below the bound there is no arithmetic that
// pays for two columns at once, so the inspector is a sheet over the run — the
// move Helm's dock already makes at the same bound — and the claim is the same
// one it always was: the reading is wide enough that no word breaks in it.
test.each([1100, 1000, 900, 768])("at %i the run is the whole content and no inspector column is drawn", async (width) => {
  await atWidth(width);
  expect(document.querySelector(".armada-inside__panel")).toBe(null);
  // One track: the run is as wide as the arrangement, give or take rounding.
  expect(boxOf(".armada-inside__run").width).toBeCloseTo(boxOf(".armada-inside").width, 0);
});

// The sheet is the reading, so it is the sheet the broken-word walker reads.
// At the floor it goes flush to both edges, which is `Sheet`'s own rule and is
// why the expected width is the ground there rather than `--w-sheet`.
test.each([1100, 1000, 900])("at %i pressing a step opens the inspector at --w-sheet, unbroken", async (width) => {
  await atWidth(width);
  await openInspector();
  expect(boxOf(".armada-inside__panel").width).toBeGreaterThanOrEqual(floor("--w-sheet") - 1);
  expect(brokenWord()).toBe(null);
});

test("at 768 the folded inspector is flush to both edges and nothing breaks", async () => {
  await atWidth(768);
  await openInspector();
  const sheet = boxOf(".armada-sheet");
  expect(sheet.width).toBeGreaterThan(floor("--w-sheet"));
  expect(brokenWord()).toBe(null);
});

// Esc closes it, and the run is still where the reader left it: a layer, never
// a route. `Sheet` catches the press in the capture phase so the same key does
// not also leave the Job.
test("Esc closes the folded inspector and leaves the Job open", async () => {
  await atWidth(1000);
  await openInspector();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => document.querySelector(".armada-inside__panel")).toBe(null);
  await expect.element(page.getByText("THE RUN").first()).toBeVisible();
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
// narrow to read. The pixels come from the left column collapsing to its rail
// — #1435 removed the column outright and the owner corrected it on 18 Sep
// 2026 — and these measure that it is the rail inside the band, that both
// floors are met wherever 80px of rail pays for them, and that nothing outside
// the band moved.
//
// **The rail costs 80 where the absence cost 16**, so the band does not close
// completely: `RAIL_PAYS` is the width below which the step panel is still
// short, and `spacing.css` has the sum.
const RAIL_PAYS = 1128;

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

// The band, from the width where the rail's 80px first pay for both floors up
// to the last width under `--window-fold-left`. 1128 is the sum in
// `spacing.css`, measured here rather than asserted from it: at exactly that
// width both columns sit on their floors to the pixel, which is what proves
// the arithmetic rather than restating it.
test.each([RAIL_PAYS, 1150, 1250, 1279])(
  "at %i the left column is the rail and both of job detail's columns hold their floors",
  async (width) => {
    await atWidth(width);
    await expect.element(page.getByRole("button", { name: "Close" }).first()).toBeVisible();
    await expect.poll(leftColumn).not.toBe(null);
    expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
    bothFloorsHold();
  },
);

test("at 1128 exactly, both of job detail's columns are on their floors to the pixel", async () => {
  await atWidth(RAIL_PAYS);
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
  expect(boxOf(".armada-inside__panel").width).toBe(floor("--w-step-panel-min"));
  expect(boxOf(".armada-inside__run").width).toBe(floor("--w-run-column-min"));
});

// The bottom of the band, where the rail is not cheap enough. The owner took
// the rail over the absence knowing this: the run column holds its floor, the
// panel gives way on `.armada-inside`'s own `min()`, and 305px is what the
// 202px that started all this became. Nothing clips and no word breaks, which
// is the whole claim — the floor is missed, not the reading.
test.each([1101, 1127])("at %i the rail is drawn and the step panel gives way without clipping", async (width) => {
  await atWidth(width);
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
  expect(boxOf(".armada-inside__run").width).toBe(floor("--w-run-column-min"));
  const panel = boxOf(".armada-inside__panel");
  expect(panel.width).toBeLessThan(floor("--w-step-panel-min"));
  expect(panel.width).toBeGreaterThan(300);
  expect(panel.right).toBeLessThanOrEqual(Math.ceil(boxOf(".armada-inside").right));
  expect(brokenWord()).toBe(null);
});

// The collapse is the window's arithmetic, so it is on every surface and not
// only the one that needs it — the same way `--layout-breakpoint` collapses the
// rail everywhere. Navigation, Stats and Fleet go together and all three are
// still in the tree, which is the #1435 regression: they were not.
test("in the band, Navigation, Stats and Fleet are at the rail rather than gone", async () => {
  await atWidth(1150);
  await expect.poll(leftColumn).not.toBe(null);
  expect(document.querySelector(".armada-shell__left-handle")).toBe(null);
  expect(document.querySelector(".armada-sidebar")).not.toBe(null);
  await expect.element(page.getByRole("button", { name: "Job Board" }).first()).toBeVisible();
  // Stats and Fleet keep one status dot each, as named regions.
  await expect.element(page.getByRole("region", { name: "Stats" }).first()).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Fleet" }).first()).toBeVisible();
  // Helm's dock is what the band is paying for, and it is untouched.
  await expect.element(page.getByLabelText("Helm").first()).toBeVisible();
});

// ---- Fleet's dot, in the title row and at the rail ------------------------
//
// Whether the dot is drawn used to be decided by the window's width, which is
// exactly what a story cannot see. It is not decided by anything now — #1438's
// condition was corrected on 18 Sep 2026 and the title row carries it at every
// width — so what is measured here is the pair: the title row's dot, and the
// Fleet panel's own at the rail, which is a second element with the same name.

/** The title row's Fleet dot. Its own class, because the pair is what the rail widths count. */
const fleetDot = (): Element | null => document.querySelector(".armada-title-bar__fleet");

// The mock answers, so the reading is a connected Fleet's. Every part comes
// from the producers Bridge itself uses — `shortLabelOf` for the panel's own
// word, `fleetSaid` for the sentence and `FLEET_DOT_TONE` for the hue — rather
// than retyped here, so a rename fails this rather than passing a stale copy.
const RUNNING = "connected" satisfies Connection["state"];
const SAID = fleetSaid(SHORT_LABEL[RUNNING]);

// At the rail there are two readings of one fact and the owner chose both: the
// title row's permanent dot, and the panel collapsed to its own dot. The second
// was silent until 18 Sep 2026 — an `aria-hidden` mark inside a region named
// "Fleet" — so a person who cannot see the hue was told the panel's name and
// never its state at the one width where the name is all there is.
test.each([1150, 1000])("at %i both Fleet dots are named and toned", async (width) => {
  await atWidth(width);
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
  // Found by accessible name, never by class: the name is the assertion, and
  // it is what a person who cannot see the hue is left with.
  const named = page.getByRole("img", { name: SAID });
  expect(named.elements()).toHaveLength(2);
  for (const element of named.elements()) expect(element.getAttribute("title")).toBe(SAID);
  expect(fleetDot()?.firstElementChild?.getAttribute("data-tone")).toBe(FLEET_DOT_TONE.running);
  // Liveness only: what the panel's rows carried is not smuggled into either.
  expect(fleetDot()?.textContent).toBe("");
});

test("at 1512 the title row keeps its dot and the panel says it in words instead", async () => {
  await atWidth(1512);
  expect(leftColumnWidth()).toBeGreaterThan(floor("--sidebar-rail"));
  // One, not two: the expanded panel has the word in its body, so it takes no
  // name of its own and the row's dot is the only thing carrying the sentence.
  expect(page.getByRole("img", { name: SAID }).elements()).toHaveLength(1);
  await expect.element(page.getByRole("img", { name: SAID }).first()).toBeVisible();
  // And the facts a dot could not carry are back beside it.
  await expect.element(page.getByText("pid").first()).toBeVisible();
});

// `--window-fold-left` is the narrowest window that still fits, not the widest
// that does not: at exactly 1280 the column is at its full width and the panel
// is at its floor to the pixel, which is the measurement #1428 landed and this
// must not move. One pixel narrower is the first rail. Two tests and not one,
// because `mount` puts a second App in the document rather than replacing it.
test("at --window-fold-left the column still stands at width, on the floors exactly", async () => {
  await atWidth(floor("--window-fold-left"));
  expect(leftColumnWidth()).toBeGreaterThan(floor("--sidebar-rail"));
  bothFloorsHold();
  expect(boxOf(".armada-inside__panel").width).toBe(floor("--w-step-panel-min"));
});

test("one pixel under --window-fold-left the column is the rail", async () => {
  await atWidth(floor("--window-fold-left") - 1);
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
  bothFloorsHold();
});

// The dock is what takes the room, so a closed dock gives the column its width
// back: it narrows at 1150 only while something is using the pixels.
test("at 1150 with Helm's dock closed the left column is at its full width", async () => {
  await atWidth(1150);
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
  // ⌘J rather than the dock's own Close: the binding is what a person presses
  // to get the room back, and it is the one press that reaches the dock's state
  // from outside it.
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.poll(leftColumnWidth).toBeGreaterThan(floor("--sidebar-rail"));
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
