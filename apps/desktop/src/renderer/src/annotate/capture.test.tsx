import { afterEach, expect, test } from "vitest";

import { SOURCE_ATTRIBUTE } from "../../../shared/annotations";
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

/** A row as the build stamps one: the JSX that drew each element, on each element. */
function stamped(): Element {
  document.body.innerHTML = `
    <main ${SOURCE_ATTRIBUTE}="packages/screens/src/Board.tsx:31">
      <div ${SOURCE_ATTRIBUTE}="packages/screens/src/JobRow.tsx:88">
        <span id="named">42-add-an-illustration</span>
        <b id="deep" ${SOURCE_ATTRIBUTE}="packages/screens/src/JobRow.tsx:94">Waiting on you</b>
      </div>
    </main>`;
  return document.getElementById("named")!;
}

const AT = new Date("2026-09-22T14:00:00Z");

test("a note carries the JSX that drew the element it was pinned on", () => {
  stamped();
  expect(capture(document.getElementById("deep")!, AT).source).toEqual({
    file: "packages/screens/src/JobRow.tsx",
    line: 94,
  });
});

test("an element the build did not stamp takes the nearest stamped ancestor, not the outermost", () => {
  expect(capture(stamped(), AT).source).toEqual({ file: "packages/screens/src/JobRow.tsx", line: 88 });
});

test("a note on an element with nothing stamped above it carries no source at all", () => {
  document.body.innerHTML = `<main><p id="target">No jobs. Propose one above.</p></main>`;
  const note = capture(document.getElementById("target")!, AT);
  expect(note.source).toBeUndefined();
  expect("source" in note).toBe(false);
});

test("a stamp that is not a file and a line is read as none rather than guessed at", () => {
  for (const value of ["packages/screens/src/Board.tsx", "packages/screens/src/Board.tsx:none", ":12"]) {
    document.body.innerHTML = `<div ${SOURCE_ATTRIBUTE}="${value}"><p id="target">Waiting on you</p></div>`;
    expect(capture(document.getElementById("target")!, AT).source, value).toBeUndefined();
  }
});
