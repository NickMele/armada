// The layer in a real window, on the case that broke it: an element as tall as
// the window. The card is `position: fixed`, so the window is the frame, and
// what this asserts is that every edge of the card is inside it — measured,
// because the defect was purely geometric and nothing about the markup showed it.

import { StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import type { Annotation, AnnotationStatus } from "../../../shared/annotations";
import "../styles/index.css";
import { Layer } from "./Layer";
import type { Sink } from "./sink";

/** The owner's window in the screenshot, and a short one. */
const OWNER = { width: 1324, height: 922 };
const SHORT = { width: 1324, height: 700 };

/** Notes the layer reads, saves back to and deletes from, held in memory. */
function sinkOf(notes: Annotation[] = []): Sink {
  const held = new Map(notes.map((note) => [note.id, note]));
  return {
    via: "dev server",
    list: async () => [...held.values()],
    save: async (note) => {
      held.set(note.id, note);
    },
    remove: async (id) => {
      held.delete(id);
    },
    root: async () => null,
    capture: async () => null,
  };
}

const sink = sinkOf();

let mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(async () => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  document.querySelectorAll("[data-tall], [data-noted]").forEach((one) => one.remove());
  sessionStorage.removeItem("armada.annotate.on");
  await page.viewport(1440, 900);
});

/** The layer already on, as it comes up after ⌥⌘A and a reload. */
async function annotating(sink: Sink): Promise<void> {
  sessionStorage.setItem("armada.annotate.on", "1");
  const host = document.createElement("div");
  host.setAttribute("data-armada-annotate", "");
  document.body.append(host);
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <Layer sink={sink} />
    </StrictMode>,
  );
  mounted.push({ root, host });
  await expect.element(page.getByRole("status")).toBeVisible();
}

/**
 * A Job settings sheet, near enough: fixed, and as tall as the window less a
 * hair. This is the element the owner picked, and the one no side has room beside.
 */
function tallSheet(): HTMLElement {
  const sheet = document.createElement("div");
  sheet.setAttribute("data-tall", "");
  sheet.setAttribute("role", "dialog");
  sheet.setAttribute("aria-label", "Job settings");
  sheet.style.cssText = "position:fixed;top:2%;bottom:2%;left:30%;width:30%";
  document.body.append(sheet);
  return sheet;
}

const rectOf = (locator: ReturnType<typeof page.getByRole>): DOMRect =>
  (locator.element() as HTMLElement).getBoundingClientRect();

/** Inside the window on every edge, to the pixel. */
function insideTheWindow(what: string, rect: DOMRect): void {
  expect(rect.top, `${what}: top edge`).toBeGreaterThanOrEqual(0);
  expect(rect.left, `${what}: leading edge`).toBeGreaterThanOrEqual(0);
  expect(rect.bottom, `${what}: bottom edge`).toBeLessThanOrEqual(window.innerHeight);
  expect(rect.right, `${what}: trailing edge`).toBeLessThanOrEqual(window.innerWidth);
  expect(rect.height, `${what}: drawn at all`).toBeGreaterThan(0);
}

async function cardOnTheTallSheet(size: { width: number; height: number }) {
  await page.viewport(size.width, size.height);
  await annotating(sink);
  tallSheet().click();
  const card = page.getByRole("dialog", { name: "New note" });
  await expect.element(card).toBeVisible();
  // The layer re-places the card off a measurement it takes each frame.
  await new Promise((settled) => requestAnimationFrame(() => requestAnimationFrame(settled)));
  return card;
}

for (const size of [OWNER, SHORT]) {
  test(`a note on an element as tall as a ${size.width}×${size.height} window is drawn whole inside it`, async () => {
    const card = await cardOnTheTallSheet(size);
    const rect = rectOf(card);
    insideTheWindow("card", rect);

    // What the owner could not see: the text box he was typing into, and the
    // two buttons, which were clipped at the top edge of the window.
    for (const name of ["Cancel", "Save note"]) {
      const button = rectOf(card.getByRole("button", { name }));
      insideTheWindow(name, button);
      expect(button.top).toBeGreaterThanOrEqual(rect.top);
      expect(button.bottom).toBeLessThanOrEqual(rect.bottom);
    }
    const box = rectOf(card.getByRole("textbox", { name: "Note" }));
    insideTheWindow("the text box", box);
    expect(box.top).toBeGreaterThanOrEqual(rect.top);
    expect(box.bottom).toBeLessThanOrEqual(rect.bottom);

    // The layer's own bar is pinned over everything; a card under it is on
    // screen and still unreadable.
    expect(rect.bottom).toBeLessThanOrEqual(rectOf(page.getByRole("status")).top);
  });
}

test("a window too short for the card gives it its own scroll rather than an edge to fall off", async () => {
  const card = await cardOnTheTallSheet({ width: 1324, height: 260 });
  const rect = rectOf(card);
  insideTheWindow("card", rect);

  const element = card.element() as HTMLElement;
  expect(element.scrollHeight).toBeGreaterThan(element.clientHeight);
  element.scrollTop = element.scrollHeight;
  expect(rectOf(card.getByRole("button", { name: "Save note" })).bottom).toBeLessThanOrEqual(rect.bottom);

  // The placement settles: a card capped by the window must not flip between
  // two sides frame after frame as it is measured, capped, and measured again.
  await new Promise((settled) => requestAnimationFrame(() => requestAnimationFrame(settled)));
  const again = rectOf(card);
  expect([again.x, again.y, again.width, again.height]).toEqual([rect.x, rect.y, rect.width, rect.height]);
});

