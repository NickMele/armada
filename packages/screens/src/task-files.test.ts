// Which files each task changed, and which no task's edits account for. #1187.
import { describe, expect, it } from "vitest";

import type { ChangedFile } from "@armada/components";
import type { PlanTask } from "@armada/protocol";

import { answered, called } from "./fixtures/build/base";
import type { TaskFile } from "./task-files";
import {
  declaredAgainstTouched,
  editsIn,
  filesByTask,
  repoPathOf,
  unownedOf,
} from "./task-files";

const STEP = "implement";
const TREE = "~/Development/armada/.armada/worktrees/01K5/";

function at(seconds: number): string {
  return new Date(Date.parse("2026-09-15T08:20:00Z") + seconds * 1000).toISOString();
}

const TASKS: PlanTask[] = [
  { id: "T1", title: "The table", state: "done", working_windows: [{ entered: at(0), left: at(10) }] },
  { id: "T2", title: "The call sites", state: "working", working_windows: [{ entered: at(10) }] },
];

const DIFF: ChangedFile[] = [
  { path: "crates/store/src/lib.rs", change: "modified", added: 2 },
  { path: "crates/fleet/src/evidence.rs", change: "modified", added: 31, deleted: 9 },
  { path: "crates/store/src/pending_evidence.rs", change: "added", added: 82 },
];

describe("which task an edit belongs to", () => {
  it("is the task being worked when it was made, and none for Bash or a failed call", () => {
    const turns = [
      called(STEP, at(1), "a", "Write", `${TREE}crates/store/src/pending_evidence.rs +82`),
      called(STEP, at(2), "b", "Bash", "sed -i s/x/y/ crates/store/src/lib.rs"),
      called(STEP, at(11), "c", "Edit", `${TREE}crates/fleet/src/evidence.rs +4 -1`),
      called(STEP, at(12), "d", "Edit", `${TREE}crates/fleet/src/settling.rs +2 -2`),
      answered(STEP, at(13), "d", true),
    ];
    expect(editsIn(turns, TASKS).map((edit) => [edit.path.slice(TREE.length), edit.task])).toEqual([
      ["crates/store/src/pending_evidence.rs", "T1"],
      ["crates/fleet/src/evidence.rs", "T2"],
    ]);
  });

  it("names no task where no window holds it", () => {
    const [edit] = editsIn([called(STEP, at(-5), "a", "Edit", "/tmp/x.rs +1 -1")], TASKS);
    expect(edit?.task).toBeUndefined();
  });
});

describe("a call's path, as the diff names it", () => {
  it("matches at a path segment, and the longest match wins", () => {
    const repo = ["src/lib.rs", "crates/store/src/lib.rs"];
    expect(repoPathOf(`${TREE}crates/store/src/lib.rs`, repo)).toBe("crates/store/src/lib.rs");
    expect(repoPathOf(`${TREE}crates/store/src/xlib.rs`, repo)).toBeUndefined();
  });
});

describe("each task's files", () => {
  it("adds up a file's edits, and draws no size where one edit had none", () => {
    const edits = [
      { id: "1", path: `${TREE}crates/fleet/src/evidence.rs`, added: 4, deleted: 1, task: "T2" },
      { id: "2", path: `${TREE}crates/fleet/src/evidence.rs`, added: 2, task: "T2" },
      { id: "3", path: `${TREE}crates/store/src/lib.rs`, added: 1, task: "T2" },
      { id: "4", path: `${TREE}crates/store/src/lib.rs`, task: "T2" },
    ];
    expect(filesByTask(edits, DIFF).get("T2")).toEqual([
      { path: "crates/fleet/src/evidence.rs", inDiff: true, added: 6, deleted: 1 },
      { path: "crates/store/src/lib.rs", inDiff: true },
    ]);
  });

  it("keeps a file the diff does not name, by the call's own path", () => {
    const edits = [{ id: "1", path: "~/notes/scratch.md", added: 3, deleted: 0, task: "T1" }];
    expect(filesByTask(edits, DIFF).get("T1")).toEqual([
      { path: "~/notes/scratch.md", inDiff: false, added: 3, deleted: 0 },
    ]);
  });
});

describe("the files no task's edits account for", () => {
  it("are the diff's files that no task's edit names, so the two readings reconcile", () => {
    const edits = [
      { id: "1", path: `${TREE}crates/fleet/src/evidence.rs`, added: 4, task: "T2" },
      { id: "2", path: `${TREE}crates/store/src/pending_evidence.rs`, added: 82, task: "T1" },
      // Edited while no task was working: that is outside every task's edits too.
      { id: "3", path: `${TREE}crates/store/src/lib.rs`, added: 2 },
    ];
    expect(unownedOf(edits, DIFF).map((file) => file.path)).toEqual(["crates/store/src/lib.rs"]);
  });
});

// `#1432`. A task says which files it will touch and Bridge already works out
// which it did; this is the two put together.
describe("declaredAgainstTouched", () => {
  const file = (path: string): TaskFile => ({ path, inDiff: true });

  it("marks a declared path the work reached, and one it did not", () => {
    const read = declaredAgainstTouched(
      ["packages/screens/src/overview.ts", "packages/screens/src/left-column.test.ts"],
      [file("packages/screens/src/overview.ts")],
    );
    expect(read.declared).toEqual([
      { path: "packages/screens/src/overview.ts", touched: true },
      { path: "packages/screens/src/left-column.test.ts", touched: false },
    ]);
    expect(read.unplanned).toEqual([]);
  });

  it("names a file the work touched and the plan did not", () => {
    const read = declaredAgainstTouched(
      ["packages/screens/src/overview.ts"],
      [file("packages/screens/src/overview.ts"), file("packages/screens/src/TheShell.stories.tsx")],
    );
    expect(read.unplanned).toEqual(["packages/screens/src/TheShell.stories.tsx"]);
  });

  // `declare_scope` takes a directory, and one real Job declared `crates/ipc`
  // then edited `crates/ipc/operations.toml`. A prefix match that tested
  // `startsWith` alone would also count `crates/ipc-extra/x.rs`.
  it("counts a file under a declared directory, and not one that merely shares its spelling", () => {
    const read = declaredAgainstTouched(
      ["crates/ipc"],
      [file("crates/ipc/operations.toml"), file("crates/ipc-extra/x.rs")],
    );
    expect(read.declared).toEqual([{ path: "crates/ipc", touched: true }]);
    expect(read.unplanned).toEqual(["crates/ipc-extra/x.rs"]);
  });

  it("says nothing either way for a task that declared nothing and touched nothing", () => {
    expect(declaredAgainstTouched([], [])).toEqual({ declared: [], unplanned: [] });
  });
});
