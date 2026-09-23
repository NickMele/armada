// The guide set, held to the rules the catalogue and the marks both assume.
//
// **Here rather than in `packages/components`.** That package's only test
// project is Storybook, so a `.test.ts` beside the data would never run. This
// is arithmetic over a table, which is what the node project is for — and it
// imports the real set, never a fixture of one.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { GUIDES, GUIDE_GROUPS } from "@armada/components";
import { describe, expect, test } from "vitest";

const ROOT = resolve(import.meta.dirname, "../../..");

describe("the guides", () => {
  test("are numbered from one, in order, with nothing repeated", () => {
    expect(GUIDES.map((guide) => guide.number)).toEqual(GUIDES.map((_, at) => at + 1));
  });

  test("each explain one piece, and no two claim the same one", () => {
    const pieces = GUIDES.map((guide) => guide.piece);
    expect(new Set(pieces).size).toBe(pieces.length);
  });

  test("are filed under a group the catalogue draws", () => {
    const groups = new Set(GUIDE_GROUPS.map((group) => group.id));
    for (const guide of GUIDES) expect(groups.has(guide.group)).toBe(true);
  });

  test("are grouped rather than interleaved, so the catalogue reads in number order", () => {
    // The catalogue draws group by group and numbers inside each. A guide
    // filed out of its run would draw 4 after 6, which is a numbered list
    // nobody can read down.
    const order = GUIDE_GROUPS.map((group) => group.id);
    const met = GUIDES.map((guide) => order.indexOf(guide.group));
    expect(met).toEqual([...met].sort((a, b) => a - b));
  });

  test("name a concepts page that exists", () => {
    // **The anti-drift seam, checked as far as a machine can check it.**
    // `docs/concepts/` holds the same knowledge for a reader of the
    // repository. Nothing generates one from the other, so what this catches
    // is a page renamed or deleted out from under a guide — the divergence
    // itself is caught by reading the two together, which is what the sweep
    // in #1602 is.
    for (const guide of GUIDES) {
      if (guide.concept === undefined) continue;
      expect(() => readFileSync(resolve(ROOT, guide.concept as string))).not.toThrow();
    }
  });

  test("say something, in paragraphs short enough to read on a card", () => {
    for (const guide of GUIDES) {
      expect(guide.body.length).toBeGreaterThan(0);
      for (const paragraph of guide.body) expect(paragraph.length).toBeLessThan(400);
    }
  });

  test("title in sentence case, with no trailing period and no question", () => {
    // The copy rules the contract fixes: sentence case everywhere, and a title
    // names the thing rather than asking about it.
    for (const guide of GUIDES) {
      expect(guide.title).not.toMatch(/[.?]$/);
      expect(guide.title.slice(1)).not.toMatch(/^[A-Z]/);
    }
  });
});
