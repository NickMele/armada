// Where a Check runs, from the screen's side. #849.
//
// A green run is not the whole bar when some Checks never ran in it, so the
// Checks chapter names what the reading left out: Checks a Drone's own run
// does not ask, a handoff Check the gate has not reached, and Checks a later
// step runs once before handoff.

import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";

import type { CheckRun as CheckRunRow } from "@armada/components";
import type { StepDetail } from "@armada/protocol";

import { checksChapter } from "./checks";
import { HELD_FOR_HANDOFF, NOT_IN_THE_DRONES_RUN, RUNS_LAST_BEFORE_HANDOFF } from "./declared";
import { droneRunOf } from "./gates";
import type { Opens } from "./phases";

const OPENS: Opens = {
  jobId: "01M130Y1380016YK5S0JXBXDQ5",
  open: () => Promise.resolve({ ok: true }),
  onSaid: () => {},
};

const AT = "2026-09-14T09:00:00Z";

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "tests",
    label: "Write tests",
    ordinal: 2,
    state: "running",
    checks: [
      { kind: "manifest_check", name: "build", run: "cargo build" },
      { kind: "manifest_check", name: "storybook", run: "pnpm build-storybook", runs_at: "gate" },
      { kind: "manifest_check", name: "e2e", run: "pnpm e2e", runs_at: "handoff" },
    ],
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: AT,
    updated_at: AT,
    ...over,
  };
}

function rowsOf(detail: StepDetail): CheckRunRow[] {
  const chapter = checksChapter(detail, [], OPENS, Date.parse(AT));
  const preview = chapter?.preview as ReactElement<{ rows: CheckRunRow[] }>;
  return preview.props.rows;
}

describe("a Drone's own run beside Checks it does not ask", () => {
  const asked = step({
    dry_run: {
      attempt: 1,
      checks: [{ name: "build", started_at: AT, took_ms: 1_000, ran: { attempt: 1, name: "build", outcome: "passed" } }],
    },
  });

  it("reads only the Checks the run asks", () => {
    expect(droneRunOf(asked)?.map((read) => read.name)).toEqual(["build"]);
  });

  it("names the ones it left out and where each runs instead", () => {
    const rows = rowsOf(asked);
    expect(rows.find((row) => row.id === "storybook")?.says).toBe(NOT_IN_THE_DRONES_RUN);
    expect(rows.find((row) => row.id === "e2e")?.says).toBe(
      "Not in the Drone's run. It runs last, before handoff.",
    );
    expect(rows.find((row) => row.id === "storybook")?.named).toBeUndefined();
  });
});

describe("the gate's reading", () => {
  it("says a handoff Check it has not reached runs last", () => {
    const e2e = rowsOf(step()).find((row) => row.id === "e2e");
    expect(e2e?.says).toBe(RUNS_LAST_BEFORE_HANDOFF);
    expect(e2e?.named).toBe("queued");
  });

  it("draws a gate-only Check like any other Check", () => {
    const storybook = rowsOf(step()).find((row) => row.id === "storybook");
    expect(storybook?.says).not.toBe(NOT_IN_THE_DRONES_RUN);
  });
});

describe("a step that leaves a Check to a later one", () => {
  it("names it as not checked here", () => {
    const earlier = step({
      step_id: "implement",
      checks: [{ kind: "manifest_check", name: "build", run: "cargo build" }],
      held_for_handoff: ["e2e"],
    });
    const held = rowsOf(earlier).find((row) => row.id === "held:e2e");
    expect(held?.says).toBe(HELD_FOR_HANDOFF);
    expect(held?.identifier).toBe("e2e");
  });
});
