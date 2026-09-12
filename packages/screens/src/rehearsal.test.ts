// The run sheet's output pane, keyed to the entry it is read against. Nick's
// own report from a live pass: the panel kept `fmt`'s log up under a server's
// serving bar, because the followed run and the selection had quietly become
// two different facts and nothing compared them.

import { describe, expect, it } from "vitest";

import type { RunFollowed } from "@armada/protocol";
import { runOutputOf } from "./rehearsal";

const FMT_STILL_RUNNING: RunFollowed = {
  state: "following",
  jobId: "job_2d90bb",
  runId: "run_1",
  name: "fmt",
  path: ".armada/jobs/2d90bb/runs/run_1.log",
  fromLine: 1,
  lines: ["Fixed 3 files"],
};

const FMT_FINISHED: RunFollowed = { ...FMT_STILL_RUNNING, ended: "exit 0" };

describe("runOutputOf", () => {
  it("reads a followed run when it is the one selected", () => {
    const output = runOutputOf(FMT_STILL_RUNNING, "fmt");
    expect(output?.rows).toEqual([{ row: "line", at: 1, text: "Fixed 3 files" }]);
  });

  it("draws nothing once a different entry is selected", () => {
    // The exact shape of the report: `fmt` is still the thing being followed
    // — nothing told Fleet to stop — but a server row is what is selected now.
    expect(runOutputOf(FMT_STILL_RUNNING, "storybook")).toBeUndefined();
  });

  it("draws nothing for a server just started, even once fmt's own run has ended", () => {
    // Ended rather than still running: the same mismatch a moment later,
    // when the stale reading would otherwise read as this run's own result.
    expect(runOutputOf(FMT_FINISHED, "storybook")).toBeUndefined();
  });

  it("still reads a run already under way when nothing has been selected yet", () => {
    // `useRunSheet` passes `undefined` here for `selected === null` — a sheet
    // opened onto a Job already mid-run, before a person has clicked a row.
    // There is nothing to compare against, which is a different fact from
    // comparing against a row that names something else.
    const output = runOutputOf(FMT_STILL_RUNNING, undefined);
    expect(output?.rows).toEqual([{ row: "line", at: 1, text: "Fixed 3 files" }]);
  });

  it("draws nothing where nothing is being followed", () => {
    expect(runOutputOf({ state: "none" }, "fmt")).toBeUndefined();
  });
});
