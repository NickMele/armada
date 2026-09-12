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

import type { CheckRun as CheckRunRow } from "@armada/components";
import type { StepDetail } from "@armada/protocol";

import { checksChapter } from "./checks";
import type { Following, Outputs } from "./outputs";
import type { Opens } from "./phases";

const OPENS: Opens = {
  jobId: "01M130Y1380016YK5S0JXBXDQ5",
  open: () => Promise.resolve({ ok: true }),
  onSaid: () => {},
};

const OUTPUTS: Outputs = { of: () => undefined, fetch: () => {} };

const FOLLOWING: Following = {
  reading: { state: "none" },
  picked: null,
  pick: () => {},
  follow: () => {},
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
  const chapter = checksChapter(step, [], OPENS, OUTPUTS, NOW, FOLLOWING);
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
    const chapter = checksChapter(gating(), [], OPENS, OUTPUTS, NOW, FOLLOWING);
    expect(chapter?.summary).toBe("1 running · 1 of 3 passed");
  });
});
