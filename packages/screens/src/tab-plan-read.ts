// The Plan destination's own read of a Job — pure, so the arithmetic and every
// sentence on the board are tested without a browser. `#1535`.
//
// **Every word the board draws is written here.** `PlanBoard` composes no
// prose, for `StepBar`'s reason: a count in words is copy, and copy has one
// owner. So does the rule that decides it.

import type { PlanBoardAsk, PlanBoardGroup, PlanBoardProps, PlanBoardTask, PlanBoardTest } from "@armada/components";
import type { PlanTaskSheetProps, PlanTaskTest, TaskMarkState } from "@armada/components";
import type { JobDetail, StepDetail } from "@armada/protocol";

import { caseViewsOf, scopeRevisionsOf, type CaseView, type ScopeRevisionView } from "./draft/cases";
import { criterionViewsOf, type CriterionView } from "./draft/criterion";
import { taskGroupsOf, type GroupState, type GroupView } from "./draft/group";
import type { JobDraft } from "./draft/held";
import { planRevisionsOf, type PlanAskKind, type PlanRevisionView } from "./draft/revision";
import type { TaskView } from "./draft/task";
import { money } from "./facts";

/** How the groups run. One line, and the one thing the order itself says. */
export const ORDER_SAID = "The groups run one at a time, in this order.";

/**
 * The plan's groups. **The draft where a moment carries one, and today's wire
 * derived otherwise** — `taskGroupsOf` draws one task per group, so a board
 * built here renders against the real Fleet thinner rather than empty.
 */
export function groupsOf(whole: JobDetail | null, draft?: JobDraft): GroupView[] {
  if (draft?.groups !== undefined) return [...draft.groups];
  return whole === null ? [] : taskGroupsOf(whole);
}

/** The cases, on the same terms: the draft's, or the specs the Job's Drones named. */
export function casesOf(whole: JobDetail | null, draft?: JobDraft): CaseView[] {
  if (draft?.cases !== undefined) return [...draft.cases];
  return whole === null ? [] : caseViewsOf(whole);
}

/** What the Job is held to, on the same terms. */
export function criteriaOf(whole: JobDetail | null, draft?: JobDraft): CriterionView[] {
  if (draft?.criteria !== undefined) return [...draft.criteria];
  return whole === null ? [] : criterionViewsOf(whole);
}

/** The changes asked of the plan's Drone, on the same terms. */
export function scopeRevisionsCarriedBy(
  whole: JobDetail | null,
  draft?: JobDraft,
): ScopeRevisionView[] {
  if (draft?.scope_revisions !== undefined) return [...draft.scope_revisions];
  return whole === null ? [] : scopeRevisionsOf(whole);
}

/** Each change asked for, paired with the Judge's answer to it. */
export function revisionsOf(
  whole: JobDetail | null,
  draft: JobDraft | undefined,
  step: StepDetail | undefined,
): PlanRevisionView[] {
  const groups = groupsOf(whole, draft);
  return planRevisionsOf(
    step,
    scopeRevisionsCarriedBy(whole, draft),
    casesOf(whole, draft),
    groups,
    criteriaOf(whole, draft),
  );
}

/** Every task of every group, in the order the groups run. */
export function tasksOf(groups: readonly GroupView[]): TaskView[] {
  return groups.flatMap((group) => group.tasks);
}

/**
 * Where a group is, in words. **`failed` says what failed** — a group that
 * stopped at its boundary and one whose tasks broke are two different
 * readings, and the boundary is the one this word is about.
 */
export function groupSaid(state: GroupState): string {
  switch (state) {
    case "pending":
      return "not started";
    case "running":
      return "working";
    case "joining":
      return "joining its work";
    case "checking":
      return "running its checks";
    case "passed":
      return "passed";
    case "failed":
      return "failed at its checks";
    case "retrying":
      return "failed at its checks, running again";
    case "landed":
      return "landed";
  }
}

/** Whether a group's boundary has already run. The tense every sentence takes. */
function hasRun(state: GroupState): boolean {
  return state === "passed" || state === "failed" || state === "retrying" || state === "landed";
}

/** `7 checks will run at this boundary`, in the tense the group's state earns. */
export function boundarySaid(state: GroupState, checks: number): string {
  const many = `${checks} ${checks === 1 ? "check" : "checks"}`;
  if (state === "checking") return `${many} are running at this boundary`;
  if (hasRun(state)) return `${many} ran at this boundary`;
  return `${many} will run at this boundary`;
}

