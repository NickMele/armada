// What the Pulse board says a Job is taking, and the answers it must not round.
//
// The board is built on `PulseView`, which is derived from today's wire. These
// are the decisions between that shape and the rows the panel draws: which
// checkout a look's finding is about, and what each figure reads when its
// reading is nought or is a fault.

import { describe, expect, it } from "vitest";

import type { JobExamined, Look, StepDetail } from "@armada/protocol";
import type { PulseView } from "./draft/pulse";
import { checksRunning, judgesRunning, pulseFiguresOf, pulseReadingOf } from "./resources";

const BRANCH = "armada/1538-pulse";

function view(over: Partial<PulseView> = {}): PulseView {
  return {
    job: "01J",
    read_at: "2026-09-22T10:30:00.000Z",
    held: "running",
    processes: [
      {
        pid: 4121,
        command: "node",
        cpu_percent: 12.5,
        memory_bytes: 184_000_000,
        running_for: "04:12",
        recorded: true,
        owner: BRANCH,
      },
    ],
    worktrees: [{ path: "/repo/.armada/worktrees/1538", branch: BRANCH, bytes: 1_020_054_016 }],
    logs: [{ kind: "job", owner: null, writing: false }],
    ...over,
  };
}

function looked(found: Look["found"]): JobExamined {
  return {
    job_id: "01J",
    looked_at: "2026-09-22T10:30:00.000Z",
    found: "cannot_tell",
    looks: [{ asked: "worktree", found, said: "" }],
    resources: { job_id: "01J", read_at: "2026-09-22T10:30:00.000Z", held: "running", processes: [] },
  };
}

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "implement",
    label: "Implement",
    ordinal: 1,
    state: "running",
    check_runs: [],
    judged: [],
    flagged: [],
    overridden: false,
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-22T09:22:00Z",
    updated_at: "2026-09-22T10:30:00Z",
    ...over,
  };
}

describe("the rows the board draws", () => {
  it("carries the owner through, so a process names the checkout it is in", () => {
    expect(pulseReadingOf(view(), null).processes[0]?.owner).toBe(BRANCH);
  });

  it("says a process nothing placed is not placed, rather than dropping the row", () => {
    const one = view({ processes: [{ ...view().processes[0]!, owner: null }] });

    expect(pulseReadingOf(one, null).processes).toHaveLength(1);
    expect(pulseReadingOf(one, null).processes[0]?.owner).toBeNull();
  });

  it("keeps an unmeasured worktree unmeasured, never a zero", () => {
    const one = view({ worktrees: [{ path: "/repo/.armada/worktrees/1538", branch: BRANCH }] });

    expect(pulseReadingOf(one, null).worktrees[0]?.bytes).toBeUndefined();
  });

  it("puts a look's finding on the one checkout it asked about", () => {
    const [row] = pulseReadingOf(view(), looked("not_working")).worktrees;

    expect(row?.state).toBe("gone");
    expect(row?.wrong).toBe(true);
  });

  it("leaves several checkouts alone, because the look asked about one", () => {
    const two = view({
      worktrees: [
        { path: "/a", branch: "armada/a" },
        { path: "/b", branch: "armada/b" },
      ],
    });

    expect(pulseReadingOf(two, looked("not_working")).worktrees.map((one) => one.wrong)).toEqual([
      undefined,
      undefined,
    ]);
  });

  it("marks a log that is still being written", () => {
    const one = view({ logs: [{ kind: "job", owner: null, writing: true }] });

    expect(pulseReadingOf(one, null).logs[0]?.writing).toBe(true);
  });
});

describe("the figures over the board", () => {
  it("says none running rather than 0, which reads as a gap", () => {
    const figures = pulseFiguresOf(view({ held: "none", processes: [] }), null);

    expect(figures.find((one) => one.label === "Drones")?.value).toBe("none running");
  });

  it("counts the one Drone Fleet recorded while it is up", () => {
    expect(pulseFiguresOf(view(), null).find((one) => one.label === "Drones")?.value).toBe(
      "1 running",
    );
  });

  it("draws a recorded pid nothing holds as a fault, not as an idle Job", () => {
    const drones = pulseFiguresOf(view({ held: "gone" }), null).find(
      (one) => one.label === "Drones",
    );

    expect(drones?.value).toBe("none running");
    expect(drones?.wrong).toBe(true);
  });

  it("does not claim a Drone count where the probe would not run", () => {
    expect(pulseFiguresOf(view({ held: "unreadable" }), null).find((one) => one.label === "Drones")?.value).toBe(
      "could not be read",
    );
  });

  it("leaves the Drones row out entirely where nothing read the machine", () => {
    expect(pulseFiguresOf(null, null).map((one) => one.label)).not.toContain("Drones");
  });

  it("draws neither cap where Fleet does not count, rather than a zero", () => {
    const labels = pulseFiguresOf(view(), null).map((one) => one.label);

    expect(labels).not.toContain("Spend");
    expect(labels).not.toContain("Turns");
  });
});

describe("what is running on the Job's own steps", () => {
  it("counts a Check the gate started and has not finished", () => {
    const running = step({
      checks: [{ kind: "manifest_check", name: "test", when: [], expect_exit_code: 0 }],
      checking: { attempt: 1, checks: [{ name: "test", started_at: "2026-09-22T10:29:00Z" }] },
    });

    expect(checksRunning([running])).toBe(1);
  });

  it("does not count a Check that has finished on this attempt", () => {
    const done = step({
      checks: [{ kind: "manifest_check", name: "test", when: [], expect_exit_code: 0 }],
      check_runs: [{ attempt: 1, name: "test", outcome: "passed" }],
      attempts: [{ attempt: 1, outcome: "advanced", started_at: "2026-09-22T09:22:00Z" }],
    });

    expect(checksRunning([done])).toBe(0);
  });

  it("counts a Judge call that is out, which is not a step state", () => {
    const judging = step({
      judging: {
        look: "criterion",
        model: "sonnet",
        call: 1,
        of: 2,
        since: "2026-09-22T10:29:30Z",
        budget_ms: 120_000,
      },
    });

    expect(judgesRunning([judging, step()])).toBe(1);
  });
});
