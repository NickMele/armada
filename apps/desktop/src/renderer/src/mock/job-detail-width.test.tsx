// Job detail's two columns, measured through `App` at the window widths the
// owner hit — the defect being that the run column held its 380px, Helm's dock
// held its 380px, and the step panel took what was left, which at 1280 was
// 192px of one-letter-a-line log.
//
// **#1583 took one of those three terms away.** The dock is a layer now, so
// the band this file used to measure — 1101 to 1279, the left column at its
// rail paying for it — is gone, and the claim is the one it did not have
// before: the content is the same width with Helm open and shut.
//
// **A geometry test, so it lives here and not in a story.** The widths that
// matter are decided by the whole window, and a story drawing one composition
// in a sized `div` cannot see any of them. `Layer.test.tsx` is the precedent
// for driving the viewport in this project.

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

/** The floors and the hairline, as `spacing.css` declares them. Read, never retyped. */
function floor(
  token:
    | "--w-run-column-min"
    | "--w-step-panel-min"
    | "--sidebar-default"
    | "--layout-breakpoint"
    | "--sidebar-rail"
    | "--w-sheet"
    | "--border-width",
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
 * **A `DOMRect` is a float and a token is an integer, so the two are never
 * compared with `toBe` or a bare `>=`.** The same sheet measured
 * 478.99993896484375 on a busy machine where a quiet one read 479, which sank
 * an assertion sitting exactly on its bound — #1574, four times in about a
 * dozen full runs. Half a pixel is the tolerance `toBeCloseTo(…, 0)` states
 * and this restates, in one place rather than a `- 1` per line: widening the
 * bound only moves the cliff, and a 4px regression fails either way.
 *
 * **Two rects measured in one layout keep `toBe`.** `panelWidth()` either side
 * of `⌘J` is one element in one window, so #1583's claim is bit-for-bit and a
 * tolerance would admit the shift it exists to refuse.
 */
const ROUNDING = 0.5;

/** Measured at `bound` or above, give or take the rounding. */
function atLeast(measured: number, bound: number): void {
  expect(measured).toBeGreaterThanOrEqual(bound - ROUNDING);
}

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
 * **Helm is shut**, which is how Bridge opens since #1583; `withHelm` is what
 * puts it up.
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

/**
 * Put Helm up. **`⌘J` rather than the title row's button**: it is the one way
 * in that works at every width and from inside a field, and it is what a
 * person presses.
 */
async function withHelm(): Promise<void> {
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.poll(() => document.querySelector(".armada-shell__dock-layer, .armada-sheet")).not.toBe(null);
}

/** Press a step in the run, which is what opens the folded inspector. */
async function openInspector(): Promise<void> {
  await page.getByRole("button", { name: "Fix", exact: true }).first().click();
  await expect.poll(() => document.querySelector(".armada-inside__panel") !== null).toBe(true);
}

// 1280 is where the owner found it. It was wide enough for Helm's dock to sit
// beside the content and too narrow for what was left to hold both columns at
// their drawn widths, so the run column gave up 140px. **Nothing yields here
// any more** — the dock stopped taking the 412px, and the 1280 measurement
// that #1428 landed is now the easy case rather than the hard one.
test("at 1280 with Helm up, neither of job detail's columns yields", async () => {
  await atWidth(1280);
  await withHelm();
  // First, because it is the thing the owner photographed: with the panel at
  // 192px this reported `Extract selectColumnOrder into its own module`, six
  // words on forty lines.
  expect(brokenWord()).toBe(null);
  atLeast(boxOf(".armada-inside__panel").width, floor("--w-step-panel-min"));
  expect(boxOf(".armada-inside__run").width).toBeCloseTo(380, 0);
});

// The run column is still at its drawn width up here too, which is what stops
// this reading as "the run column got narrower".
test("at 1512 the run column is still at its drawn width", async () => {
  await atWidth(1512);
  expect(boxOf(".armada-inside__run").width).toBeCloseTo(380, 0);
  atLeast(boxOf(".armada-inside__panel").width, floor("--w-step-panel-min"));
  expect(brokenWord()).toBe(null);
});

// ---- what #1583 claims, measured ------------------------------------------
//
// **Opening Helm changes nothing about the width of what is behind it.** The
// issue's own definition of done, and the fact three screens each wrote their
// own workaround around on 22 September. Measured on the shell's panel rather
// than on job detail's columns, because it is true of every surface and job
// detail is only the one with a fixture open in this file.

const panelWidth = (): number => boxOf(".armada-shell__panel").width;

test.each([2000, 1512, 1440, 1280, 1101])(
  "at %i the content is the same width with Helm shut and open",
  async (width) => {
    await atWidth(width);
    const shut = panelWidth();
    await withHelm();
    expect(panelWidth()).toBe(shut);
    // And the run under it did not move either, which is the part a person sees.
    expect(boxOf(".armada-inside__run").width).toBeCloseTo(380, 0);
    // Shut again, by the one press the issue asks for — the dock's own Close,
    // scoped to it, because job detail draws a Close of its own.
    await page.getByRole("complementary", { name: "Helm" }).getByRole("button", { name: /^Close/ }).click();
    await expect.poll(() => document.querySelector(".armada-shell__dock-layer")).toBe(null);
    expect(panelWidth()).toBe(shut);
  },
);

// Below the breakpoint the dock is a sheet rather than a panel, and the strip
// that opens it is drawn whether or not it is open — so the claim holds there
// too, and for a different reason worth measuring separately.
test.each([1000, 768])("at %i the content is the same width with the sheet shut and open", async (width) => {
  await atWidth(width);
  const shut = panelWidth();
  await withHelm();
  await expect.element(page.getByRole("dialog", { name: "Helm" })).toBeVisible();
  expect(panelWidth()).toBe(shut);
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
//
// **The panel is a hairline narrower than the sheet, and that was never slack.**
// `Sheet`'s trailing side draws `border-left: var(--border-width)`, so the sheet
// is `--w-sheet` to the pixel and the panel inside it is one less. The `- 1`
// here read as tolerance and was arithmetic, which is how it came to sit exactly
// on the bound a busy machine rounded under — #1574. Both are named now, each
// against what declares it.
test.each([1100, 1000, 900])("at %i pressing a step opens the inspector at --w-sheet, unbroken", async (width) => {
  await atWidth(width);
  await openInspector();
  expect(boxOf(".armada-sheet").width).toBeCloseTo(floor("--w-sheet"), 0);
  expect(boxOf(".armada-inside__panel").width).toBeCloseTo(floor("--w-sheet") - floor("--border-width"), 0);
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
  atLeast(boxOf(".armada-inside__run").width, floor("--w-run-column-min"));
  expect(brokenWord()).toBe(null);
});

// ---- the left column, and the band it used to collapse in -----------------
//
// The column had two collapse points: `--layout-breakpoint`, and a second at
// `--window-fold-left` where it fell to its 48px rail so Helm's dock could
// keep its 380. #1435 removed the column outright there and the owner
// corrected it on 18 Sep 2026 to the rail; **#1583 removed the reason**, so
// the band is gone and the rail's 152px are not owed to anyone. What these
// measure is the other half of the same claim: the column is where the window
// puts it, and Helm cannot move it.

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
  atLeast(boxOf(".armada-inside__panel").width, floor("--w-step-panel-min"));
  atLeast(boxOf(".armada-inside__run").width, floor("--w-run-column-min"));
  expect(brokenWord()).toBe(null);
}

// The old band, end to end, with Helm up. Every one of these drew the rail
// before and drew a step panel under its floor at the bottom of it; all four
// are the ordinary case now.
test.each([1101, 1150, 1250, 1279])(
  "at %i the left column stands at width with Helm up, and both of job detail's columns hold",
  async (width) => {
    await atWidth(width);
    await withHelm();
    expect(leftColumnWidth()).toBe(floor("--sidebar-default"));
    bothFloorsHold();
  },
);

// One pixel over the breakpoint, which is where the column now goes from rail
// to width. Nothing between here and the top of the range is a special case.
test("one pixel over --layout-breakpoint the column is at width, with Helm up", async () => {
  await atWidth(floor("--layout-breakpoint") + 1);
  await withHelm();
  expect(leftColumnWidth()).toBe(floor("--sidebar-default"));
  bothFloorsHold();
});

// The collapse is the window's arithmetic, so it is on every surface and not
// only the one that needs it. Navigation, Stats and Fleet go together and all
// three are still in the tree, which is the #1435 regression: they were not.
test("at the rail, Navigation, Stats and Fleet are there rather than gone", async () => {
  await atWidth(1000);
  await expect.poll(leftColumn).not.toBe(null);
  expect(document.querySelector(".armada-shell__left-handle")).toBe(null);
  expect(document.querySelector(".armada-sidebar")).not.toBe(null);
  await expect.element(page.getByRole("button", { name: "Job Board" }).first()).toBeVisible();
  // Stats and Fleet keep one status dot each, as named regions.
  await expect.element(page.getByRole("region", { name: "Stats" }).first()).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Fleet" }).first()).toBeVisible();
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
test.each([1000, 900])("at %i both Fleet dots are named and toned", async (width) => {
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

// `--layout-breakpoint` is the widest window the rail is drawn at, and one
// pixel over it the column stands at its full width. Two tests and not one,
// because `mount` puts a second App in the document rather than replacing it.
test("at --layout-breakpoint the column is the rail", async () => {
  await atWidth(floor("--layout-breakpoint"));
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
});

test("one pixel over --layout-breakpoint the column is at its full width", async () => {
  await atWidth(floor("--layout-breakpoint") + 1);
  expect(leftColumnWidth()).toBe(floor("--sidebar-default"));
  bothFloorsHold();
});

// **Helm is not one of the terms.** This is the test that used to say the
// opposite — at 1150 the column was the rail until ⌘J gave the pixels back,
// which is the coupling #1583 ends. Both presses, so a width that only holds
// while the dock has never been opened would fail here.
test("at 1150 the left column does not move when Helm opens or closes", async () => {
  await atWidth(1150);
  const width = leftColumnWidth();
  expect(width).toBe(floor("--sidebar-default"));
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.poll(() => document.querySelector(".armada-shell__dock-layer")).not.toBe(null);
  expect(leftColumnWidth()).toBe(width);
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.poll(() => document.querySelector(".armada-shell__dock-layer")).toBe(null);
  expect(leftColumnWidth()).toBe(width);
});

// Above the breakpoint nothing is at the rail, and below it everything is.
test.each([1512, 1440, 1150, 1101])("at %i the left column is at its full width", async (width) => {
  await atWidth(width);
  await expect.poll(leftColumn).not.toBe(null);
  expect(leftColumnWidth()).toBe(floor("--sidebar-default"));
});

test.each([1100, 900, 768])("at %i the left column is still the rail", async (width) => {
  await atWidth(width);
  await expect.poll(leftColumn).not.toBe(null);
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
});
