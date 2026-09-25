// The three shapes of a guide, through `App` — a **prototype for one decision**
// and not a landing. The owner's note, 25 September 2026, on the Guides
// catalogue: *"I would prefer guides be animations or steps, not just a wall of
// text."* Three shapes are built so he can look at all three; the two he does
// not choose get deleted.
//
// Storybook cannot make these claims. What the shape is read from — the mock's
// `?guides=`, never a setting in Bridge — is a fact about the window, and the
// last test here is the one that says the app itself is unchanged.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import { GUIDE_COMPLETION, GUIDE_MEMBER_LINK } from "@armada/components";
import type { Guide, GuideShape } from "@armada/components";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The scenario he opens to compare, and the one in the pull request's URLs. */
const SCENARIO = "arc/executing-concurrent";

/** The catalogue, reached the way a person reaches it: by the rail. */
async function catalogue(shape?: GuideShape): Promise<void> {
  mount(SCENARIO, shape);
  await page.getByRole("button", { name: "Guides", exact: true }).first().click();
}

/** A guide's row in the list. */
const rowFor = (guide: Guide) =>
  page.getByRole("button", { name: `Guide ${guide.number}, ${guide.title}` });

/** The panel beside the list, holding one guide. */
const panelFor = (guide: Guide) => page.getByRole("article", { name: guide.title });

/** The `?` beside a piece, by the name `GuideMark` gives it. */
const markFor = (guide: Guide) =>
  page.getByRole("button", { name: `Open guide ${guide.number}, ${guide.title}` });

/** The card a `?` opens. */
const cardFor = (guide: Guide) =>
  page.getByRole("dialog", { name: `Guide ${guide.number}, ${guide.title}` });

/** A guide's steps, which only the two rewritten ones carry. */
function stepsOf(guide: Guide): { lines: readonly string[]; hasFigure: boolean } {
  const shapes = guide.shapes;
  if (shapes === undefined) throw new Error(`guide ${guide.number} carries no shapes`);
  return { lines: shapes.steps.lines, hasFigure: shapes.steps.figure !== undefined };
}

/** A guide's captions under its drawing. */
function captionsOf(guide: Guide): readonly string[] {
  const shapes = guide.shapes;
  if (shapes === undefined) throw new Error(`guide ${guide.number} carries no shapes`);
  return shapes.figure.captions;
}

/** Every numbered step drawn in one region, as its text. */
const drawnSteps = (where: ReturnType<typeof panelFor>) =>
  where
    .getByRole("listitem")
    .elements()
    .map((one) => one.textContent ?? "");

/**
 * **A step list is numbered, one line each, and every line is the guide's own.**
 * Read off the drawing rather than off the data twice: a shape that drew the
 * lines in the wrong order, or dropped one, fails here.
 */
async function readsAsSteps(where: ReturnType<typeof panelFor>, guide: Guide): Promise<void> {
  const { lines } = stepsOf(guide);
  await expect.poll(() => drawnSteps(where).length).toBe(lines.length);
  const drawn = drawnSteps(where);
  for (const [index, line] of lines.entries()) {
    const step = drawn[index] ?? "";
    expect(step.startsWith(String(index + 1)), `step ${index + 1} is not numbered: ${step}`).toBe(true);
    expect(step).toContain(line);
  }
}

test("?guides=steps draws guide 2 as numbered steps with the relation drawn under one of them", async () => {
  await catalogue("steps");
  await rowFor(GUIDE_MEMBER_LINK).click();

  const panel = panelFor(GUIDE_MEMBER_LINK);
  await readsAsSteps(panel, GUIDE_MEMBER_LINK);
  expect(stepsOf(GUIDE_MEMBER_LINK).hasFigure).toBe(true);

  // The relation is the thing being learned, so it is drawn — and it is drawn
  // inside the step that names the order, not floating beside the list.
  const figure = panel.getByRole("img", { name: /landing onto it in order/ });
  await expect.element(figure).toBeVisible();
  const carrying = drawnSteps(panel).filter((step) => step.includes("main"));
  expect(carrying).toHaveLength(1);
});

test("?guides=steps draws guide 1 as steps with no figure and no frame where one would go", async () => {
  await catalogue("steps");
  // Guide 1 is the first, so the panel is already on it: arriving on an empty
  // panel is the one thing the list-and-panel arrangement must never do.
  const panel = panelFor(GUIDE_COMPLETION);
  await readsAsSteps(panel, GUIDE_COMPLETION);

  // There is no honest picture of a rule, so the shape draws none rather than
  // inventing one — and holds no space for the one it is not drawing.
  expect(stepsOf(GUIDE_COMPLETION).hasFigure).toBe(false);
  expect(panel.getByRole("img").query()).toBeNull();
});

