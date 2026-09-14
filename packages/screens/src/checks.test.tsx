// A step whose gate is running its Checks, as Job detail draws it.
//
// **`#628`, from the screen's side.** A person watching a Job after its Drone
// submitted saw every Check say nothing had run it and the strip say the Drone
// was working, for as long as the gate took. What is pinned here is what the
// rows and the strip say while `StepDetail.checking` is there: a running Check
// reads as running with how long it has run, one waiting for a slot reads as
// waiting, a finished one reads as its result, and the Drone is no longer the
// one working.

import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";

import { AssertionSet, type CheckRun as CheckRunRow } from "@armada/components";
import type { CheckRun, CheckUnderway, StepDetail } from "@armada/protocol";

import { checkSheetOf, checksChapter, saidOf } from "./checks";
import type { Opens } from "./phases";

const OPENS: Opens = {
  jobId: "01M130Y1380016YK5S0JXBXDQ5",
  open: () => Promise.resolve({ ok: true }),
  onSaid: () => {},
};

const STARTED = "2026-09-11T09:00:00Z";

/** A minute and four seconds after the running Check started. */
const NOW = Date.parse(STARTED) + 64_000;

function gating(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "implement",
    label: "Write tests",
    ordinal: 1,
    state: "running",
    checks: [
      { kind: "manifest_check", name: "build", run: "cargo build --workspace --locked" },
      { kind: "manifest_check", name: "test", run: "cargo nextest run --workspace" },
      { kind: "manifest_check", name: "format", run: "cargo fmt --check" },
    ],
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: STARTED,
    updated_at: STARTED,
    checking: {
      attempt: 1,
      checks: [
        {
          name: "build",
          started_at: STARTED,
          took_ms: 9_000,
          ran: { attempt: 1, name: "build", outcome: "passed" },
          output_path: ".armada/checks/01M1/implement.1.live.0.log",
        },
        {
          name: "test",
          started_at: STARTED,
          output_path: ".armada/checks/01M1/implement.1.live.1.log",
        },
        { name: "format" },
      ],
    },
    ...over,
  };
}

function rowsOf(step: StepDetail): CheckRunRow[] {
  const chapter = checksChapter(step, [], OPENS, NOW);
  const preview = chapter?.preview as ReactElement<{ rows: CheckRunRow[] }>;
  return preview.props.rows;
}

describe("a step whose gate is running its Checks", () => {
  it("draws a running Check as running, with how long it has run", () => {
    const test = rowsOf(gating()).find((row) => row.id === "test");
    expect(test?.named).toBe("running");
    expect(test?.result).toBe("running");
    expect(test?.says).toBe("Running for 1m 04s.");
    expect(test?.output).toBe("implement.1.live.1.log");
  });

  it("draws a Check waiting for a slot as waiting, and a finished one as its result", () => {
    const rows = rowsOf(gating());
    const format = rows.find((row) => row.id === "format");
    expect(format?.named).toBe("queued");
    expect(format?.result).toBe("waiting");
    expect(rows.find((row) => row.id === "build")?.named).toBe("passed");
  });

  it("counts what is running in the chapter's summary", () => {
    const chapter = checksChapter(gating(), [], OPENS, NOW);
    expect(chapter?.summary).toBe("1 running · 1 of 3 passed");
  });
});

// #1063 — Checks waiting for room other work holds on the machine read as
// queued behind it, so a slow gate does not read as a stuck one.
describe("a gate waiting for room on the machine", () => {
  const queued = gating({
    checking: {
      attempt: 1,
      checks: [
        { name: "build", waiting_behind: 3 },
        { name: "test", waiting_behind: 3 },
        { name: "format", waiting_behind: 3 },
      ],
    },
  });

  it("says each Check waits behind other work, and how much", () => {
    const build = rowsOf(queued).find((row) => row.id === "build");
    expect(build?.says).toBe("Waiting for room behind 3 other Checks on this machine.");
    expect(build?.result).toBe("waiting");
    expect(build?.named).toBe("queued");
  });

  it("says it waits for room in the summary", () => {
    expect(checksChapter(queued, [], OPENS, NOW)?.summary).toBe("3 waiting for room");
  });

  it("names one other Check in the singular", () => {
    const one = gating({ checking: { attempt: 1, checks: [{ name: "format", waiting_behind: 1 }] } });
    expect(rowsOf(one).find((row) => row.id === "format")?.says).toBe(
      "Waiting for room behind 1 other Check on this machine.",
    );
  });

  it("says a Check waiting only on its own run is waiting to start", () => {
    expect(rowsOf(gating()).find((row) => row.id === "format")?.says).toBe("Waiting to start.");
  });
});

// #1102 — a heavier Check says how many places it takes, only where that is
// more than one.
describe("a Check that takes more than one place", () => {
  it("names the places it needs, waiting behind other work", () => {
    const heavy = gating({
      checking: {
        attempt: 1,
        checks: [{ name: "build", waiting_behind: 2, places: 3 }],
      },
    });
    expect(rowsOf(heavy).find((row) => row.id === "build")?.says).toBe(
      "Waiting for room behind 2 other Checks on this machine. It takes 3 places.",
    );
  });

  it("names the places it needs, waiting on nothing but its own run", () => {
    const heavy = gating({
      checking: { attempt: 1, checks: [{ name: "build", places: 3 }] },
    });
    expect(rowsOf(heavy).find((row) => row.id === "build")?.says).toBe(
      "Waiting to start. It takes 3 places.",
    );
  });

  it("says nothing extra for a Check that takes one place", () => {
    const one = gating({
      checking: { attempt: 1, checks: [{ name: "build", waiting_behind: 2, places: 1 }] },
    });
    expect(rowsOf(one).find((row) => row.id === "build")?.says).toBe(
      "Waiting for room behind 2 other Checks on this machine.",
    );
  });
});

