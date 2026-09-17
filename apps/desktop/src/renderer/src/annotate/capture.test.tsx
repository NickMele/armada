import { afterEach, expect, test } from "vitest";

import { capture } from "./capture";

afterEach(() => {
  document.body.innerHTML = "";
});

/** A rail item as `Sidebar` draws one: glyph, label, count, and the binding revealed under Cmd. */
function rail(): Element {
  document.body.innerHTML = `
    <nav>
      <button aria-current="page" class="armada-sidebar__item">
        <svg aria-hidden="true"></svg>
        <span class="armada-sidebar__label">Job Board</span>
        <span class="armada-sidebar__count">22</span>
        <span class="armada-sidebar__kbd"><kbd>⌘</kbd><kbd>2</kbd></span>
      </button>
    </nav>
    <main><p id="target">No jobs. Propose one above.</p></main>`;
  return document.getElementById("target")!;
}

test("a note names the screen by the rail item's label, not its count or binding", () => {
  expect(capture(rail(), new Date("2026-09-17T14:00:00Z")).screen).toBe("Job Board");
});
