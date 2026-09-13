// The changed-files panel is the result line's run, and a run's diff is drawn
// as Fleet answered it — never with anything standing in for a gone snapshot.

import { describe, expect, it } from "vitest";

import type { CheckoutRunRecord } from "@armada/protocol";
import {
  checkoutChangedOf,
  checkoutRunDiffReadingOf,
  RUN_CHANGED_NO_LINES,
  RUN_CHANGED_NOTHING,
} from "./checkout-run-diff";

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

const ACTS = { onOpenDiff: () => {}, onUndo: () => {} };

describe("the changed-files panel, for the result line's run", () => {
  it("offers the diff and Undo on a run that changed files", () => {
    const panel = checkoutChangedOf(record(), ACTS);
    expect(panel.files).toHaveLength(1);
    expect(panel.onOpenDiff).toBeDefined();
    expect(panel.onUndo).toBeDefined();
  });

  it("says a run changed nothing and offers neither act", () => {
    // `format` after `fmt`: the panel under format's result must not carry
    // fmt's files and an Undo that reads as undoing format.
    const panel = checkoutChangedOf(record({ name: "format", changed: [] }), ACTS);
    expect(panel).toEqual({ files: [] });
  });

  it("offers no Undo where Fleet says there is no snapshot behind the run", () => {
    expect(checkoutChangedOf(record({ undoable: false }), ACTS).onUndo).toBeUndefined();
  });

  it("keeps an undone run's diff, says it was undone, and offers no second Undo", () => {
    const panel = checkoutChangedOf(record({ undone_at: "2026-09-12T14:20:00Z" }), ACTS);
    expect(panel.onOpenDiff).toBeDefined();
    expect(panel.onUndo).toBeUndefined();
    expect(panel.undone).toMatch(/^Undone at /);
  });

  it("says why where the change could not be read, rather than that nothing changed", () => {
    const panel = checkoutChangedOf(record({ changed: [], changed_unreadable: "git status failed" }), ACTS);
    expect(panel.unreadable).toMatch(/git status failed/);
    expect(panel.onOpenDiff).toBeUndefined();
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
        reading: { state: "read", files: record().changed, patch: PATCH },
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
    const reading = checkoutRunDiffReadingOf("crun_1", { ok: false, outcome: { ok: false, why: "not_connected" } });
    expect(reading.state).toBe("failed");
  });
});
