// A group of tasks, and the boundary its Checks run at. Draft, for
// `crates/ipc/src/detail/step.rs`.
//
// Source of truth today: `StepDetail` — `checks`, `check_runs`, `last_verdict`,
// `attempts` and `state`. A group is the unit **between** a step and a task,
// and the wire has no such unit, so this is the draft with the least on the
// wire under it.
//
// **Checks run at each group's end** (#1530, 21 Sep), which is why the group
// and not the task is what carries a verdict, a commit and a retry count.

import type { JobDetail } from "@armada/protocol";

import { derivedGroupId, stepAttemptOf } from "./coord";
import type { LedgerRow } from "./ledger";
import { taskViewsOf, type TaskView } from "./task";

/**
 * Where a group is. Eight values, and none of them is a Job status: a group is
 * inside one step of one Job, and the Job's own machine does not move when a
 * group does.
 */
export type GroupState =
  | "pending"
  | "running"
  | "joining"
  | "checking"
  | "passed"
  | "failed"
  | "retrying"
  | "landed";

/** One group of a step's work. */
export type GroupView = {
  /** Stable for the life of the plan. Never the ordinal — see `RunCoord`. */
  id: string;
  /** Its position in the step, counted from one. Moves when a group does. */
  ordinal: number;
  tasks: TaskView[];
  /** Every path the group's tasks claim, de-duplicated, in first-seen order. */
  scope: string[];
  /** Whether its tasks run at the same time. */
  concurrent: boolean;
  state: GroupState;
  /** The Checks selected to run at this group's boundary, by name. */
  checks_selected: string[];
  /**
   * The cases that run at this boundary, by id. **A case runs at the boundary
   * of the last group holding a task it covers**, and again at handoff
   * (#1530, 21 Sep).
   */
  cases_at_boundary: string[];
  /**
   * When the group started and ended.
   *
   * **Nothing on the wire times a group** — a step is timed and a group sits
   * between a step and a task, so both are absent until #1545 decides whether
   * Fleet records them. A surface says it is not timed rather than drawing a
   * dash or a zero.
   */
  started_at?: string;
  ended_at?: string;
  /** What the boundary came to. Absent until the Checks have run. */
  verdict?: string;
  /** How many times this group has been run again after a failure. */
  retry_count: number;
  /** The commit the group left, where it left one. */
  commit?: string;
};

/**
 * Today's wire has no groups, so **one task per group, in plan order**, and the
 * group takes the step's own state and Checks.
 *
 * That is what makes a board built on this render against the real Fleet: a
 * Job with six tasks draws six groups of one rather than nothing at all.
 */
export function taskGroupsOf(detail: JobDetail): GroupView[] {
  const step = detail.steps.find(
    (candidate) => candidate.step_id === detail.job.current_step_id,
  );
  const attempts = stepAttemptOf(step);
  // `DeclaredCheck.name` is absent on `diff_nonempty`, which names no Manifest
  // Check, so the kind stands in for it rather than an empty row appearing.
  const checks = (step?.checks ?? []).map((check) => check.name ?? check.kind);
  const state =
    step?.checking !== undefined ? "checking" : groupStateOf(step?.state);
  const commit = detail.delivery?.commit;

  return taskViewsOf(detail).map((task, index) => {
    const group: GroupView = {
      id: derivedGroupId(task.id),
      ordinal: index + 1,
      tasks: [task],
      scope: [...task.scope],
      concurrent: false,
      state: task.state === "done" ? "passed" : state,
      checks_selected: checks,
      cases_at_boundary: [],
      // A step's `attempts` counts runs of the step, and with one group per
      // step a second run of the step is a second run of every group in it.
      retry_count: attempts - 1,
    };
    if (step?.last_verdict !== undefined) group.verdict = step.last_verdict.named;
    if (commit !== undefined) group.commit = commit;
    return group;
  });
}

/**
 * The groups, timed by what the Record says happened inside each one.
 *
 * **The first and last row recorded at a group, and nothing invented**: a task
 * carries no instants either, so this is the only source a board has. A group
 * with no rows of its own keeps both fields absent and reads as not timed.
 */
export function groupsTimedBy(groups: GroupView[], rows: readonly LedgerRow[]): GroupView[] {
  return groups.map((group) => {
    const at = rows
      .filter((row) => row.coord?.group === group.id)
      .map((row) => row.at)
      .sort();
    const first = at[0];
    const last = at[at.length - 1];
    if (first === undefined || last === undefined || first === last) return group;
    return { ...group, started_at: first, ended_at: last };
  });
}

// A step state is the nearest thing the wire has to a group state, and the two
// sets are not the same. `domain/step-states.toml` declares six — `advanced`,
// `awaiting_human`, `not_started`, `retrying`, `running`, `stopped` — and none
// of them is `joining`, `checking` or `landed`. `checking` is read off
// `StepDetail.checking` above rather than out of this set.
function groupStateOf(stepState: string | undefined): GroupState {
  switch (stepState) {
    case "running":
    case "awaiting_human":
      return "running";
    case "advanced":
      return "passed";
    case "stopped":
      return "failed";
    case "retrying":
      return "retrying";
    default:
      return "pending";
  }
}
