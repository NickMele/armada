// The implement step, opened — pure, so every sentence the board draws is
// tested without a browser. `#1536`.
//
// **The sentences are `tab-plan-read.ts`'s wherever that file already wrote
// one.** A group's state in words, a task's spend, the retry count and the flag
// naming the later task are all the same claim read on two destinations, and a
// second spelling of any of them is the drift `lib/job-states.js` was deleted
// for. What is written here is what only a running step says: what the boundary
// came to, what stopping it forbids, and what the next Drone is told.

import type {
  GroupBoundaryCheck,
  GroupBoundaryProps,
  ImplementBoardProps,
  ImplementGroup,
  ImplementTask,
} from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import type { CaseView } from "./draft/cases";
import type { GroupState, GroupView } from "./draft/group";
import type { TaskView } from "./draft/task";
import {
  besideSaid,
  boundarySaid,
  caseReads,
  dronesAtOnceSaid,
  failedChecksOf,
  groupSaid,
  markOf,
  retrySaid,
  runBySaid,
  spentSaid,
  testsSaid,
  touchedByOf,
} from "./tab-plan-read";

/** Why no case is drawn at a boundary. Fleet serves none, which is not "none owed". */
export const NO_CASES_AT_BOUNDARY =
  "Fleet does not serve the cases a boundary owes yet, so none is drawn here. " +
  "What runs at this boundary is the Checks above.";

/** Whether a group's boundary has already run. */
function hasRun(state: GroupState): boolean {
  return state === "passed" || state === "failed" || state === "retrying" || state === "landed";
}

/**
 * What one Check at this boundary reads as.
 *
 * **Attributed by the group's own state and never by the run alone.** A step's
 * `check_runs` is one list for every group in it, so a passed group would
 * otherwise take a later group's red.
 */
function checkReads(
  group: GroupView,
  name: string,
  failed: readonly string[],
): GroupBoundaryCheck["reads"] {
  if (failed.includes(name)) return "failed";
  if (group.state === "checking") return "running";
  return hasRun(group.state) ? "passed" : "not run";
}

/**
 * What the boundary came to, in words. **Absent until it has run** — a
 * boundary nothing reached says nothing rather than `0 failed`.
 */
export function verdictSaid(group: GroupView, failed: readonly string[]): string | undefined {
  if (group.state === "checking") return "running now";
  if (!hasRun(group.state)) return undefined;
  if (failed.length > 0) return `${failed.join(", ")} failed`;
  const passed = group.checks_selected.length;
  return passed === 0 ? undefined : `all ${passed} passed`;
}

/**
 * What stopping this group forbids, named with the group it holds back.
 * Absent where the boundary passed, and on the last group, which holds nothing.
 */
export function stopsSaid(group: GroupView, next: GroupView | undefined): string | undefined {
  if (group.state !== "failed" && group.state !== "retrying") return undefined;
  if (next === undefined) return "This is the last group, so nothing is waiting behind it.";
  return `No task of group ${next.ordinal} starts until this boundary passes.`;
}

/**
 * What the next Drone is told: the failed Check's own output, verbatim.
 *
 * **Never a summary.** A retry working from a paraphrase is a retry working
 * from something nobody can check against the run that produced it.
 */
export function toldNextOf(step: StepDetail | undefined, failed: readonly string[]): string | undefined {
  const run = (step?.check_runs ?? []).find((one) => failed.includes(one.name) && one.outcome === "failed");
  if (run === undefined) return undefined;
  const lines = [run.expected, run.produced].filter((one): one is string => one !== undefined);
  return lines.length === 0 ? undefined : lines.join("\n");
}

/**
 * `2 tasks, at the same time` — fan out — or `2 tasks, one after another`.
 *
 * **A group running at once carries the Job's Drone cap** (`#1550`), because
 * that number is what bounds the fan out and this is the only place on the
 * screen where it decides anything a person can see.
 */
export function shapeSaid(group: GroupView, droneCap?: number): string {
  const many = `${group.tasks.length} ${group.tasks.length === 1 ? "task" : "tasks"}`;
  if (group.tasks.length === 1) return `${many}, on its own`;
  if (!group.concurrent) return `${many}, one after another`;
  const cap = dronesAtOnceSaid(droneCap);
  return cap === undefined ? `${many}, at the same time` : `${many}, at the same time \u00b7 ${cap}`;
}

/** `difficult · opus · its own agent`. The planner's tier, and what it resolved to. */
export function tierSaid(task: TaskView): string {
  return `${task.tier} · ${task.model} · ${runBySaid(task)}`;
}

