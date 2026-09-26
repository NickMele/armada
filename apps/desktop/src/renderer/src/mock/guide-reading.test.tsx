// How a guide reads, through `App`. **One shape**: the numbered steps, with
// the drawing of the real app under the line it belongs to. The owner chose
// both together on 26 September 2026, out of three that were built for him to
// compare, and the other two are gone.
//
// Storybook cannot make these claims. The catalogue's own list, a `?` card
// drawing the same shape, and a retired number staying retired are all facts
// about the window rather than about a component.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import {
  GUIDES,
  GUIDE_COMPLETION,
  GUIDE_MEMBER_LINK,
  RETIRED_GUIDE_NUMBERS,
} from "@armada/components";
import type { Guide } from "@armada/components";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The scenario the catalogue is opened from, and the one in the pull request's URLs. */
const SCENARIO = "arc/executing-concurrent";

/** The catalogue, reached the way a person reaches it: by the rail. */
async function catalogue(): Promise<void> {
  mount(SCENARIO);
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

/** Every numbered step drawn in one region, as its text. */
const drawnSteps = (where: ReturnType<typeof panelFor>) =>
  where
    .getByRole("listitem")
    .elements()
    .map((one) => one.textContent ?? "");

/**
 * **Numbered, one line each, and every line is the guide's own.** Read off the
 * drawing rather than off the data twice: a guide that drew its lines in the
 * wrong order, or dropped one, fails here.
 */
async function readsAsSteps(where: ReturnType<typeof panelFor>, guide: Guide): Promise<void> {
  await expect.poll(() => drawnSteps(where).length).toBe(guide.steps.length);
  const drawn = drawnSteps(where);
  for (const [index, line] of guide.steps.entries()) {
    const step = drawn[index] ?? "";
    expect(step.startsWith(String(index + 1)), `step ${index + 1} is not numbered: ${step}`).toBe(true);
    expect(step).toContain(line);
  }
}

test("every guide in the catalogue is in the list, and the one open reads as steps", async () => {
  await catalogue();

  // The list is the claim: a guide is reachable rather than already on screen.
  for (const guide of GUIDES) {
    await expect.element(rowFor(guide)).toBeVisible();
  }

  // The catalogue opens on its first guide: arriving on an empty panel is the
  // one thing the list-and-panel arrangement must never do. First is *What is
  // a job?*, which is the reading order rather than the lowest number.
  const [first] = GUIDES;
  if (first === undefined) throw new Error("no guides to draw");
  await readsAsSteps(panelFor(first), first);
});

test("a guide with a relation draws it under the step that names it, from the real components", async () => {
  await catalogue();
  await rowFor(GUIDE_MEMBER_LINK).click();

  const panel = panelFor(GUIDE_MEMBER_LINK);
  await readsAsSteps(panel, GUIDE_MEMBER_LINK);

  const figure = panel.getByRole("img", { name: /landing in order/ });
  await expect.element(figure).toBeVisible();

  // Inside the step that names the order, not floating beside the list.
  const carrying = drawnSteps(panel).filter((step) => step.includes("Stacked on the one before it."));
  expect(carrying).toHaveLength(1);

  // It is `JobMembers` itself. The link sentences are that component's own
  // fixed copy, so a change to them changes this guide — which is the binding
  // the owner took over a diagram that would have drifted in silence.
  for (const said of [
    "Stacked on the one before it.",
    "Waits on what the one before it publishes.",
  ]) {
    await expect.element(figure.getByText(said, { exact: true })).toBeVisible();
  }

  // Nothing in a figure is a control: the drawing is hidden from the
  // accessibility tree, and a focus stop inside it would be unreachable.
  expect(figure.element().querySelectorAll("button, a, input, [tabindex]")).toHaveLength(0);
});

test("a guide with no relation draws no figure, and holds no frame where one would go", async () => {
  await catalogue();
  await rowFor(GUIDE_COMPLETION).click();
  const panel = panelFor(GUIDE_COMPLETION);
  await readsAsSteps(panel, GUIDE_COMPLETION);

  // There is no honest picture of a rule, so the guide draws none rather than
  // inventing one, and holds no space for the one it is not drawing.
  expect(panel.getByRole("img").query()).toBeNull();
  expect(GUIDE_COMPLETION.figure).toBeUndefined();
});

test("the drawing animates once when the guide is opened, and never on a loop", async () => {
  await catalogue();
  await rowFor(GUIDE_MEMBER_LINK).click();
  const figure = panelFor(GUIDE_MEMBER_LINK).getByRole("img", { name: /landing in order/ });
  await expect.element(figure).toBeVisible();

  // A loop is what says *still working*, and a guide is not working —
  // `docs/contracts/design-system.md`.
  const running = figure.element().getAnimations({ subtree: true });
  expect(running.length).toBeGreaterThan(0);
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

  await catalogue();
  await rowFor(GUIDE_MEMBER_LINK).click();
  const figure = panelFor(GUIDE_MEMBER_LINK).getByRole("img", { name: /landing in order/ });
  await expect.element(figure).toBeVisible();

  expect(figure.element().getAnimations({ subtree: true })).toHaveLength(0);
  // Held still it is the finished list: three members on their rail, in order,
  // each saying which link it carries. Nothing was carried by the movement.
  for (const word of ["1", "2", "3"]) {
    await expect.element(figure.getByText(word, { exact: true })).toBeVisible();
  }
  await expect.element(figure.getByText("Stacked on the one before it.")).toBeVisible();
});

test("the card a ? opens draws the same shape, figure and all", async () => {
  // The Job with members is where guide 2's `?` sits, so this is the card path.
  mount("members/stacked");
  await onScreen();
  await markFor(GUIDE_MEMBER_LINK).click();

  const card = cardFor(GUIDE_MEMBER_LINK);
  await expect.element(card).toBeVisible();
  await readsAsSteps(card, GUIDE_MEMBER_LINK);

  const figure = card.getByRole("img", { name: /landing in order/ });
  await expect.element(figure).toBeVisible();
  // The drawing takes the card's width rather than overflowing it: a figure
  // wider than the layer it is in is a guide a person has to scroll sideways.
  const inside = figure.element().getBoundingClientRect();
  const layer = card.element().getBoundingClientRect();
  expect(inside.width).toBeGreaterThan(0);
  expect(inside.right).toBeLessThanOrEqual(layer.right);
});

test("a retired number is gone from the catalogue and is never handed out again", async () => {
  await catalogue();

  // Guide 11 explained the second edge on the Workflow canvas. The plan's
  // graph moved to the Plan tab on 25 September 2026, so it was retired rather
  // than rewritten, and the five guides added took numbers above the highest.
  expect(RETIRED_GUIDE_NUMBERS).toContain(11);
  for (const retired of RETIRED_GUIDE_NUMBERS) {
    expect(GUIDES.map((guide) => guide.number)).not.toContain(retired);
    const row = page.getByRole("button", { name: new RegExp(`^Guide ${retired},`) });
    expect(row.elements()).toHaveLength(0);
  }
});
