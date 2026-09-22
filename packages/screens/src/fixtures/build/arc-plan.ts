// The plan the arc runs: four groups, eight tasks, one agent each.
//
// **An unmarked task gets its own agent** (owner, 22 Sep 2026), so every task
// here is `own_drone` and carries a Drone id of its own once it has run.
//
// **The groups are where the Checks run** (#1530, 21 Sep), and which Checks
// those are comes from `armada.yml`'s `when:` globs rather than from a
// drawing: the group that writes Rust runs four, and the three that write
// Bridge run seven each.
//
// Every builder here returns the plan as it stands before anything has run.
// A moment moves it with `withGroup` and `withTask`, so no two moments spell
// the same task twice and a field added to `TaskView` lands in one place.

import type { CaseView, GroupView, TaskView } from "../../draft";
import { BRIDGE_CHECKS, checkNames, RUST_CHECKS } from "./arc-base";

/** The Drone put on each task, by task id. One per task, never one per Job. */
export const ARC_DRONES: Record<string, string> = {
  T1: "01M2D5HKQP001DRONE0000T1",
  T2: "01M2D5HKQP001DRONE0000T2",
  T3: "01M2D5HKQP001DRONE0000T3",
  T4: "01M2D5HKQP001DRONE0000T4",
  T5: "01M2D5HKQP001DRONE0000T5",
  T6: "01M2D5HKQP001DRONE0000T6",
  T7: "01M2D5HKQP001DRONE0000T7",
  T8: "01M2D5HKQP001DRONE0000T8",
};

/** Which model each tier resolves to. The planner picks the tier; this follows. */
export const ARC_TIERS = { difficult: "opus", medium: "sonnet", easy: "haiku" } as const;

/** One task, before anything has run it. */
function task(
  id: string,
  group: string,
  title: string,
  scope: string[],
  tier: TaskView["tier"],
  cases: string[],
  expects: string,
): TaskView {
  return {
    id,
    title,
    scope,
    expects,
    state: "open",
    touched_after_done: false,
    group,
    concurrent_with: [],
    tier,
    model: ARC_TIERS[tier],
    treatment: "own_drone",
    cases,
    coord: { step: "implement", step_attempt: 1, group, group_attempt: 1, task: id },
  };
}

/** One group, before anything has run it. */
function group(
  id: string,
  ordinal: number,
  tasks: TaskView[],
  checks: string[],
  boundary: string[],
  concurrent = false,
): GroupView {
  return {
    id,
    ordinal,
    tasks,
    scope: [...new Set(tasks.flatMap((one) => one.scope))],
    concurrent,
    state: "pending",
    checks_selected: checks,
    cases_at_boundary: boundary,
    retry_count: 0,
  };
}

/**
 * The plan as the `plan` step recorded it: four groups, in the order they run.
 *
 * Group three's two tasks run at the same time and write different files —
 * concurrent tasks share one checkout, so a group whose tasks overlapped on a
 * path would be a plan that cannot run.
 */
