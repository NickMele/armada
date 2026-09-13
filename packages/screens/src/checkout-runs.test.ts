// The Manifest surface's reading. Three things it must get right, each of
// which is a rule from Journey 9 rather than a detail of the shapes.

import { describe, expect, it } from "vitest";

import type {
  CheckoutRunFollowed,
  CheckoutRunRecord,
  CheckoutRunSheet,
  RunEntry,
} from "@armada/protocol";
import {
  checkoutChangedRunOf,
  checkoutGroupsOf,
  checkoutOutputOf,
  checkoutRunDiffReadingOf,
  checkoutRunnablesOf,
  checkoutUndoOffered,
  RUN_CHANGED_NO_LINES,
  RUN_CHANGED_NOTHING,
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

function record(over: Partial<CheckoutRunRecord> = {}): CheckoutRunRecord {
  return {
    id: "crun_1",
    name: "fmt",
    command: "cargo fmt --all",
    required: [],
    started_at: "2026-09-12T14:18:02Z",
    ended_at: "2026-09-12T14:18:05Z",
    duration_ms: 2740,
    exit_code: 0,
    expect_exit_code: 0,
    ended: "exited",
    stopped: false,
    changed: [{ path: "crates/api/src/rehearsing.rs", change: "modified" }],
    undoable: true,
    log: "runs/crun_1/output.log",
    ...over,
  };
}

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

describe("which run the changed-files panel is about, and what it offers", () => {
  it("offers no Undo where Fleet says there is no snapshot behind the run", () => {
    // The rule this exists for: the main checkout holds a person's own
    // uncommitted work, so an Undo that could not be honoured would be the one
    // control on this page whose failure costs somebody theirs.
    const run = checkoutChangedRunOf([record({ undoable: false })]);
    expect(run?.id).toBe("crun_1");
    expect(checkoutUndoOffered(run!)).toBe(false);
  });

  it("keeps a run already undone, so its diff can still be opened, and offers no Undo on it", () => {
    const run = checkoutChangedRunOf([record({ undone_at: "2026-09-12T14:20:00Z" })]);
    expect(run?.id).toBe("crun_1");
    expect(checkoutUndoOffered(run!)).toBe(false);
  });

  it("does not reach past an undone run to offer Undo on an older one", () => {
    // Restoring the older run's snapshot would put back a tree from before a
    // run that has since been undone — a restore nobody could reason about.
    const newer = record({ id: "crun_2", undone_at: "2026-09-12T14:20:00Z" });
    const older = record({ id: "crun_1" });
    expect(checkoutChangedRunOf([newer, older])?.id).toBe("crun_2");
  });

  it("passes over a run that changed nothing", () => {
    expect(checkoutChangedRunOf([record({ changed: [] })])).toBeUndefined();
  });
});

describe("a run's diff, as the sheet draws it", () => {
  const PATCH = [
    "diff --git a/crates/api/src/rehearsing.rs b/crates/api/src/rehearsing.rs",
    "--- a/crates/api/src/rehearsing.rs",
    "+++ b/crates/api/src/rehearsing.rs",
    "@@ -1,1 +1,1 @@",
    "-fn a() {}",
    "+fn a() { }",
  ].join("\n");

  it("draws a snapshot that is gone as gone, in Fleet's words, and nothing in its place", () => {
    const reading = checkoutRunDiffReadingOf("crun_1", {
      ok: true,
      diff: { id: "crun_1", against: "run_snapshot", reading: { state: "gone", why: "no snapshot was taken" } },
    });
    expect(reading).toEqual({ state: "gone", why: "no snapshot was taken" });
  });

  it("does not draw an answer for another run under the one that is open", () => {
    const reading = checkoutRunDiffReadingOf("crun_2", {
      ok: true,
      diff: { id: "crun_1", against: "run_snapshot", reading: { state: "read", files: [], patch: PATCH } },
    });
    expect(reading.state).toBe("reading");
  });

  it("splits the patch into the files and lines git wrote", () => {
    const reading = checkoutRunDiffReadingOf("crun_1", {
      ok: true,
      diff: {
        id: "crun_1",
        against: "run_snapshot",
        reading: {
          state: "read",
          files: [{ path: "crates/api/src/rehearsing.rs", change: "modified" }],
          patch: PATCH,
        },
      },
    });
    expect(reading.state === "read" && reading.files.map((file) => file.path)).toEqual([
      "crates/api/src/rehearsing.rs",
    ]);
  });

  it("says a run changed nothing only where no file changed", () => {
    const nothing = checkoutRunDiffReadingOf("crun_1", {
      ok: true,
      diff: { id: "crun_1", against: "run_snapshot", reading: { state: "read", files: [] } },
    });
    expect(nothing.state === "read" && nothing.emptyNote).toBe(RUN_CHANGED_NOTHING);

    // A binary file changed and git's patch has no line of it. The page lists
    // the file, so "changed nothing" here would contradict the screen behind.
    const binary = checkoutRunDiffReadingOf("crun_1", {
      ok: true,
      diff: {
        id: "crun_1",
        against: "run_snapshot",
        reading: { state: "read", files: [{ path: "logo.png", change: "modified" }] },
      },
    });
    expect(binary.state === "read" && binary.emptyNote).toBe(RUN_CHANGED_NO_LINES);
  });

  it("says Fleet did not answer where it refused", () => {
    const reading = checkoutRunDiffReadingOf("crun_1", {
      ok: false,
      outcome: { ok: false, why: "not_connected" },
    });
    expect(reading.state).toBe("failed");
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
