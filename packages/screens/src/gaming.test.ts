// The gaming check's reading: where a flag's lines are, and what the rail and
// the panel say about it. #1079.

import { describe, expect, it } from "vitest";
import type { DeclaredJudge, Diff, Flagged } from "@armada/protocol";

import { freshStep } from "./fixtures/build/base";
import {
  declaredPatterns,
  flagsOf,
  gamingStands,
  gamingSummary,
  hunkFor,
  sentBackWords,
} from "./gaming";

const JOB = "01M22TYSAE0023MADDP5ZQEYGW";
const FILE = "packages/settings/test/useColumnSelectors.test.ts";

/** Two hunks: an assertion replaced (line 40 of the post-image), and two removed. */
const PATCH = [
  `diff --git a/${FILE} b/${FILE}`,
  `--- a/${FILE}`,
  `+++ b/${FILE}`,
  '@@ -38,5 +38,5 @@ describe("useColumnSelectors", () => {',
  '   it("keeps hidden columns out of the visible set", () => {',
  "     const visible = selectVisible(state);",
  '-    expect(visible).toEqual(["name", "status", "owner"]);',
  "+    expect(visible.length).toBeGreaterThan(0);",
  "   });",
  "@@ -52,6 +52,4 @@",
  '   it("drops a column that was hidden", () => {',
  '-    expect(next.hidden).toContain("owner");',
  '-    expect(selectVisible(next)).not.toContain("owner");',
  "     expect(next.version).toBe(state.version + 1);",
  "   });",
].join("\n");

function diff(jobId = JOB): Diff {
  return { state: "read", jobId, work: { files: [], plan_declared: true, patch: PATCH } };
}

function flag(over: Partial<Flagged> = {}): Flagged {
  return { attempt: 1, pattern: "assertion_weakened", cited: "", at: { file: FILE }, ...over };
}

describe("where a flag's lines are", () => {
  it("finds a flag on an added line by that line, counted off the hunk header", () => {
    const found = hunkFor(flag({ at: { file: FILE, line: 40 } }), diff(), JOB);
    expect(found?.lines[0]?.text).toMatch(/^@@ -38,5 \+38,5 @@/);
    expect(found?.lines.some((line) => line.text.includes("toBeGreaterThan"))).toBe(true);
  });

  it("finds a flag on a removed line by the words it quotes, since it has no line", () => {
    const cited = '`expect(selectVisible(next)).not.toContain("owner")` was taken out, and nothing replaces it.';
    const found = hunkFor(flag({ cited }), diff(), JOB);
    expect(found?.lines[0]?.text).toBe("@@ -52,6 +52,4 @@");
  });

  it("draws nothing where no line carries the words, rather than a hunk near them", () => {
    expect(hunkFor(flag({ cited: "`assert_eq!(routes.len(), served.len())`" }), diff(), JOB)).toBeUndefined();
  });

  it("draws nothing for a line no hunk holds", () => {
    expect(hunkFor(flag({ at: { file: FILE, line: 120 } }), diff(), JOB)).toBeUndefined();
  });

  it("draws nothing from another Job's patch", () => {
    expect(hunkFor(flag({ at: { file: FILE, line: 40 } }), diff("another"), JOB)).toBeUndefined();
  });
});

describe("what the rail and the panel say", () => {
  const held = flag();
  const cleared: Flagged = { ...flag(), cleared: { why: "a doc comment, not a check" } };

  it("splits the flags that hold the step from those a second reading cleared", () => {
    const read = flagsOf(freshStep("implement", "Implement", 2), [held, cleared]);
    expect(read.held).toEqual([held]);
    expect(read.cleared).toEqual([cleared]);
  });

  it("says on the rail that the gaming check stopped the step", () => {
    expect(gamingStands({ held: [held], cleared: [] }, true)).toBe("1 flag · stopped here");
    expect(gamingStands({ held: [], cleared: [cleared, cleared] }, false)).toBe("2 flags cleared");
    expect(gamingStands({ held: [], cleared: [] }, false)).toBeUndefined();
  });

  it("says in the panel what it flagged, and never a count of patterns it cannot see", () => {
    expect(gamingSummary({ held: [held], cleared: [] }, true, true)).toBe("1 flagged · stopped the step");
    expect(gamingSummary({ held: [], cleared: [cleared] }, true, false)).toBe("1 cleared");
    expect(gamingSummary({ held: [], cleared: [] }, true, false)).toBe("nothing flagged");
    expect(gamingSummary({ held: [], cleared: [] }, false, false)).toBe("not reached");
  });

  it("counts flagged patterns against every declared one, where Fleet names them", () => {
    const declared = ["assertion_weakened", "test_skipped", "test_deleted", "tautological_test", "check_config_edited", "test_scope_narrowed"];
    expect(gamingSummary({ held: [held, held], cleared: [] }, true, true, declared)).toBe(
      "1 of 6 flagged · stopped the step",
    );
    expect(gamingSummary({ held: [], cleared: [] }, true, false, declared)).toBe("0 of 6 flagged");
    expect(gamingSummary({ held: [], cleared: [] }, false, false, declared)).toBe("not reached");
  });

  it("reads the declared patterns in order, and nothing from a Fleet that does not name them", () => {
    const naming: DeclaredJudge = {
      criteria: 0,
      gaming_check: true,
      gaming_patterns: ["test_deleted", "assertion_weakened"],
    };
    expect(declaredPatterns({ ...freshStep("implement", "Implement", 2), judge_checks: [naming] })).toEqual([
      "test_deleted",
      "assertion_weakened",
    ]);
    const silent = { ...freshStep("implement", "Implement", 2), judge_checks: [{ criteria: 0, gaming_check: true }] };
    expect(declaredPatterns(silent)).toBeUndefined();
  });
});

describe("the words Send it back redirects a Drone with", () => {
  const cited = flag({ cited: "`expect(a).toBe(b)` was removed", asked: "Is that assertion made nowhere else?" });

  it("carry the pattern's headline, what was cited, what was asked, and the note", () => {
    const words = sentBackWords([cited], "  Put it back  ");
    expect(words).toContain("A test may have been weakened to make this step pass.");
    expect(words).toContain("It cited: `expect(a).toBe(b)` was removed");
    expect(words).toContain("It asked: Is that assertion made nowhere else?");
    expect(words).toContain("The person's note: Put it back");
  });

  it("say nothing about a note nobody wrote", () => {
    expect(sentBackWords([cited], undefined)).not.toContain("note");
  });
});
