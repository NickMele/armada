import { describe, expect, it } from "vitest";
import type { DiffLine } from "@armada/components";
import { hunkOf } from "./view";

const LINES: DiffLine[] = [
  { kind: "hunk", text: "@@ -14,6 +14,9 @@ export function columns() {" },
  { kind: "context", text: " const order = []" },
  { kind: "added", text: "+import { selectColumnOrder } from './selectors/columns'" },
  { kind: "hunk", text: "@@ -40,3 +43,2 @@" },
  { kind: "removed", text: "-  return store.columns" },
];

describe("hunkOf", () => {
  it("takes a hunk from its header up to the next one", () => {
    expect(hunkOf(LINES, "@@ -14,6 +14,9 @@")).toEqual(LINES.slice(0, 3));
  });

  it("takes the last hunk to the end of the file", () => {
    expect(hunkOf(LINES, "@@ -40,3 +43,2 @@")).toEqual(LINES.slice(3));
  });

  it("finds nothing for a header the patch no longer holds", () => {
    expect(hunkOf(LINES, "@@ -99,1 +99,1 @@")).toBeNull();
  });

  it("finds nothing for text that is not a header", () => {
    expect(hunkOf(LINES, "const order")).toBeNull();
  });
});
