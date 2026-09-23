// Nobody listed them, versus a list that found nothing — and what a floor
// derived from two reads is allowed to claim.

import type { ManifestDeclared, WorktreeHeld } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { baseBranch, branchesOf } from "./branches";

function declared(over: Partial<ManifestDeclared> = {}): ManifestDeclared {
  return {
    checks: [],
    commands: [],
    ports: [],
    auto_merge: { written: "never", offered: ["never", "always"] },
    review_gate: { written: "always", offered: ["always", "never"] },
    ...over,
  };
}

function worktree(branch: string, title: string): WorktreeHeld {
  return {
    job_id: `01${title}`,
    job_title: title,
    status: "running",
    last_moved_at: "2026-09-22T08:31:00Z",
    path: `/Users/user/armada/.armada/worktrees/${branch}`,
    on_disk: true,
    branch,
    held: [],
  };
}

describe("the difference between absent and empty", () => {
  it("answers null where neither read was made", () => {
    expect(branchesOf()).toBeNull();
  });

  it("answers an empty list where the reads found nothing", () => {
    expect(branchesOf(declared(), [])).toEqual([]);
  });

  it("keeps them apart, which is what the field draws two ways", () => {
    expect(branchesOf()).not.toEqual(branchesOf(declared(), []));
  });
});

describe("what the floor holds", () => {
  it("puts the base first and marks it", () => {
    const branches = branchesOf(declared({ base: "main" }), [
      worktree("armada/18-fold-the-capacity-read", "Fold the capacity read into one query"),
    ]);

    expect(branches?.map((one) => one.name)).toEqual([
      "main",
      "armada/18-fold-the-capacity-read",
    ]);
    expect(branches?.[0]?.base).toBe(true);
    expect(branches?.[1]?.base).toBe(false);
  });

  it("names the Job sitting on a branch, and names none on the base", () => {
    const branches = branchesOf(declared({ base: "main" }), [
      worktree("armada/19-give-the-rail-its-own-scroll", "Give the rail its own scroll"),
    ]);

    expect(branches?.[0]?.job).toBeUndefined();
    expect(branches?.[1]?.job).toBe("Give the rail its own scroll");
  });

  it("draws a Job cut straight onto the base once, still marked as the base", () => {
    const branches = branchesOf(declared({ base: "main" }), [worktree("main", "A Job on the base")]);

    expect(branches).toHaveLength(1);
    expect(branches?.[0]).toEqual({ name: "main", base: true });
  });

  it("holds the worktrees in the order the read gave them, unsorted", () => {
    const branches = branchesOf(declared({ base: "main" }), [
      worktree("zebra", "Last alphabetically, first read"),
      worktree("alpha", "First alphabetically, second read"),
    ]);

    expect(branches?.map((one) => one.name)).toEqual(["main", "zebra", "alpha"]);
  });

  it("lists the worktrees alone where the Manifest names no base", () => {
    const branches = branchesOf(declared(), [worktree("armada/7-something", "Something")]);

    expect(branches?.map((one) => one.base)).toEqual([false]);
    expect(baseBranch(branches)).toBeNull();
  });
});

describe("what both fields open on", () => {
  it("is the base, read off the list rather than off the Manifest again", () => {
    expect(baseBranch(branchesOf(declared({ base: "trunk" }), []))).toBe("trunk");
  });

  it("is nothing where nobody listed them", () => {
    expect(baseBranch(null)).toBeNull();
  });
});
