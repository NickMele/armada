// The Manifest surface's reading. Three things it must get right, each of
// which is a rule from Journey 9 rather than a detail of the shapes.

import { describe, expect, it } from "vitest";

import type {
  CheckoutRunFollowed,
  CheckoutRunSheet,
  RunEntry,
} from "@armada/protocol";
import {
  checkoutGroupsOf,
  checkoutOutputOf,
  checkoutRunnablesOf,
  runningEntryOf,
} from "./checkout-runs";

function entry(name: string, run: string, over: Partial<RunEntry> = {}): RunEntry {
  return {
    name,
    run,
    narrows: false,
    requires: [],
    expect_exit_code: 0,
    destructive: false,
    frozen: false,
    ...over,
  };
}

const SHEET: CheckoutRunSheet = {
  setup: [entry("bootstrap", "pnpm install --frozen-lockfile")],
  checks: [
    // `narrows` is true and `narrow_run` is absent, which is exactly what
    // Fleet sends for the checkout: the Check declares a `narrow` and there is
    // no diff for one to resolve against.
    entry("test", "cargo nextest run --workspace --exclude acceptance", { narrows: true }),
  ],
  commands: [entry("fmt", "cargo fmt --all", { destructive: true })],
};

describe("the groups the Manifest surface lists", () => {
  it("draws no narrowing, on a Check that declares one", () => {
    const checks = checkoutGroupsOf(SHEET).find((group) => group.kind === "checks");
    // Nothing on `RunPageEntry` can carry a narrowed command, so the assertion
    // is that the row is the whole command and nothing else.
    expect(checks?.entries[0]?.run).toBe("cargo nextest run --workspace --exclude acceptance");
  });

  it("carries a Command's destructive flag through, because this is where it means something", () => {
    const commands = checkoutGroupsOf(SHEET).find((group) => group.kind === "commands");
    expect(commands?.entries[0]?.destructive).toBe(true);
  });

  it("gives the palette one row per entry, with its run line", () => {
    expect(checkoutRunnablesOf({ state: "read", sheet: SHEET })).toEqual([
      { id: "setup:bootstrap", label: "bootstrap", value: "pnpm install --frozen-lockfile" },
      {
        id: "check:test",
        label: "test",
        value: "cargo nextest run --workspace --exclude acceptance",
      },
      { id: "command:fmt", label: "fmt", value: "cargo fmt --all" },
    ]);
  });

  it("lists nothing for the palette before the read has answered", () => {
    expect(checkoutRunnablesOf({ state: "reading" })).toEqual([]);
  });
});

describe("the output pane, keyed to the selection", () => {
  const FMT: CheckoutRunFollowed = {
    state: "following",
    runId: "crun_1",
    name: "fmt",
    path: ".armada/runs/crun_1/output.log",
    fromLine: 1,
    lines: ["Diff in crates/api/src/rehearsing.rs"],
  };

  it("draws nothing where what is followed is not what is selected", () => {
    // `rehearsal.test.ts`'s finding, one surface over: a server's bar came up
    // live while the pane beneath still read the Command run before it.
    expect(checkoutOutputOf(FMT, "storybook_dev")).toBeUndefined();
  });

  it("draws the run where the two agree", () => {
    expect(checkoutOutputOf(FMT, "fmt")?.rows).toHaveLength(1);
  });

  it("draws a run already under way when nothing is selected yet", () => {
    expect(checkoutOutputOf(FMT)?.following).toBe(true);
  });
});

describe("the entry a run in flight is for", () => {
  it("is the row the run's name names, so reopening onto it lights that row", () => {
    expect(runningEntryOf(checkoutGroupsOf(SHEET), "test")).toBe("check:test");
  });

  it("is nothing where nothing is running", () => {
    expect(runningEntryOf(checkoutGroupsOf(SHEET), undefined)).toBeUndefined();
  });
});
