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
import type { JobSummary, WorktreeHeld } from "@armada/protocol";
import type { RowChoice } from "@armada/components";

import {
  choiceOf,
  chosenRows,
  confirmOpening,
  confirmTitle,
  decides,
  divided,
  filesDestroyed,
  NO_CHOICE,
  namedByHandle,
  offeredOn,
  planned,
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
    on_disk: true,
    ...over,
  };
}

/** One row, chosen for the acts named — the rest read `false`. */
function choosing(over: Partial<RowChoice>): RowChoice {
  return { ...NO_CHOICE, ...over };
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
  const row = held({
    held: [UNMERGED, { why: "uncommitted", files: ["src/log.rs", "notes.md"] }],
  });
  const cost = planned([row], { [row.job_id]: choosing({ removeCheckout: true }) });

  expect(cost.checkouts).toBe(1);
  expect(filesDestroyed(cost)).toBe(2);
  expect(cost.destroying[0]!.files).toEqual(["src/log.rs", "notes.md"]);
  // Not a loss, and the confirmation says so in those words: nothing chose to
  // delete the branch, so it is kept and the commits stay reachable.
  expect(cost.keeping[0]).toEqual({
    jobId: "01JOB0001",
    title: "Port the settings selectors",
    branch: "armada/01JOB0001",
    commits: 3,
    tip: "9f1c2ab84d5e",
  });
  expect(cost.deletingBranches).toEqual([]);
});

test("an unmerged branch alone costs nothing, and the sentence for that exists", () => {
  const row = held({ held: [UNMERGED] });
  const cost = planned([row], { [row.job_id]: choosing({ removeCheckout: true }) });

  expect(cost.destroying).toEqual([]);
  expect(filesDestroyed(cost)).toBe(0);
  expect(cost.keeping).toHaveLength(1);
});

/**
 * **A branch chosen for deletion carries its commit count and its tip**, the
 * two facts the confirmation names per rule 4 — never folded into `keeping`.
 */
test("a branch chosen for deletion is named with its commit count and its tip", () => {
  const row = held({ held: [UNMERGED] });
  const cost = planned([row], { [row.job_id]: choosing({ deleteBranch: true }) });

  expect(cost.keeping).toEqual([]);
  expect(cost.deletingBranches).toEqual([
    {
      jobId: "01JOB0001",
      title: "Port the settings selectors",
      branch: "armada/01JOB0001",
      commits: 3,
      tip: "9f1c2ab84d5e",
    },
  ]);
});

test("a record chosen to be forgotten is named on the confirmation", () => {
  const row = held({ held: [] });
  const cost = planned([row], { [row.job_id]: choosing({ forget: true }) });

  expect(cost.forgetting).toEqual([{ jobId: "01JOB0001", title: "Port the settings selectors" }]);
});

test("the confirmation names the act and what survives it, never a byte count", () => {
  const rowA = held({ held: [UNMERGED] });
  const rowB = held({ job_id: "b" });
  const opening = confirmOpening(
    planned(
      [rowA, rowB],
      {
        [rowA.job_id]: choosing({ removeCheckout: true }),
        [rowB.job_id]: choosing({ removeCheckout: true }),
      },
    ),
  );

  expect(confirmTitle(1)).toBe("Clean up this row?");
  expect(confirmTitle(2)).toBe("Clean up 2 rows?");
  expect(opening).toContain("2 checkouts are removed");
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
  const row = held({ held: [{ why: "uncommitted", files: ["src/log.rs"] }] });
  const cost = planned([row], { [row.job_id]: choosing({ removeCheckout: true }) });

  expect(cost.destroying[0]!.lastMovedAt).toBe("2026-08-30T09:14:00Z");
  expect(sitting(cost.destroying[0]!.lastMovedAt, NOW)).toBe("4 days");
});

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01BLOCKER00000000000000000",
    handle: "9-fix-the-retry-ceiling",
    title: "Fix the retry ceiling",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-30T09:00:00Z",
    ...over,
  };
}

test("a depended_on reason names its blocker by handle, not by id", () => {
  const row = held({ held: [{ why: "depended_on", by: ["01BLOCKER00000000000000000"] }] });

  const named = namedByHandle(row, [job()]);

  expect(named.held).toEqual([{ why: "depended_on", by: ["9-fix-the-retry-ceiling"] }]);
});

test("a blocker no longer on the board keeps its id, rather than showing nothing", () => {
  const row = held({ held: [{ why: "depended_on", by: ["01GONE00000000000000000000"] }] });

  const named = namedByHandle(row, [job()]);

  expect(named.held).toEqual([{ why: "depended_on", by: ["01GONE00000000000000000000"] }]);
});

test("every other reason passes through namedByHandle unchanged", () => {
  const row = held({ held: [{ why: "unmerged", base: "main", commits: 1, tip: "abc123" }] });

  expect(namedByHandle(row, [job()])).toEqual(row);
});

/**
 * **`on_disk` is what removing the checkout is offered on**, never a job's
 * status or its reasons — a checkout fleet already took back offers nothing
 * to remove, whatever else the row still carries.
 */
test("removing the checkout is offered only while on_disk is true", () => {
  const onDisk = held({ on_disk: true, held: [UNMERGED] });
  const gone = held({ on_disk: false, held: [UNMERGED] });

  expect(offeredOn(onDisk, NO_CHOICE).removeCheckout).toBe(true);
  expect(offeredOn(gone, NO_CHOICE).removeCheckout).toBe(false);
});

test("deleting the branch is offered only while an unmerged reason is present", () => {
  const unmerged = held({ held: [UNMERGED] });
  const clean = held({ held: [] });

  expect(offeredOn(unmerged, NO_CHOICE).deleteBranch).toBe(true);
  expect(offeredOn(clean, NO_CHOICE).deleteBranch).toBe(false);
});

/**
 * **The case rule 2 exists for.** Forgetting a record whose checkout still
 * stands orphans that disk from this page — `worktrees_held` walks Job
 * records, not directories — so forget is withheld until the checkout is
 * gone, and joins the moment removing it is chosen in the same act.
 */
test("forgetting the job is withheld until the checkout is gone or chosen", () => {
  const row = held({ on_disk: true, held: [] });

  expect(offeredOn(row, NO_CHOICE).forget).toBe(false);
  expect(offeredOn(row, choosing({ removeCheckout: true })).forget).toBe(true);
});

test("forgetting the job is withheld until an unmerged branch is gone or chosen", () => {
  const row = held({ on_disk: false, held: [UNMERGED] });

  expect(offeredOn(row, NO_CHOICE).forget).toBe(false);
  expect(offeredOn(row, choosing({ deleteBranch: true })).forget).toBe(true);
});

test("forgetting the job is withheld while a branch with no answerable base stands", () => {
  const row = held({ on_disk: false, held: [{ why: "base_unanswered", detail: "none of main is here" }] });

  expect(offeredOn(row, NO_CHOICE).forget).toBe(false);
});

test("a row with neither the checkout nor an unmerged branch offers forget outright", () => {
  const row = held({ on_disk: false, held: [] });

  expect(offeredOn(row, NO_CHOICE).forget).toBe(true);
});

test("chosenRows keeps a row only where something on it is chosen", () => {
  const a = held({ job_id: "a" });
  const b = held({ job_id: "b" });

  const rows = chosenRows([a, b], { a: choosing({ forget: true }) });

  expect(rows.map((row) => row.job_id)).toEqual(["a"]);
});

test("choiceOf answers NO_CHOICE for a row nothing has touched yet", () => {
  expect(choiceOf({}, "untouched")).toEqual(NO_CHOICE);
});