export function arcGroups(): GroupView[] {
  const rust = checkNames(RUST_CHECKS);
  const bridge = checkNames(BRIDGE_CHECKS);
  const t5 = task(
    "T5",
    "g3",
    "Draw what is running, in four lists",
    ["packages/screens/src/Running.tsx"],
    "difficult",
    ["c-panel"],
    "The panel lists Drones, Checks, Judge calls and proposer calls",
  );
  const t6 = task(
    "T6",
    "g3",
    "Open a Drone's Job from its row",
    ["packages/screens/src/running-rows.tsx"],
    "medium",
    ["c-panel"],
    "Pressing a Drone's row opens that Job",
  );
  t5.concurrent_with = ["T6"];
  t6.concurrent_with = ["T5"];

  return [
    group(
      "g1",
      1,
      [
        task(
          "T1",
          "g1",
          "Serve one read of everything running",
          ["crates/api/src/running.rs", "crates/ipc/operations.toml"],
          "difficult",
          ["c-api"],
          "One read answers every Drone, Check and Judge call the machine has out",
        ),
        task(
          "T2",
          "g1",
          "Send an event when what is running changes",
          ["crates/fleet/src/running.rs"],
          "medium",
          ["c-api"],
          "A Drone starting or ending moves the read without a poll",
        ),
      ],
      rust,
      ["c-api"],
    ),
    group(
      "g2",
      2,
      [
        task(
          "T3",
          "g2",
          "Reword the stat to one running beside the most",
          ["packages/screens/src/overview.ts"],
          "easy",
          ["c-overview"],
          "The stat reads one running and two at most",
        ),
        task(
          "T4",
          "g2",
          "Say the same words on the Board and in Settings",
          ["packages/screens/src/Board.tsx", "packages/screens/src/SettingsSurface.tsx"],
          "easy",
          ["c-board"],
          "Neither surface spells the pair with of",
        ),
      ],
      bridge,
      ["c-board"],
    ),
    group("g3", 3, [t5, t6], bridge, [], true),
    group(
      "g4",
      4,
      [
        task(
          "T7",
          "g4",
          "Say what holds the next Drone back",
          ["packages/screens/src/running-rows.tsx", "packages/screens/src/overview.ts"],
          "medium",
          ["c-overview", "c-panel"],
          "While something is queued the panel says what the hold is",
        ),
        task(
          "T8",
          "g4",
          "Four stories: nothing out, one Drone, three at once, and the most",
          ["packages/screens/src/Running.stories.tsx"],
          "medium",
          ["c-panel"],
          "Each of the four draws without a Fleet behind it",
        ),
      ],
      bridge,
      ["c-overview", "c-panel"],
    ),
  ];
}

/** The plan with one group changed, everything else untouched. */
export function withGroup(
  groups: GroupView[],
  id: string,
  change: Partial<GroupView>,
): GroupView[] {
  return groups.map((one) => (one.id === id ? { ...one, ...change } : one));
}

/** The plan with one task changed, wherever in the plan it sits. */
export function withTask(
  groups: GroupView[],
  id: string,
  change: Partial<TaskView>,
): GroupView[] {
  return groups.map((one) => ({
    ...one,
    tasks: one.tasks.map((each) => (each.id === id ? { ...each, ...change } : each)),
  }));
}

/**
 * A task its own agent finished. **The cost arrives here and not at the
 * group's boundary** (owner, 22 Sep 2026): it reaches Armada on the session's
 * last line, which is this instant and not the Checks'.
 */
export function finished(turns: number, costMicros: number, shown: string): Partial<TaskView> {
  return { state: "done", turns, cost_micros: costMicros, shown };
}

/** A task whose agent is still on it: turns, and no cost to read yet. */
export function working(id: string, turns: number): Partial<TaskView> {
  return { state: "working", turns, drone_id: ARC_DRONES[id] };
}

/**
 * The cases the plan owes, from the planner's scope and each Drone's
 * declaration combined. **A case with no spec reads as not covered**, never as
 * one that passed by having nothing to run.
 */
export function arcCases(): CaseView[] {
  return [
    {
      id: "c-api",
      spec: "crates/api/src/tests/running.rs",
      covers: ["crates/api/src/running.rs", "crates/fleet/src/running.rs"],
      has_spec: true,
      tasks: ["T1", "T2"],
      groups: ["g1"],
      state: "owed",
    },
    {
      id: "c-overview",
      spec: "packages/screens/src/overview.test.ts",
      covers: ["packages/screens/src/overview.ts"],
      has_spec: true,
      // Two groups touch `overview.ts`, so it runs at the end of the last of
      // them and again at handoff (#1530, 21 Sep).
      tasks: ["T3", "T7"],
      groups: ["g4"],
      state: "owed",
    },
    {
      id: "c-panel",
      spec: "packages/screens/src/Running.test.tsx",
      covers: [
        "packages/screens/src/Running.tsx",
        "packages/screens/src/running-rows.tsx",
      ],
      has_spec: true,
      tasks: ["T5", "T6", "T7"],
      groups: ["g4"],
      state: "owed",
    },
    {
      id: "c-board",
      spec: "packages/screens/src/Board.test.tsx",
      covers: ["packages/screens/src/Board.tsx"],
      has_spec: false,
      tasks: ["T4"],
      groups: ["g2"],
      state: "owed",
    },
  ];
}
