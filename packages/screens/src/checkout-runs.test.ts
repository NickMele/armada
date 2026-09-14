// The Manifest surface's reading. Three things it must get right, each of
// which is a rule from Journey 9 rather than a detail of the shapes.

import { describe, expect, it } from "vitest";

import type {
  CheckoutRunFollowed,
  CheckoutRunRecord,
  CheckoutRunSheet,
  RunEntry,
  ServerState,
} from "@armada/protocol";
import {
  checkoutGroupsOf,
  checkoutOutputOf,
  checkoutResultRunOf,
  checkoutRunnablesOf,
  exitedOf,
  runningEntryOf,
} from "./checkout-runs";
import { checkoutStartOf, followedWorkspaceOf } from "./checkout-workspace";

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
    expect(checkoutOutputOf(FMT, { name: "storybook_dev" })).toBeUndefined();
  });

  it("draws the run where the two agree", () => {
    expect(checkoutOutputOf(FMT, { name: "fmt" })?.rows).toHaveLength(1);
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

describe("the result line, keyed to the selection", () => {
  // The owner's screen after a reload: `storybook_dev` selected, and the panel under it reading
  // `gate was stopped`, the newest run of anything.
  const run = (id: string, name: string) => ({ id, name }) as CheckoutRunRecord;
  const RUNS = [run("r2", "gate"), run("r1", "fmt")];

  it("is nothing for a server, which is not a run", () => {
    expect(checkoutResultRunOf(RUNS, "server:storybook_dev", undefined)).toBeUndefined();
  });

  it("is the selected entry's own newest run, not the newest of anything", () => {
    expect(checkoutResultRunOf(RUNS, "command:fmt", undefined)?.id).toBe("r1");
  });

  it("is nothing for an entry that has not run", () => {
    expect(checkoutResultRunOf(RUNS, "check:test", undefined)).toBeUndefined();
  });

  it("is the run opened from Earlier runs, whatever is selected", () => {
    expect(checkoutResultRunOf(RUNS, "server:storybook_dev", "r2")?.id).toBe("r2");
  });
});

describe("an ended server", () => {
  const ENDED = { phase: "exited", stopped: false } as ServerState;

  it("does not say it stopped on its own, or exit 0, when somebody pressed Stop", () => {
    expect(exitedOf({ ...ENDED, stopped: true })).toEqual({ phase: "exited", stopped: true });
  });

  it("carries its code where it exited on its own", () => {
    expect(exitedOf({ ...ENDED, exit_code: 1 })).toEqual({ phase: "exited", stopped: false, exitCode: 1 });
  });
});

describe("a workspace's own Commands", () => {
  const ROOTLESS: CheckoutRunSheet = {
    setup: [],
    checks: [],
    commands: [],
    workspaces: [
      { dir: "apps/web", commands: [entry("dev", "pnpm dev")] },
      { dir: "apps/api", commands: [entry("dev", "cargo run")] },
    ],
  };
  const groups = checkoutGroupsOf(ROOTLESS, true);

  it("lists only Commands where there is no root file, each saying where it runs", () => {
    expect(groups.map((group) => group.label)).toEqual(["Commands"]);
    expect(groups[0]?.entries.map((row) => row.note)).toEqual(["In apps/web.", "In apps/api."]);
  });

  it("starts a row with its workspace named", () => {
    expect(checkoutStartOf(groups[0]!.entries[1]!.id)).toEqual({ name: "dev", workspace: "apps/api" });
    expect(checkoutStartOf("command:fmt")).toEqual({ name: "fmt" });
  });

  it("keeps two of one name apart, in flight and in Earlier runs", () => {
    expect(runningEntryOf(groups, "dev", "apps/api")).toBe(groups[0]!.entries[1]!.id);
    const runs = [
      { id: "r2", name: "dev", workspace: "apps/api" },
      { id: "r1", name: "dev", workspace: "apps/web" },
    ] as CheckoutRunRecord[];
    expect(checkoutResultRunOf(runs, groups[0]!.entries[0]!.id, undefined)?.id).toBe("r1");
    expect(checkoutRunnablesOf({ state: "read", sheet: ROOTLESS }).map((row) => row.label)).toEqual([
      "dev in apps/web",
      "dev in apps/api",
    ]);
  });
});

describe("two Commands of one name in different workspaces", () => {
  const DEV_IN_API: CheckoutRunFollowed = {
    state: "following",
    runId: "crun_api",
    name: "dev",
    path: ".armada/runs/main/crun_api/output.log",
    fromLine: 1,
    lines: ["listening"],
  };
  const followedIn = followedWorkspaceOf(DEV_IN_API, new Map([["crun_api", "apps/api"]]), []);

  it("draw only the selected one's output, never the other's", () => {
    expect(checkoutOutputOf(DEV_IN_API, { name: "dev", workspace: "apps/web" }, followedIn)).toBeUndefined();
    expect(checkoutOutputOf(DEV_IN_API, { name: "dev", workspace: "apps/api" }, followedIn)?.rows).toHaveLength(1);
    expect(checkoutOutputOf(DEV_IN_API, { name: "dev" }, followedIn)).toBeUndefined();
  });

  it("know a finished run's workspace from its record once the sheet no longer has it out", () => {
    const runs = [{ id: "crun_api", name: "dev", workspace: "apps/api" }] as CheckoutRunRecord[];
    expect(followedWorkspaceOf(DEV_IN_API, new Map(), runs)).toBe("apps/api");
  });
});

describe("a root Manifest with workspaces of its own", () => {
  const groups = checkoutGroupsOf({
    ...SHEET,
    workspaces: [{ dir: "apps/web", commands: [entry("fmt", "pnpm format")] }],
  });

  it("keeps the root's three groups and lists the workspace's Commands after the root's", () => {
    expect(groups.map((group) => group.label)).toEqual(["Setup", "Checks", "Commands"]);
    const commands = groups[2]!.entries;
    const fmt = commands.filter((row) => row.name === "fmt").map((row) => row.note ?? "root");
    expect(fmt).toEqual(["root", "In apps/web."]);
    expect(runningEntryOf(groups, "fmt")).toBe("command:fmt");
  });
});
