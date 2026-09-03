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

import { confirmOpening, confirmTitle, decides, divided, filesDestroyed, losing } from "./held";

/** The shape fleet answers with, in one place so no case drifts from it. */
function held(over: Partial<WorktreeHeld> = {}): WorktreeHeld {
  return {
    job_id: "01JOB0001",
    job_title: "Port the settings selectors",
    status: "completed_success",
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
