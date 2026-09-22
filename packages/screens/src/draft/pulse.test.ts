// One worktree becomes a list, and a process gains an owner.

import type { JobProcess, JobResources } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { pulseViewOf } from "./pulse";

function process(over: Partial<JobProcess> = {}): JobProcess {
  return {
    pid: 4121,
    command: "node",
    cpu_percent: 12.5,
    memory_bytes: 184_000_000,
    running_for: "04:12",
    recorded: true,
    ...over,
  };
}

function resources(over: Partial<JobResources> = {}): JobResources {
  return {
    job_id: "01J",
    read_at: "2026-09-22T10:30:00Z",
    held: "running",
    processes: [process()],
    worktree: { path: "/repo/.armada/worktrees/1532", branch: "armada/1532-draft-schema" },
    ...over,
  };
}

describe("the worktrees a Job holds", () => {
  it("is a list of the one the wire carries", () => {
    const view = pulseViewOf(resources());

    expect(view.worktrees).toHaveLength(1);
    expect(view.worktrees[0]?.branch).toBe("armada/1532-draft-schema");
  });

  it("is empty where the Job has no checkout, never a row with blank fields", () => {
    const bare = resources();
    delete bare.worktree;

    expect(pulseViewOf(bare).worktrees).toEqual([]);
  });
});

describe("who owns a process", () => {
  it("is the one worktree's branch, because that is all a Job has today", () => {
    expect(pulseViewOf(resources()).processes[0]?.owner).toBe("armada/1532-draft-schema");
  });

  it("is null where there is no worktree to place it in", () => {
    const bare = resources();
    delete bare.worktree;

    expect(pulseViewOf(bare).processes[0]?.owner).toBeNull();
  });

  it("keeps the process named by its command and never its arguments", () => {
    const view = pulseViewOf(resources({ processes: [process({ command: "cargo" })] }));

    expect(view.processes[0]?.command).toBe("cargo");
    expect(view.processes[0] && "args" in view.processes[0]).toBe(false);
  });

  it("carries an empty process list through, which is loud and not a gap", () => {
    expect(pulseViewOf(resources({ processes: [] })).processes).toEqual([]);
  });
});

describe("the logs a board lists", () => {
  it("is the Job's own, where anything has ever been written to it", () => {
    const view = pulseViewOf(resources({ wrote_last_at: "2026-09-22T10:29:00Z" }));

    expect(view.logs).toEqual([{ kind: "job", owner: null, writing: false }]);
  });

  it("says nothing is writing, because the wire cannot say that it is", () => {
    const view = pulseViewOf(resources({ wrote_last_at: "2026-09-22T10:29:00Z" }));

    expect(view.logs[0]?.writing).toBe(false);
  });

  it("gives no size, because nothing measures one", () => {
    const view = pulseViewOf(resources({ wrote_last_at: "2026-09-22T10:29:00Z" }));

    expect(view.logs[0]?.bytes).toBeUndefined();
  });

  it("is empty where nothing has been written at all", () => {
    expect(pulseViewOf(resources()).logs).toEqual([]);
  });
});

describe("when the reading was taken", () => {
  it("is carried, because a panel drawing the figures without it lies", () => {
    expect(pulseViewOf(resources()).read_at).toBe("2026-09-22T10:30:00Z");
  });

  it("carries Fleet's reading of its recorded Drone unchanged", () => {
    expect(pulseViewOf(resources({ held: "gone" })).held).toBe("gone");
  });
});