/** `1 test runs at this boundary`. Nothing where none does. */
export function testsSaid(state: GroupState, tests: number): string | undefined {
  if (tests === 0) return undefined;
  const many = `${tests} ${tests === 1 ? "test" : "tests"}`;
  if (hasRun(state)) return `${many} ran at this boundary`;
  return `${many} ${tests === 1 ? "runs" : "run"} at this boundary`;
}

/**
 * What a case reads as. **A case with no spec is `not covered`**, never green
 * and never a pass by having nothing to run — `#1530`, 21 Sep.
 */
export function caseReads(one: CaseView): PlanBoardTest["reads"] {
  if (one.state === "dropped") return "dropped";
  return one.has_spec ? "owed" : "not covered";
}

/** What dropped a case, as one line. Nothing where it is still owed. */
export function droppedSaid(one: CaseView): string | undefined {
  const by = one.dropped_by;
  if (by === undefined) return undefined;
  if (by.dropped === "scope_revision") return "dropped by a scope revision";
  const where = by.coord.task ?? by.coord.group ?? by.coord.step;
  return `dropped on a retry of ${where}`;
}

/** One case as the board and the inspector both draw it. */
function testOf(one: CaseView): PlanBoardTest {
  const dropped = droppedSaid(one);
  return {
    id: one.id,
    spec: one.spec,
    reads: caseReads(one),
    ...(dropped === undefined ? {} : { droppedSays: dropped }),
  };
}

/**
 * What a task has spent. **Turns while it runs, and the cost only once its own
 * agent stopped** — a live figure would be invented, since cost reaches Armada
 * on a session's last line (`#1530`, 22 Sep).
 */
export function spentSaid(task: TaskView): string | undefined {
  const parts: string[] = [];
  if (task.turns !== undefined) parts.push(`${task.turns} turns`);
  if (task.cost_micros !== undefined) parts.push(money(task.cost_micros));
  return parts.length === 0 ? undefined : parts.join(" · ");
}

/** How a task is run, in words a person reads rather than the wire's enum. */
export function runBySaid(task: TaskView): string {
  switch (task.treatment) {
    case "own_drone":
      return "its own agent";
    case "job":
      return "a Job of its own";
    default:
      return "the step's Drone";
  }
}

/** `runs beside T5`. Nothing where the task runs alone. */
export function besideSaid(task: TaskView): string | undefined {
  return task.concurrent_with.length === 0
    ? undefined
    : `runs beside ${task.concurrent_with.join(", ")}`;
}

/**
 * Which later task reached into a finished one's files, by task id.
 *
 * **Derived rather than served.** `touched_after_done` says a later task
 * edited this one's file and never which; the plan's own order and scopes say
 * it exactly, and a flag naming nobody is a flag a person cannot act on.
 */
export function touchedByOf(groups: readonly GroupView[]): Map<string, string> {
  const order = tasksOf(groups);
  const found = new Map<string, string>();
  order.forEach((task, at) => {
    if (!task.touched_after_done) return;
    const claimed = new Set(task.scope);
    const later = order
      .slice(at + 1)
      .find((candidate) => candidate.scope.some((path) => claimed.has(path)));
    if (later !== undefined) found.set(task.id, later.id);
  });
  return found;
}

/**
 * Files two groups both claim. **A warning line and not a refusal** — the plan
 * is legal, and what it costs is a task finished once and edited again, which
 * is what `touched_after_done` then flags.
 */
export function clashesOf(groups: readonly GroupView[]): { path: string; says: string }[] {
  const claims = new Map<string, { group: number; task: string }[]>();
  for (const group of groups) {
    for (const task of group.tasks) {
      for (const path of task.scope) {
        const held = claims.get(path) ?? [];
        if (!held.some((one) => one.group === group.ordinal)) {
          held.push({ group: group.ordinal, task: task.id });
        }
        claims.set(path, held);
      }
    }
  }
  const clashes: { path: string; says: string }[] = [];
  for (const [path, held] of claims) {
    if (held.length < 2) continue;
    clashes.push({
      path,
      says: held.map((one) => `group ${one.group} (${one.task})`).join(" and "),
    });
  }
  return clashes;
}

/**
 * What may be asked of the Drone about a group, and what each control is
 * called. **A plan is a record, so these are requests** — the verb is what a
 * person wants, and `#1552` is why none of them edits anything.
 */
export const ASK_LABEL: Record<PlanAskKind, string> = {
  move_up: "Move up",
  move_down: "Move down",
  remove: "Remove",
  rewrite: "Rewrite this task",
};

