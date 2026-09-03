// What the held list divides into, and what a confirmation says it costs.
//
// **Unit tests and not a `play`**, because every function here computes: a
// browser mounting a screen to check a partition is a unit test paying a
// browser's price. What earns a `play` is behaviour a rendering cannot show,
// and that is in `Worktrees.test.tsx` beside it.
//
// The case this file exists for is
// [`the_cost_of_a_reclaim_separates_what_ends_from_what_survives`] — that a
// worktree held for two reasons has one of them destroyed by the act and the
// other left exactly where it is, and that the confirmation is built to say
// both rather than to average them into one warning.

import { expect, test } from "vitest";
import type { WorktreeHeld } from "@armada/protocol";

import {
  confirmOpening,
  confirmTitle,
  decides,
  divided,
  filesDestroyed,
  losing,
  sitting,
} from "./held";

/** The shape fleet answers with, in one place so no case drifts from it. */
function held(over: Partial<WorktreeHeld> = {}): WorktreeHeld {
  return {
    job_id: "01JOB0001",
    job_title: "Port the settings selectors",
    status: "completed_success",
    last_moved_at: "2026-08-30T09:14:00Z",
    path: "/Users/user/armada/.armada/worktrees/01JOB0001",
    branch: "armada/01JOB0001",
    held: [],
    ...over,
  };
}

const UNMERGED = {
  why: "unmerged",
  base: "main",
  commits: 3,
  tip: "9f1c2ab84d5e",
} as const;

test("a worktree nothing holds is fleet's own, not a person's decision", () => {
  const groups = divided([held()]);

  expect(groups.automatic.map((one) => one.job_id)).toEqual(["01JOB0001"]);
  expect(groups.deciding).toEqual([]);
  expect(groups.waiting).toEqual([]);
});

test("a job that has not ended is drawn and never offered", () => {
  const running = held({
    job_id: "01JOB0002",
    held: [{ why: "not_terminal", status: "running" }],
  });

  const groups = divided([running]);

  // Fleet refuses the act on a status that is not terminal, so a checkbox on
  // this row would be a control whose only outcome is a 409.
  expect(groups.waiting.map((one) => one.job_id)).toEqual(["01JOB0002"]);
  expect(groups.deciding).toEqual([]);
});

test("the three groups keep fleet's order inside each of them", () => {
  const groups = divided([
    held({ job_id: "a", held: [UNMERGED] }),
    held({ job_id: "b" }),
    held({ job_id: "c", held: [UNMERGED] }),
    held({ job_id: "d", held: [{ why: "not_terminal", status: "running" }] }),
    held({ job_id: "e" }),
  ]);

  expect(groups.deciding.map((one) => one.job_id)).toEqual(["a", "c"]);
  expect(groups.automatic.map((one) => one.job_id)).toEqual(["b", "e"]);
  expect(groups.waiting.map((one) => one.job_id)).toEqual(["d"]);
});

/**
 * **The case the module exists for.** One worktree, two reasons, and the act
 * treats them in opposite directions: the branch survives with every commit on
 * it, and the two loose files do not survive at all.
 */
test("the cost of a reclaim separates what ends from what survives", () => {
  const cost = losing([
    held({
      held: [UNMERGED, { why: "uncommitted", files: ["src/log.rs", "notes.md"] }],
    }),
  ]);

  expect(cost.checkouts).toBe(1);
  expect(filesDestroyed(cost)).toBe(2);
  expect(cost.destroying[0].files).toEqual(["src/log.rs", "notes.md"]);
  // Not a loss, and the confirmation says so in those words: there is no force
  // on this seam, so the branch is kept and the commits stay reachable.
  expect(cost.keeping[0]).toEqual({
    jobId: "01JOB0001",
    title: "Port the settings selectors",
    branch: "armada/01JOB0001",
    commits: 3,
    tip: "9f1c2ab84d5e",
  });
});

test("an unmerged branch alone costs nothing, and the sentence for that exists", () => {
  const cost = losing([held({ held: [UNMERGED] })]);

  expect(cost.destroying).toEqual([]);
  expect(filesDestroyed(cost)).toBe(0);
  expect(cost.keeping).toHaveLength(1);
});

test("the confirmation names the act and what survives it, never a byte count", () => {
  const opening = confirmOpening(losing([held({ held: [UNMERGED] }), held({ job_id: "b" })]));

  expect(confirmTitle(1)).toBe("Reclaim this worktree?");
  expect(confirmTitle(2)).toBe("Reclaim 2 worktrees?");
  expect(opening).toContain("2 checkouts are removed");
  expect(opening).toContain("stay on the board");
  // Bytes are not the decision, and no arithmetic here produces one.
  expect(opening).not.toMatch(/byte|MB|GB|disk space/i);
});

/**
 * Where several reasons apply, the row is summarised by the one that ends
 * something. A row summarised by its unmerged branch would tell a person the
 * safe half of a decision that also has an unsafe half.
 */
test("uncommitted work decides a row that also has an unmerged branch", () => {
  const reason = decides(
    held({ held: [UNMERGED, { why: "uncommitted", files: ["src/log.rs"] }] }),
  );

  expect(reason?.why).toBe("uncommitted");
});

test("a row with nothing holding it has no deciding reason to draw", () => {
  expect(decides(held())).toBeNull();
});

/** A fixed instant, so an age is arithmetic rather than a race with the wall. */
const NOW = Date.parse("2026-09-03T12:00:00Z");

test("an age is coarse, and reads in the unit a person decides in", () => {
  expect(sitting("2026-09-03T11:59:40Z", NOW)).toBe("under a minute");
  expect(sitting("2026-09-03T11:59:00Z", NOW)).toBe("1 minute");
  expect(sitting("2026-09-03T11:38:00Z", NOW)).toBe("22 minutes");
  expect(sitting("2026-09-03T11:00:00Z", NOW)).toBe("1 hour");
  expect(sitting("2026-09-03T04:00:00Z", NOW)).toBe("8 hours");
  expect(sitting("2026-09-02T12:00:00Z", NOW)).toBe("1 day");
  expect(sitting("2026-08-30T09:14:00Z", NOW)).toBe("4 days");
});

/**
 * It rounds down and never up. `last_moved_at` is already a floor — armada
 * moved the job then, and the files were written at or before it — so rounding
 * up would turn a floor into a claim.
 */
test("an age rounds down, because the stamp it is measured from is a floor", () => {
  // Twenty-three and a half hours is not "1 day".
  expect(sitting("2026-09-02T12:30:00Z", NOW)).toBe("23 hours");
  // And three days and twenty-two hours is not "4 days".
  expect(sitting("2026-08-30T14:00:00Z", NOW)).toBe("3 days");
});

/**
 * A stamp that will not parse says nothing rather than showing an age measured
 * from zero — the convention `instant` sets. A stamp in the future is the same
 * refusal: a clock disagreeing is not an age.
 */
test("an unreadable or future stamp draws no age at all", () => {
  expect(sitting("not a date", NOW)).toBeNull();
  expect(sitting("2026-09-04T12:00:00Z", NOW)).toBeNull();
});

/** The confirmation carries the stamp per job, so it can say it row by row. */
test("what is destroyed carries the stamp it is read against", () => {
  const cost = losing([
    held({ held: [{ why: "uncommitted", files: ["src/log.rs"] }] }),
  ]);

  expect(cost.destroying[0].lastMovedAt).toBe("2026-08-30T09:14:00Z");
  expect(sitting(cost.destroying[0].lastMovedAt, NOW)).toBe("4 days");
});
