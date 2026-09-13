// What a reclaim notice says when a Fleet sends an empty field as `null`.
//
// A Fleet built before #827 did, within the same protocol, and the notice
// called `.slice` on the null tip and took the window down.

import { describe, expect, it } from "vitest";

import type { WorktreeReclaimed } from "@armada/protocol";

import { reclaimed } from "./copy";

function answer(branch: Partial<WorktreeReclaimed["branch"]>): WorktreeReclaimed {
  return {
    job_id: "01JOB",
    worktree: { path: "/worktrees/01JOB", removed: true, why: null },
    branch: { branch: "armada/01JOB", deleted: true, tip: null, why: null, base: null, unmerged_commits: null, ...branch },
  };
}

describe("a reclaim an older Fleet answered with nulls", () => {
  it("names a deleted branch without a commit it was not given", () => {
    expect(reclaimed(answer({}))).toBe(
      "The worktree at /worktrees/01JOB is gone. Branch armada/01JOB was deleted.",
    );
  });

  it("reads a null unmerged count as a branch left alone, not a deliberate keep", () => {
    expect(reclaimed(answer({ deleted: false }))).toBe(
      "The worktree at /worktrees/01JOB is gone. Branch armada/01JOB was left alone — no reason was given.",
    );
  });
});