/** The standing line over the group controls. Once, above the cards. */
export const ASKS_SAY =
  "The plan is the Drone's record. Each of these asks it for a different split, and it may refuse.";

/**
 * The rewrite ask's own words. **Prose, because a rewrite is not a field** —
 * what a task should be instead is a sentence the Drone reads, and a picker
 * for each of a task's parts would be editing the record.
 */
export const REWRITE_ASK = {
  label: ASK_LABEL.rewrite,
  lead: "Say what this task should be instead. The Drone that wrote the plan decides, and it may refuse.",
  placeholder: "Take the panel's rows out of this one and give them a task of their own",
  send: "Ask the Drone",
} as const;

/**
 * The asks one group offers. **The first group cannot move up and the last
 * cannot move down**, and both are drawn off rather than left out: a card
 * whose controls change place as it moves is a card a person has to re-read.
 */
export function asksOf(groups: readonly GroupView[], at: number): PlanBoardAsk[] {
  return [
    { id: "move_up", label: ASK_LABEL.move_up, disabled: at === 0 },
    { id: "move_down", label: ASK_LABEL.move_down, disabled: at === groups.length - 1 },
    { id: "remove", label: ASK_LABEL.remove },
  ];
}

/**
 * Where a reorder would contradict the tasks' declared files.
 *
 * **The one mechanical catch there is** (`#1552`). Two groups claiming one
 * path have an order between them that their own scopes decide, so swapping
 * that pair is the ask whose consequence can be computed rather than guessed.
 * Everything else about a split — why these tasks, why this size — is the
 * Drone's to answer, which is what the ask is for.
 *
 * **A warning and not a refusal**, on `clashesOf`'s terms: the reorder is
 * legal and what it costs is a file written in the other order.
 */
export function reorderWarning(
  groups: readonly GroupView[],
  groupId: string,
  ask: PlanAskKind,
): string | undefined {
  if (ask !== "move_up" && ask !== "move_down") return undefined;
  const at = groups.findIndex((one) => one.id === groupId);
  const other = ask === "move_up" ? at - 1 : at + 1;
  if (at < 0 || other < 0 || other >= groups.length) return undefined;
  const moving = groups[at]!;
  const passed = groups[other]!;
  const claimed = new Set(passed.scope);
  const shared = moving.scope.filter((path) => claimed.has(path));
  if (shared.length === 0) return undefined;
  const first = at < other ? moving : passed;
  return `Group ${moving.ordinal} and group ${passed.ordinal} both claim ${shared.join(", ")}. Group ${first.ordinal} writes it first as the plan stands, and this ask reverses that.`;
}

/** The mark a task's row leads with. `TaskMark` prints no word beside it. */
export function markOf(state: TaskView["state"]): TaskMarkState {
  return state;
}

function taskRowOf(task: TaskView, touchedBy: Map<string, string>): PlanBoardTask {
  const beside = besideSaid(task);
  const spent = spentSaid(task);
  const later = touchedBy.get(task.id);
  return {
    id: task.id,
    title: task.title,
    mark: markOf(task.state),
    tier: task.tier,
    model: task.model,
    ...(beside === undefined ? {} : { besideSays: beside }),
    ...(spent === undefined ? {} : { spentSays: spent }),
    ...(later === undefined ? {} : { touchedSays: `touched later · ${later}` }),
    ...(task.failed_reason === undefined ? {} : { failedReason: task.failed_reason }),
  };
}

/**
 * Which Checks failed at a group's boundary, by name.
 *
 * Read off the step's own `check_runs`, because that is where a Check result
 * lives on today's wire. **Only a group that is carrying the failure takes
 * it** — the step's runs are one list for every group in it, so attributing
 * them by run alone would paint a passed group with another's red.
 */
export function failedChecksOf(whole: JobDetail | null, group: GroupView): string[] {
  const carrying = group.state === "failed" || group.state === "retrying";
  if (whole === null || !carrying) return [];
  const step = whole.steps.find((one) => one.step_id === whole.job.current_step_id);
  const runs = step?.check_runs ?? [];
  const latest = Math.max(0, ...runs.map((run) => run.attempt));
  return runs
    .filter((run) => run.attempt === latest && run.outcome === "failed")
    .map((run) => run.name);
}

/**
 * How many times a group has been run. **Nothing on its first run** — a count
 * of one would read as a retry that has not happened.
 */
