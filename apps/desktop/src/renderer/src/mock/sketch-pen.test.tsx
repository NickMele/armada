// The pen on the sketch pad, through `App`. The owner's note of 2026-09-23:
// "The designs had this with more than just a box. We could do a free draw as
// well" — the pad drew boxes and joins and nothing a hand could make.
//
// **Four claims, and only one of them is about drawing.** That a drag leaves a
// line is the pad's own and is asserted in its story. What needs the app is
// what the pad cannot hold on its own: a line kept across a switch to Write, a
// line reopened with the sketch it belongs to, and a line taken back — each
// one a trip through the draft `DispatchJob` keeps.
//
// **A line drawn by hand is not a join.** The two are read by different names,
// so a test that stopped telling them apart would fail here rather than later.

import { expect, test, describe } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** What one line drawn by hand is called, to somebody who cannot see it. */
const A_HAND_LINE = "Drawn by hand";

/** What the pad is, and how the graph itself is found. */
const PAD = "The picture attached to this request";

/**
 * One drag across the pad, in pointer events.
 *
 * **Dispatched by hand rather than through the browser's pointer helper.** The
 * layer that catches the pen carries no role — a surface to draw on is not a
 * control — so there is nothing to locate it by; `elementFromPoint` finds what
 * a real pointer would have hit. The move and the lift go to the window, which
 * is where the pad listens, so a line that leaves the pad still ends.
 */
function drawOnThePad(): void {
  const at = page.getByLabelText(PAD).element().getBoundingClientRect();
  const from = { x: at.x + at.width / 3, y: at.y + at.height / 3 };
  const pen = document.elementFromPoint(from.x, from.y);
  if (pen === null) throw new Error("Nothing is under the pen.");
  const event = (kind: string, x: number, y: number) =>
    new PointerEvent(kind, { clientX: x, clientY: y, bubbles: true, button: 0 });

  pen.dispatchEvent(event("pointerdown", from.x, from.y));
  for (let step = 1; step <= 8; step += 1) {
    window.dispatchEvent(event("pointermove", from.x + step * 12, from.y + step * 6));
  }
  window.dispatchEvent(event("pointerup", from.x + 96, from.y + 48));
}

describe("the pen", () => {
  test(
    "arc/dispatch-sketch: a line drawn by hand comes back with the boxes, because the pad " +
      "reopens what was drawn and not a picture of it",
    async () => {
      mount("arc/dispatch-sketch");
      await page.getByRole("tab", { name: "Sketch" }).click();

      await expect.element(page.getByRole("group", { name: "Box: Drones 1 of 2" })).toBeVisible();
      expect(page.getByRole("img", { name: A_HAND_LINE }).elements()).toHaveLength(1);
      expect(page.getByRole("group", { name: /^A line from / }).elements()).toHaveLength(2);
    },
  );

  test(
    "arc/dispatch-typing: a line drawn on an empty pad attaches the sketch, survives a trip " +
      "through Write, and comes back off with Undo",
    async () => {
      mount("arc/dispatch-typing");
      await page.getByRole("tab", { name: "Sketch" }).click();
      await expect.element(page.getByRole("button", { name: "Draw" })).toBeVisible();
      expect(page.getByRole("img", { name: A_HAND_LINE }).elements()).toHaveLength(0);

      await page.getByRole("button", { name: "Draw" }).click();
      drawOnThePad();
      await expect.element(page.getByRole("img", { name: A_HAND_LINE })).toBeInTheDocument();
      // An empty pad attaches nothing, so the chip is the picture counting as
      // one on the strength of the line alone, with no box on the pad at all.
      await expect.element(page.getByText("sketch 1")).toBeVisible();

      await page.getByRole("tab", { name: "Write" }).click();
      await expect.element(page.getByRole("textbox", { name: "Request" })).toBeVisible();
      await page.getByRole("tab", { name: "Sketch" }).click();
      await expect.element(page.getByRole("img", { name: A_HAND_LINE })).toBeInTheDocument();

      await page.getByRole("button", { name: "Undo" }).click();
      await expect.element(page.getByRole("img", { name: A_HAND_LINE })).not.toBeInTheDocument();
      await expect.element(page.getByText("sketch 1")).not.toBeInTheDocument();
    },
  );
});
