// The guide set, held to the rules the catalogue and the marks both assume.
//
// **Here rather than in `packages/components`.** That package's only test
// project is Storybook, so a `.test.ts` beside the data would never run. This
// is arithmetic over a table, which is what the node project is for — and it
// imports the real set, never a fixture of one.

import { GUIDES, GUIDE_GROUPS, RETIRED_GUIDE_NUMBERS } from "@armada/components";
import { describe, expect, test } from "vitest";

/**
 * Every page under `docs/concepts/`, read by the bundler rather than by node.
 * This package's tests run through Vite with no node types, and a glob is what
 * both halves of that allow — it is resolved at build time, so a page that
 * does not exist is simply not in the set.
 */
const CONCEPTS = new Set(
  Object.keys(
    (import.meta as unknown as { glob: (pattern: string) => Record<string, unknown> }).glob(
      "../../../docs/concepts/*.md",
    ),
  ).map((path) => path.replace("../../../", "")),
);

describe("the guides", () => {
  test("carry a number of their own, and no two share one", () => {
    const numbers = GUIDES.map((guide) => guide.number);
    expect(new Set(numbers).size).toBe(numbers.length);
    for (const number of numbers) expect(number).toBeGreaterThan(0);
  });

  test("never take a retired number back", () => {
    // **A retired guide keeps its number.** 11 explained the second edge on
    // the Workflow canvas; the graph moved to the Plan tab on 25 September
    // 2026, so the guide went and 11 went with it. A new guide that reached
    // for the next free slot would give a reader guide 11 twice.
    for (const retired of RETIRED_GUIDE_NUMBERS) {
      expect(GUIDES.map((guide) => guide.number)).not.toContain(retired);
    }
  });

  test("account for every number ever issued, in use or retired", () => {
    // What makes the catalogue's order and a guide's number two independent
    // facts: a guide filed at the top of a group still took the next number,
    // so nothing a person cited yesterday moved overnight. Every number from
    // one to the highest is one or the other, and nothing is skipped.
    const numbers = GUIDES.map((guide) => guide.number);
    const ceiling = Math.max(...numbers, ...RETIRED_GUIDE_NUMBERS);
    const issued = [...numbers, ...RETIRED_GUIDE_NUMBERS].sort((a, b) => a - b);
    expect(issued).toEqual(Array.from({ length: ceiling }, (_, at) => at + 1));
  });

  test("each explain one piece, and no two claim the same one", () => {
    const pieces = GUIDES.map((guide) => guide.piece);
    expect(new Set(pieces).size).toBe(pieces.length);
  });

  test("are filed under a group the catalogue draws", () => {
    const groups = new Set(GUIDE_GROUPS.map((group) => group.id));
    for (const guide of GUIDES) expect(groups.has(guide.group)).toBe(true);
  });

  test("are grouped rather than interleaved, so the catalogue reads in group order", () => {
    // The catalogue draws group by group. A guide filed out of its run would
    // put a plan guide in the middle of the job ones.
    const order = GUIDE_GROUPS.map((group) => group.id);
    const met = GUIDES.map((guide) => order.indexOf(guide.group));
    expect(met).toEqual([...met].sort((a, b) => a - b));
  });

  test("name a concepts page that exists", () => {
    // **The anti-drift seam, checked as far as a machine can check it.**
    // `docs/concepts/` holds the same knowledge for a reader of the
    // repository. Nothing generates one from the other, so what this catches
    // is a page renamed or deleted out from under a guide.
    for (const guide of GUIDES) {
      if (guide.concept === undefined) continue;
      expect([...CONCEPTS]).toContain(guide.concept);
    }
  });

  test("read as steps: one line each, short enough to say out loud", () => {
    for (const guide of GUIDES) {
      expect(guide.steps.length).toBeGreaterThan(0);
      // A line somebody could say out loud. Past this it is a paragraph that
      // has not been split yet.
      for (const line of guide.steps) expect(line.length).toBeLessThan(160);
    }
  });

  test("hang a figure under a step that exists", () => {
    for (const guide of GUIDES) {
      if (guide.figure === undefined) continue;
      expect(guide.figure.at).toBeGreaterThan(0);
      expect(guide.figure.at).toBeLessThanOrEqual(guide.steps.length);
    }
  });

  test("title with a question a person would type", () => {
    // The owner, 25 September 2026, reversing the rule an agent wrote: a
    // person arrives at the catalogue holding a question, and a row that names
    // a part of the app asks them to translate before they can find it.
    for (const guide of GUIDES) {
      expect(guide.title).toMatch(/\?$/);
      expect(guide.title.slice(1)).not.toMatch(/^[A-Z]/);
    }
  });

  test("write the guides the way Armada writes", () => {
    // The humanizer skill, applied to the whole set: no em dash, no en dash,
    // no triple-dot trail. The prose rules ban a mid-sentence aside and these
    // are how one gets written.
    for (const guide of GUIDES) {
      for (const line of guide.steps) {
        expect(line, `guide ${guide.number}`).not.toMatch(/[—–]/);
        expect(line, `guide ${guide.number}`).not.toMatch(/\.\.\./);
      }
    }
  });
});
