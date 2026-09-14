// What Setup says about `setup.seed`, on the run sheet and the Manifest
// surface. #1064.

import { describe, expect, it } from "vitest";

import type { CheckoutRunSheet, DeclaredSeed, RunSheet } from "@armada/protocol";
import { checkoutGroupsOf } from "./checkout-runs";
import { runSheetGroupsOf } from "./rehearsal";
import { seedingSaid, seedSaid } from "./seed";

const COMMIT = "a787ffc2c1d0aa00000000000000000000000000";

const SHEET: RunSheet = {
  job_id: "job_2d90bb",
  setup: [],
  checks: [],
  commands: [],
  worktree_on_disk: true,
  worktree_differs: false,
  drone_working: false,
};

const DECLARED: DeclaredSeed = {
  paths: ["target"],
  warmed_by: ["warm_build", "warm_tests"],
  warmth: { state: "warm", commit: COMMIT },
};

describe("seedingSaid", () => {
  it("says nothing where the Job's Manifest declares no seed", () => {
    expect(seedingSaid(undefined)).toBeUndefined();
  });

  it("names what was cloned and the base commit it came from", () => {
    expect(seedingSaid({ state: "seeded", commit: COMMIT, paths: ["target"] })).toBe(
      "This worktree was seeded with target from base a787ffc2c1d0.",
    );
  });

  it("carries Fleet's own reason for a cold start", () => {
    expect(seedingSaid({ state: "cold", why: "the seed at base a787ffc2c1d0 is still warming" })).toBe(
      "This worktree started cold: the seed at base a787ffc2c1d0 is still warming.",
    );
  });

  it("says when nothing was written down rather than guessing", () => {
    expect(seedingSaid({ state: "unrecorded" })).toBe("Nothing records whether this worktree was seeded.");
  });

  it("is the run sheet's Setup line, and only Setup's", () => {
    const groups = runSheetGroupsOf({ ...SHEET, seeding: { state: "seeded", commit: COMMIT, paths: ["target"] } }, new Set());
    expect(groups.find((group) => group.kind === "setup")?.says).toContain("seeded with target");
    expect(groups.filter((group) => group.says !== undefined)).toHaveLength(1);
    expect(runSheetGroupsOf(SHEET, new Set()).some((group) => "says" in group)).toBe(false);
  });
});

describe("seedSaid", () => {
  it("names the directories, the warm-up and how warm it is", () => {
    expect(seedSaid(DECLARED)).toBe(
      "A new worktree is seeded with target, warmed by warm_build, warm_tests. Warm at base a787ffc2c1d0.",
    );
  });

  it("says a Job cut during a warm-up starts cold", () => {
    expect(seedSaid({ ...DECLARED, warmth: { state: "warming", commit: COMMIT } })).toContain(
      "a Job cut now starts cold",
    );
  });

  it("is the Manifest surface's Setup line, and absent where none is declared", () => {
    const sheet: CheckoutRunSheet = { setup: [], checks: [], commands: [] };
    expect(checkoutGroupsOf({ ...sheet, seed: DECLARED }).find((group) => group.kind === "setup")?.says).toContain(
      "warmed by warm_build",
    );
    expect(checkoutGroupsOf(sheet).some((group) => "says" in group)).toBe(false);
  });
});
