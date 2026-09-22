// A change a person asked the plan's Drone for, and what came back. Draft, for
// `crates/ipc/src/detail/step.rs`.
//
// Source of truth today: the plan step's `attempts` and `judged`, paired with
// the `ScopeRevisionView`s the plan carries. The wire has no revision act — a
// plan is recorded whole and approved whole — so this is the draft with the
// least under it after `GroupView` itself.
//
// **A revision is a request, and the Drone may refuse it** (#1552). The plan
// is a record the Drone wrote, so a person moving a group or narrowing a task
// is asking for a different split rather than editing a row. What comes back
// is the Judge's answer on the step that recorded the plan, which is why the
// refusal is the Judge record's own three fields and not a shape of its own.

import type { Judged, StepDetail } from "@armada/protocol";

import type { CaseView, ScopeRevisionView } from "./cases";
import type { CriterionView } from "./criterion";
import type { GroupView } from "./group";

/** What may be asked of a plan before it is approved. */
export type PlanAskKind = "move_up" | "move_down" | "remove" | "rewrite";

/**
 * What came back. **`asked` is a request still out** — the Drone has been told
 * and the step has not been run again, which is neither a yes nor a no.
 */
export type PlanAnswer = "asked" | "taken" | "refused";

/** Why the Drone would not take it, as the Judge record's own fields. */
export type PlanRefusal = {
  /** The criterion the Judge answered, in the words the Job is held to. */
  criterion?: string;
  expected?: string;
  produced?: string;
  consequence?: string;
};

/** One change asked for, and the answer to it. */
export type PlanRevisionView = {
  at: string;
  kind: PlanAskKind;
  /** The group it was asked on. Absent on a rewrite. */
  group?: string;
  /** The task it was asked on. Absent on a group ask. */
  task?: string;
  /** What was asked, in words. */
  says: string;
  answer: PlanAnswer;
  /** Present on `refused` and on nothing else. */
  refusal?: PlanRefusal;
  /**
   * The cases that fell out with the ask. **Named by their spec and not by
   * their id**: a case id is composed from the step that named it, and what a
   * person can act on is the file that will stop running.
   */
  dropped: { id: string; spec: string }[];
};

/**
 * Which task a scope revision touched.
 *
 * **Derived rather than served**, for `touchedByOf`'s reason: the wire says a
 * case was dropped and never by whom, and a revision naming no task is one a
 * person cannot check. A dropped case still lists every task the planner gave
 * it, so the task that no longer claims it is the task the ask narrowed —
 * exactly one, or the derivation says nothing rather than guessing.
 */
export function revisedTaskOf(
  revision: ScopeRevisionView,
  cases: readonly CaseView[],
  groups: readonly GroupView[],
): string | undefined {
  const tasks = groups.flatMap((group) => group.tasks);
  const narrowed = new Set<string>();
  for (const id of revision.cases_dropped) {
    const one = cases.find((each) => each.id === id);
    if (one === undefined) continue;
    for (const named of one.tasks) {
      const task = tasks.find((each) => each.id === named);
      if (task !== undefined && !task.cases.includes(id)) narrowed.add(named);
    }
  }
  return narrowed.size === 1 ? [...narrowed][0] : undefined;
}

/** `Take a.ts out of T5.` — what the ask changed, in one line. */
export function revisionSaid(revision: ScopeRevisionView, task: string | undefined): string {
  const on = task ?? "the plan";
  const out = revision.paths_removed.join(", ");
  const inn = revision.paths_added.join(", ");
  if (out !== "" && inn !== "") return `Take ${out} out of ${on} and put ${inn} in.`;
  if (out !== "") return `Take ${out} out of ${on}.`;
  if (inn !== "") return `Put ${inn} into ${on}.`;
  return `Rewrite ${on}.`;
}

/**
 * Which run of the plan step a revision was answered on.
 *
 * **The first run recorded the plan**, so the first revision is the second
 * run. One ask, one run: a Drone asked twice in one run would be two asks the
 * record cannot tell apart, and the record is what this reads.
 */
function attemptOf(at: number): number {
  return at + 2;
}

/** The refusal on that run, where the Judge answered one. */
function refusedOn(step: StepDetail, attempt: number): Judged | undefined {
  return (step.judged ?? []).find(
    (one) => one.attempt === attempt && one.verdict === "not_met",
  );
}

/** Whether the step has been run that many times at all. */
function ranTo(step: StepDetail, attempt: number): boolean {
  return (step.attempts ?? []).some((one) => one.attempt >= attempt);
}

/**
 * Every revision the plan carries, with the answer to each.
 *
 * `undefined` for the step is a Job whose plan step has not arrived, and then
 * nothing was asked: the revisions read as still out rather than as taken.
 */
export function planRevisionsOf(
  step: StepDetail | undefined,
  revisions: readonly ScopeRevisionView[],
  cases: readonly CaseView[],
  groups: readonly GroupView[],
  criteria: readonly CriterionView[],
): PlanRevisionView[] {
  return revisions.map((revision, at) => {
    const task = revisedTaskOf(revision, cases, groups);
    const attempt = attemptOf(at);
    const refused = step === undefined ? undefined : refusedOn(step, attempt);
    const view: PlanRevisionView = {
      at: revision.at,
      kind: "rewrite",
      says: revisionSaid(revision, task),
      answer:
        refused !== undefined
          ? "refused"
          : step !== undefined && ranTo(step, attempt)
            ? "taken"
            : "asked",
      dropped: revision.cases_dropped.map((id) => ({
        id,
        spec: cases.find((one) => one.id === id)?.spec ?? id,
      })),
    };
    if (task !== undefined) view.task = task;
    if (refused !== undefined) view.refusal = refusalOf(refused, criteria);
    return view;
  });
}

/** The Judge's answer, with the criterion spelled in the words it answered. */
function refusalOf(refused: Judged, criteria: readonly CriterionView[]): PlanRefusal {
  const criterion = criteria.find((one) => one.criterion_id === refused.criterion_id);
  const refusal: PlanRefusal = {};
  if (criterion !== undefined) refusal.criterion = criterion.text;
  if (refused.expected !== undefined) refusal.expected = refused.expected;
  if (refused.produced !== undefined) refusal.produced = refused.produced;
  if (refused.consequence !== undefined) refusal.consequence = refused.consequence;
  return refusal;
}