/** One task's row, as the running board draws it. */
function taskRowOf(task: TaskView, touchedBy: Map<string, string>): ImplementTask {
  const beside = besideSaid(task);
  const spent = spentSaid(task);
  const later = touchedBy.get(task.id);
  return {
    id: task.id,
    title: task.title,
    mark: markOf(task.state),
    says: tierSaid(task),
    ...(spent === undefined ? {} : { spentSays: spent }),
    ...(beside === undefined ? {} : { besideSays: beside }),
    ...(later === undefined ? {} : { touchedSays: `touched later · ${later}` }),
    ...(task.failed_reason === undefined ? {} : { failedReason: task.failed_reason }),
  };
}

/** One group's boundary — the Checks bar, and the tests region kept apart. */
export function boundaryOf(
  group: GroupView,
  next: GroupView | undefined,
  cases: readonly CaseView[],
  whole: JobWhole | null,
  step: StepDetail | undefined,
): GroupBoundaryProps {
  const failed = failedChecksOf(whole, group);
  const checks = group.checks_selected.map((name) => ({
    name,
    reads: checkReads(group, name, failed),
  }));
  const verdict = verdictSaid(group, failed);
  const retry = retrySaid(group.retry_count);
  const stops = stopsSaid(group, next);
  const told = toldNextOf(step, failed);
  const atBoundary = cases.filter((one) => one.groups.includes(group.id));
  const tests = atBoundary.map((one) => ({ id: one.id, spec: one.spec, reads: caseReads(one) }));
  const testsSay = testsSaid(group.state, tests.length);
  return {
    says: boundarySaid(group.state, checks.length),
    checks,
    checksAbsent: "No Check runs at this group's end.",
    ...(verdict === undefined ? {} : { verdictSays: verdict }),
    ...(failed.length > 0
      ? { verdictNamed: "failed" as const }
      : hasRun(group.state)
        ? { verdictNamed: "passed" as const }
        : {}),
    ...(retry === undefined ? {} : { retrySays: retry }),
    ...(group.commit === undefined ? {} : { commit: group.commit }),
    ...(stops === undefined ? {} : { stopsSays: stops }),
    ...(told === undefined ? {} : { toldNext: told }),
    ...(testsSay === undefined ? {} : { testsSay }),
    ...(tests.length === 0 ? {} : { tests }),
    testsAbsent: NO_CASES_AT_BOUNDARY,
  };
}

/** Which groups open themselves: the one that is moving, or the one that broke. */
export function groupsThatOpen(groups: readonly GroupView[]): string[] {
  const moving = groups.filter((one) => one.state !== "pending" && one.state !== "passed" && one.state !== "landed");
  if (moving.length > 0) return moving.map((one) => one.id);
  const last = groups[groups.length - 1];
  return last === undefined ? [] : [last.id];
}

export type ImplementBoardReading = {
  whole: JobWhole | null;
  groups: readonly GroupView[];
  cases: readonly CaseView[];
  /** The step the groups hang under, off `workflow-canvas.ts`'s own rule. */
  step: StepDetail | undefined;
  openGroups: readonly string[];
  onOpenGroup: (groupId: string) => void;
  openTaskId?: string;
  onOpenTask: (taskId: string) => void;
  /** How many Drones this Job may run at once, where the gate settled one. */
  droneCap?: number;
};

/**
 * The whole board. `undefined` where the plan holds no group, which is a step
 * with nothing to open rather than an empty board.
 */
export function implementBoardOf({
  whole,
  groups,
  cases,
  step,
  openGroups,
  onOpenGroup,
  openTaskId,
  onOpenTask,
  droneCap,
}: ImplementBoardReading): ImplementBoardProps | undefined {
  if (groups.length === 0 || step === undefined) return undefined;
  const touchedBy = touchedByOf(groups);
  const drawn: ImplementGroup[] = groups.map((group, at) => ({
    id: group.id,
    ordinal: group.ordinal,
    state: group.state,
    says: groupSaid(group.state),
    shapeSays: shapeSaid(group, droneCap),
    ...(group.commit === undefined ? {} : { commit: group.commit }),
    tasks: group.tasks.map((task) => taskRowOf(task, touchedBy)),
    boundary: boundaryOf(group, groups[at + 1], cases, whole, step),
  }));
  return {
    stepName: step.label,
    groups: drawn,
    openGroups,
    onOpenGroup,
    ...(openTaskId === undefined ? {} : { openTaskId }),
    onOpenTask,
  };
}