// A batch of notes has been fixed and the layer is turned on again. What the
// owner saw was every fixed note still pinned to its screen, numbered, so a new
// note came up as 9 while one was open. A done note is now out of the drawing
// and out of the numbering, and the bar is where it is said how many there are.

/** An element a note points at, found again by the selector the note carries. */
function noted(name: string, top: number): string {
  const element = document.createElement("button");
  element.setAttribute("data-noted", name);
  element.textContent = name;
  // A share of the window, as the tall sheet above is: the pins only have to
  // land clear of one another, and a length literal is off-contract here too.
  element.style.cssText = `position:fixed;left:20%;top:${top}%;width:15%;height:4%`;
  document.body.append(element);
  return `[data-noted="${name}"]`;
}

/** A saved note on that element, written at a time the numbering sorts by. */
function note(name: string, status: AnnotationStatus, at: string, top: number): Annotation {
  return {
    id: `2026091${at}`,
    status,
    text: `${name}: what the owner said`,
    component: "JobRowStacked",
    owners: ["ActiveJobsList", "Board"],
    ownersFrom: "parent",
    selector: noted(name, top),
    element: { tag: "button", text: name, label: null },
    screen: "Job Board",
    layer: null,
    location: "/",
    scenario: null,
    box: { x: 0, y: top, width: 0, height: 0 },
    window: { width: 1440, height: 900 },
    createdAt: `2026-09-1${at}T10:00:00.000Z`,
    updatedAt: `2026-09-1${at}T10:00:00.000Z`,
  };
}

/** Every pin drawn, as a person reads it: its number, or nothing where it has none. */
function pinned(): { name: string; reads: string }[] {
  return page
    .getByRole("button", { name: /^(Note \d+, open|Done note)/ })
    .elements()
    .map((one) => ({ name: one.getAttribute("aria-label") ?? "", reads: one.textContent ?? "" }));
}

/** Three notes, the middle one already fixed by an agent, on three live elements. */
const batch = (): Annotation[] => [
  note("first", "open", "5", 15),
  note("second", "done", "6", 30),
  note("third", "open", "7", 45),
];

/** ⌥⌘A, which is how the layer is put away and brought back. */
async function pressToggle(): Promise<void> {
  window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyA", metaKey: true, altKey: true, bubbles: true }));
  await new Promise((settled) => setTimeout(settled, 0));
}

test("a note marked done draws no pin, and the open ones are numbered 1 to n with no gap", async () => {
  await annotating(sinkOf(batch()));

  await expect.element(page.getByRole("button", { name: /^Note 1, open/ })).toBeVisible();
  expect(pinned()).toEqual([
    { name: "Note 1, open: first: what the owner said", reads: "1" },
    { name: "Note 2, open: third: what the owner said", reads: "2" },
  ]);

  // The count is still said, because the file is still there.
  await expect.element(page.getByRole("status")).toHaveTextContent("2 open, 1 done");
});

test("the bar shows the done notes again, unnumbered, and their cards still work", async () => {
  await annotating(sinkOf(batch()));
  const bar = page.getByRole("status");

  await bar.getByRole("button", { name: "Show done notes" }).click();
  const back = page.getByRole("button", { name: /^Done note/ });
  await expect.element(back).toBeVisible();
  expect(pinned()).toEqual([
    { name: "Note 1, open: first: what the owner said", reads: "1" },
    { name: "Done note: second: what the owner said", reads: "" },
    { name: "Note 2, open: third: what the owner said", reads: "2" },
  ]);

  // The card opens off the pin, and reopening from it puts the note back into
  // the numbering — at 2, where it was written, not at the end.
  await back.click();
  const card = page.getByRole("dialog", { name: "Note" });
  await expect.element(card).toHaveTextContent("second: what the owner said");
  await expect.element(card.getByRole("button", { name: "Delete" })).toBeVisible();
  await card.getByRole("button", { name: "Reopen" }).click();
  await expect.element(page.getByRole("button", { name: /^Note 2, open: second/ })).toBeVisible();
  await expect.element(bar).toHaveTextContent("3 open, 0 done");
});

test("putting the layer away puts the done notes away with it, and looking wrote nothing", async () => {
  const held = sinkOf(batch());
  await annotating(held);
  const bar = page.getByRole("status");

  await bar.getByRole("button", { name: "Show done notes" }).click();
  await expect.element(page.getByRole("button", { name: /^Done note/ })).toBeVisible();
  await bar.getByRole("button", { name: "Hide done notes" }).click();
  expect(page.getByRole("button", { name: /^Done note/ }).elements()).toHaveLength(0);

  // ⌥⌘A off and on again, which is how the notes are read from the files a
  // second time: the layer comes up on the open notes, as it did the first time.
  await pressToggle();
  await expect.element(bar).not.toBeInTheDocument();
  await pressToggle();
  await expect.element(page.getByRole("status")).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Show done notes" })).toBeVisible();
  expect(page.getByRole("button", { name: /^Done note/ }).elements()).toHaveLength(0);

  // Nothing about a note was written by looking: the files say what they said.
  expect((await held.list()).map((one) => one.status)).toEqual(["open", "done", "open"]);
});