test("?guides=figure leads with the drawing on both guides, captions under it", async () => {
  await catalogue("figure");

  // Guide 1, whose subject is a rule: it still leads with a drawing, because
  // that is what the shape means. The drawing is the four rules in four boxes.
  const first = panelFor(GUIDE_COMPLETION);
  const rules = first.getByRole("img", { name: /four landing rules/ });
  await expect.element(rules).toBeVisible();
  await leads(rules, first, GUIDE_COMPLETION);
  // Not a step list wearing a figure: the shape is one or the other.
  expect(first.getByRole("listitem").query()).toBeNull();

  await rowFor(GUIDE_MEMBER_LINK).click();
  const second = panelFor(GUIDE_MEMBER_LINK);
  const landing = second.getByRole("img", { name: /landing onto it in order/ });
  await expect.element(landing).toBeVisible();
  await leads(landing, second, GUIDE_MEMBER_LINK);
});

/** The drawing comes first and every caption follows it. */
async function leads(
  figure: ReturnType<typeof panelFor>,
  where: ReturnType<typeof panelFor>,
  guide: Guide,
): Promise<void> {
  const drawing = figure.element();
  for (const caption of captionsOf(guide)) {
    const words = where.getByText(caption);
    await expect.element(words).toBeVisible();
    const after = drawing.compareDocumentPosition(words.element()) & Node.DOCUMENT_POSITION_FOLLOWING;
    expect(after, `caption is not under the drawing: ${caption}`).toBeTruthy();
  }
}

test("the relation animates once when the guide is opened, and never on a loop", async () => {
  await catalogue("figure");
  await rowFor(GUIDE_MEMBER_LINK).click();
  const figure = panelFor(GUIDE_MEMBER_LINK).getByRole("img", { name: /landing onto it in order/ });
  await expect.element(figure).toBeVisible();

  // Six: a segment and a mark for each of the three members. A loop is what says
  // *still working*, and a guide is not working — `docs/contracts/design-system.md`.
  const running = figure.element().getAnimations({ subtree: true });
  expect(running).toHaveLength(6);
  for (const one of running) {
    expect(one.effect?.getTiming().iterations).toBe(1);
  }
});

test("under prefers-reduced-motion the figure holds still and still reads", async () => {
  // A real list that always matches rather than `{ matches: true }`, the way
  // `packages/screens/src/travel.test.tsx` takes the preference.
  const matchMedia = window.matchMedia.bind(window);
  vi.spyOn(window, "matchMedia").mockImplementation((query) =>
    matchMedia(query === "(prefers-reduced-motion: reduce)" ? "all" : query),
  );

  await catalogue("figure");
  await rowFor(GUIDE_MEMBER_LINK).click();
  const figure = panelFor(GUIDE_MEMBER_LINK).getByRole("img", { name: /landing onto it in order/ });
  await expect.element(figure).toBeVisible();

  expect(figure.element().getAnimations({ subtree: true })).toHaveLength(0);
  // Held still it is the finished branch: three members landed in order, each
  // labelled with the link it carries. Nothing was carried by the movement.
  for (const word of ["1", "2", "3", "Stacked", "Parked", "Waiting on a release", "main"]) {
    await expect.element(figure.getByText(word, { exact: true })).toBeVisible();
  }
});

test("both shapes read in the card the ? opens, not only in the panel", async () => {
  // The Job with members is where guide 2's `?` sits, so this is the card path.
  mount("members/stacked", "steps");
  await onScreen();
  await markFor(GUIDE_MEMBER_LINK).click();

  const card = cardFor(GUIDE_MEMBER_LINK);
  await expect.element(card).toBeVisible();
  await readsAsSteps(card, GUIDE_MEMBER_LINK);
  await expect.element(card.getByRole("img", { name: /landing onto it in order/ })).toBeVisible();
});

test("a figure-first guide fits the card, which is where that shape costs most", async () => {
  mount("members/stacked", "figure");
  await onScreen();
  await markFor(GUIDE_MEMBER_LINK).click();

  const card = cardFor(GUIDE_MEMBER_LINK);
  await expect.element(card).toBeVisible();
  const figure = card.getByRole("img", { name: /landing onto it in order/ });
  await expect.element(figure).toBeVisible();
  // The drawing takes the card's width rather than overflowing it: a figure
  // wider than the layer it is in is the cost being priced, not a bug to keep.
  const inside = figure.element().getBoundingClientRect();
  const layer = card.element().getBoundingClientRect();
  expect(inside.width).toBeGreaterThan(0);
  expect(inside.right).toBeLessThanOrEqual(layer.right);
});

test("the window Bridge draws is prose, because nothing in the app provides a shape", async () => {
  // No shape, which is what `App` mounts with: `apps/desktop/src/renderer/src/main.tsx`
  // provides no `GuideShapeProvider` and the context's default is prose.
  await catalogue();
  const panel = panelFor(GUIDE_COMPLETION);
  const [paragraph] = GUIDE_COMPLETION.body;
  if (paragraph === undefined) throw new Error("guide 1 has no body");

  await expect.element(panel.getByText(paragraph)).toBeVisible();
  expect(panel.getByRole("listitem").query()).toBeNull();
  expect(panel.getByRole("img").query()).toBeNull();
});