export function retrySaid(retries: number): string | undefined {
  if (retries <= 0) return undefined;
  if (retries === 1) return "second run";
  if (retries === 2) return "third run";
  return `run ${retries + 1}`;
}

/** One group's card, sentences and all. */
export function groupCardOf(
  group: GroupView,
  cases: readonly CaseView[],
  touchedBy: Map<string, string>,
  whole: JobDetail | null,
  asks: readonly PlanBoardAsk[] = [],
): PlanBoardGroup {
  const atBoundary = cases.filter((one) => one.groups.includes(group.id));
  const tests = atBoundary.map(testOf);
  const saidTests = testsSaid(group.state, tests.length);
  const failed = failedChecksOf(whole, group);
  const retry = retrySaid(group.retry_count);
  return {
    id: group.id,
    ordinal: group.ordinal,
    state: group.state,
    says: groupSaid(group.state),
    scope: group.scope,
    tasks: group.tasks.map((task) => taskRowOf(task, touchedBy)),
    boundarySays: boundarySaid(group.state, group.checks_selected.length),
    checks: group.checks_selected,
    ...(failed.length === 0 ? {} : { checksFailed: failed }),
    ...(saidTests === undefined ? {} : { testsSay: saidTests }),
    ...(tests.length === 0 ? {} : { tests }),
    ...(retry === undefined ? {} : { retrySays: retry }),
    ...(group.commit === undefined ? {} : { commit: group.commit }),
    ...(group.concurrent
      ? { concurrentSays: `${group.tasks.length} tasks run at the same time` }
      : {}),
    ...(asks.length === 0 ? {} : { asks }),
  };
}

/**
 * The whole board, from a Job and whatever draft the moment carries.
 *
 * `revisable` is whether the plan may still be asked about — the step that
 * recorded it is waiting on a person, and nothing else is out. **A plan past
 * its gate draws no controls at all**: it is a record then, and a control
 * that sends an ask nobody will answer is worse than none.
 */
export function planBoardOf(
  whole: JobDetail | null,
  draft: JobDraft | undefined,
  onOpenTask: (taskId: string) => void,
  openTaskId?: string,
  revisable = false,
): PlanBoardProps | undefined {
  const groups = groupsOf(whole, draft);
  if (groups.length === 0) return undefined;
  const cases = casesOf(whole, draft);
  const touchedBy = touchedByOf(groups);
  const clashes = clashesOf(groups);
  return {
    approach: whole?.work_plan?.approach ?? "",
    orderSays: ORDER_SAID,
    groups: groups.map((group, at) =>
      groupCardOf(group, cases, touchedBy, whole, revisable ? asksOf(groups, at) : []),
    ),
    ...(clashes.length === 0 ? {} : { clashes }),
    ...(revisable ? { asksSay: ASKS_SAY } : {}),
    ...(openTaskId === undefined ? {} : { openTaskId }),
    onOpenTask,
  };
}

/**
 * The inspector's own reading of one task. `undefined` where the plan holds no
 * task by that id, which is a sheet that should not be open.
 */
export function taskSheetOf(
  taskId: string,
  groups: readonly GroupView[],
  cases: readonly CaseView[],
  touchedBy: Map<string, string>,
): Omit<PlanTaskSheetProps, "open"> | undefined {
  const task = tasksOf(groups).find((one) => one.id === taskId);
  if (task === undefined) return undefined;
  const owed: PlanTaskTest[] = cases
    .filter((one) => one.tasks.includes(task.id) || task.cases.includes(one.id))
    .map(testOf);
  const note = noteWithFlag(task.note, touchedBy.get(task.id));
  return {
    id: task.id,
    title: task.title,
    state: markOf(task.state),
    scope: task.scope,
    tier: task.tier,
    model: task.model,
    runBy: runBySaid(task),
    beside: task.concurrent_with,
    tests: owed,
    ...(note === undefined ? {} : { note }),
    ...(task.expects === undefined ? {} : { expects: task.expects }),
    ...(task.shown === undefined ? {} : { shown: task.shown }),
    ...(task.reason === undefined ? {} : { reason: task.reason }),
    ...(task.failed_reason === undefined ? {} : { failedReason: task.failed_reason }),
  };
}

/** The flag, carried into the inspector's own brief rather than lost with the row. */
function noteWithFlag(note: string | undefined, later: string | undefined): string | undefined {
  if (later === undefined) return note;
  const flag = `${later} edited a file this task had already finished.`;
  return note === undefined ? flag : `${note} ${flag}`;
}
