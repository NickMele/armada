// The layer in a real window, on the case that broke it: an element as tall as
// the window. The card is `position: fixed`, so the window is the frame, and
// what this asserts is that every edge of the card is inside it — measured,
// because the defect was purely geometric and nothing about the markup showed it.

import { StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import "../styles/index.css";
import { Layer } from "./Layer";
import type { Sink } from "./sink";

/** The owner's window in the screenshot, and a short one. */
const OWNER = { width: 1324, height: 922 };
const SHORT = { width: 1324, height: 700 };

const sink: Sink = {
  via: "dev server",
  list: async () => [],
  save: async () => undefined,
  remove: async () => undefined,
  root: async () => null,
  capture: async () => null,
};

let mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(async () => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  document.querySelectorAll("[data-tall]").forEach((one) => one.remove());
  sessionStorage.removeItem("armada.annotate.on");
  await page.viewport(1440, 900);
});

/** The layer already on, as it comes up after ⌥⌘A and a reload. */
async function annotating(): Promise<void> {
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
  await annotating();
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