// #1021 — a press names the Check, and the sheet is what decides live or
// kept. This file draws the chapter, not the sheet, so what is proved here is
// narrower: pressing a row reports the pressed Check and nothing more, and
// nothing the chapter draws is a reading with no end.
describe("a press on a Check's row", () => {
  it("reports the Check pressed, rather than opening a file itself", () => {
    const opened: string[] = [];
    const chapter = checksChapter(gating(), [], OPENS, NOW, undefined, undefined, undefined, (checkId) =>
      opened.push(checkId),
    );
    const preview = chapter?.preview as ReactElement<{ onOpen?: (checkId: string) => void }>;
    preview.props.onOpen?.("test");
    expect(opened).toEqual(["test"]);
  });

  it("draws no console output under the rows, live or kept — only the assertion set has an end", () => {
    // One Check recorded (`build`, so `assertedIn` has a row to draw) beside
    // one still running (`test`, with a live `output_path`) — the exact shape
    // #1021 poured a growing log out of. The content this chapter offers is
    // whatever survived that fix: `AssertionSet` alone, never a `ConsoleOutput`
    // for the running Check's log.
    const chapter = checksChapter(
      gating({ check_runs: [{ attempt: 1, name: "build", outcome: "passed" }] }),
      [],
      OPENS,
      NOW,
    );
    const content = chapter?.content as ReactElement<{ rows: unknown[] }> | undefined;
    expect(content?.type).toBe(AssertionSet);
  });
});

// The sheet's own question — `Sheets.tsx`'s `CheckSheet` calls this on every
// render, so a Check that finishes while its sheet is open moves from live to
// kept without the sheet closing.
describe("what a Check's output sheet should read", () => {
  it("reads live while the gate is still writing it", () => {
    expect(checkSheetOf(gating(), "test")).toEqual({
      kind: "live",
      kept: "implement.1.live.1.log",
    });
  });

  it("reads kept once the gate has ruled and stopped writing", () => {
    const step = gating({
      checking: undefined,
      check_runs: [
        { attempt: 1, name: "test", outcome: "passed", output_path: ".armada/checks/01M1/implement.1.test.log" },
      ],
    });
    expect(checkSheetOf(step, "test")).toEqual({ kind: "kept", kept: "implement.1.test.log" });
  });

  it("reads nothing for a Check that has never run", () => {
    expect(checkSheetOf(gating(), "format")).toBeUndefined();
  });

  it("reads nothing for a Check nobody declared", () => {
    expect(checkSheetOf(gating(), "nonexistent")).toBeUndefined();
  });
});

describe("a Check the gate reused from the Drone's own dry run", () => {
  const passed: CheckRun = { attempt: 1, name: "build", outcome: "passed" };
  const reused: CheckRun = { ...passed, reused_from_dry_run: "2026-09-13T09:00:00Z" };

  it("says it was reused, and a Check the gate ran itself does not", () => {
    expect(saidOf(reused)).toBe("Passed — reused from the drone's run");
    expect(saidOf(passed)).toBe("Passed");
  });
});

// #1062 — a Drone's own mid-step run, drawn where the gate's would be and
// marked as the Drone's, so a person never reads it as a ruling.
describe("a Drone's own run of the step's Checks", () => {
  function asking(checks: CheckUnderway[]): StepDetail {
    return gating({ checking: undefined, dry_run: { attempt: 1, checks } });
  }

  it("draws which Check is running and each result as it lands, marked as the Drone's run", () => {
    const step = asking([
      { name: "build", started_at: STARTED, took_ms: 9_000, ran: { attempt: 1, name: "build", outcome: "passed" } },
      { name: "test", started_at: STARTED },
      { name: "format" },
    ]);
    expect(checksChapter(step, [], OPENS, NOW)?.summary).toBe("The Drone's run · 1 running · 1 of 3 passed");
    const rows = rowsOf(step);
    expect(rows.find((row) => row.id === "build")?.named).toBe("passed");
    expect(rows.find((row) => row.id === "test")?.says).toBe("Running for 1m 04s.");
    expect(rows.find((row) => row.id === "format")?.named).toBe("queued");
  });

  it("counts the Checks its first failure stopped apart from the one that failed", () => {
    const stopped = "stopped when `build` did not pass";
    const step = asking([
      {
        name: "build",
        started_at: STARTED,
        took_ms: 4_000,
        ran: { attempt: 1, name: "build", outcome: "failed", produced: "it exited 1" },
      },
      {
        name: "test",
        started_at: STARTED,
        took_ms: 4_100,
        ran: { attempt: 1, name: "test", outcome: "signalled", produced: stopped },
        stopped_by: "build",
      },
      {
        name: "format",
        took_ms: 0,
        ran: { attempt: 1, name: "format", outcome: "never_ran", produced: stopped },
        stopped_by: "build",
      },
    ]);
    expect(checksChapter(step, [], OPENS, NOW)?.summary).toBe("The Drone's run · 1 of 3 did not pass · 2 stopped");
    const test = rowsOf(step).find((row) => row.id === "test");
    expect(test?.says).toBe("Stopped when build did not pass.");
    expect(test?.named).toBeUndefined();
  });

  it("gives way to the gate's own run, which is the step's now", () => {
    const both = gating({ dry_run: { attempt: 1, checks: [{ name: "build", started_at: STARTED }] } });
    expect(checksChapter(both, [], OPENS, NOW)?.summary).toBe("1 running · 1 of 3 passed");
  });
});
