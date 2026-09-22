// Test cases: what a task owes, when it was run, and who ran it. Draft, for
// `crates/ipc/src/showing.rs`.
//
// Source of truth today: `ShowAgain` on `JobDetail.show_again` — `specs`, every
// spec this Job's Drones named, and `shown`, every press a person made, each a
// `ShownSet` with its frames.
//
// **A case run is never an Evidence row, and is never derived into one**
// (#1530, 21 Sep). It carries no verdict and no evidence id, and there is no
// field here either could be put in. A case with no spec reads as **not
// covered**, never as passing.
//
// **No baseline run exists.** #602 switched the "before" half off; Fleet
// photographs the worktree alone. Nothing here draws one, and `KeptFrame.side`
// being on the wire is not permission to.

import type { JobDetail, NamedSpec, ShownSet } from "@armada/protocol";

import type { RunCoord } from "./coord";

/** Whether a task still owes this case, lost it, or gained it. */
export type CaseState = "owed" | "dropped" | "added";

/**
 * Why a case is no longer owed.
 *
 * Two shapes, because they are two different events: a person revising the
 * scope, and **a retry that declared fewer files** (#1530, 21 Sep) — the second
 * is shown as dropped on that retry alone.
 */
export type DroppedBy =
  | { dropped: "scope_revision"; at: string }
  | { dropped: "retry"; coord: RunCoord };

/** One case, and what owes it. */
export type CaseView = {
  id: string;
  /** The spec, as a repository path. */
  spec: string;
  /**
   * Which source paths it covers.
   *
   * **Read from a `COVERS` file per spec folder, in CODEOWNERS format, nearest
   * ancestor first** — #1274's decision, which is parked and unbuilt. Coverage
   * is deliberately *not* written in `armada.yml`, and Armada never reads
   * inside a test file. Empty is a case nothing said the coverage of.
   */
  covers: string[];
  /**
   * Whether the spec is in the repository. **`false` reads as not covered** and
   * never as a case that passes by having nothing to run.
   */
  has_spec: boolean;
  /**
   * The tasks that owe it, by id — from **the planner's scope and the Drone's
   * declaration combined** (#1530, 21 Sep).
   */
  tasks: string[];
  /**
   * The groups whose boundary runs it, by id. **The last group holding a task
   * it covers**, and again at handoff.
   */
  groups: string[];
  state: CaseState;
  /** Present where `state` is `dropped`, and on nothing else. */
  dropped_by?: DroppedBy;
  /** The last run of it, where there was one. */
  last_run?: CaseRunView;
};

/** Who ran a case. */
export type CaseRunActor = "fleet" | "drone" | "person" | "contributor";

/** Why it ran. */
export type CaseRunPurpose =
  | "capture"
  | "group_boundary"
  | "handoff"
  | "show_again"
  | "reviewer";

/** Which tree it ran against. */
export type CaseRunTree = "branch" | "merged";

/** What became of the run itself — never of the work. */
export type CaseRunOutcome = "ran" | "run_failed" | "not_run";

/**
 * One run of one case.
 *
 * **No verdict and no evidence id.** What a run produced is frames; whether the
 * work is right is a question a person answers from them, and a field here
 * saying so would make a case run into an Evidence row by another name.
 */
export type CaseRunView = {
  id: string;
  /** The case this ran, by id. */
  case: string;
  /** Where in the Job it ran. **`null` is after the Job** — a reviewer, or a merge. */
  coord: RunCoord | null;
  actor: CaseRunActor;
  /** Who, where the actor is a person or a contributor. */
  who?: string;
  purpose: CaseRunPurpose;
  tree: CaseRunTree;
  outcome: CaseRunOutcome;
  /** Why it did not run. Present on `not_run` and on nothing else. */
  not_run_reason?: string;
  /** How many frames it produced. */
  frames: number;
  ran_at: string;
};

/** What a person changed about which files a Job may write. */
export type ScopeRevisionView = {
  at: string;
  paths_added: string[];
  paths_removed: string[];
  /** The cases that fell out, by id. */
  cases_dropped: string[];
  /** The cases that came in, by id. */
  cases_added: string[];
  /** Who made the revision. */
  by: string;
  /** What it came to, in words. */
  outcome: string;
};

/**
 * The cases a Job knows about, from the specs its Drones named.
 *
 * **Every one is `added`** — a spec exists because a Drone wrote and submitted
 * it, so nothing here is owed-and-unwritten. `covers` is empty for all of them:
 * no `COVERS` file is read by anything yet, and inventing coverage would make a
 * case claim to guard files nobody said it guards.
 */
export function caseViewsOf(detail: JobDetail): CaseView[] {
  const again = detail.show_again;
  const specs = again?.specs ?? [];
  const shown = again?.shown ?? [];
  return specs.map((named) => {
    const view: CaseView = {
      id: caseIdOf(named),
      spec: named.spec,
      covers: [],
      has_spec: named.on_disk,
      tasks: [],
      groups: [],
      state: "added",
    };
    const last = lastRunOf(named, shown);
    if (last !== undefined) view.last_run = last;
    return view;
  });
}

/**
 * The runs a person made from the Job screen.
 *
 * Every press is `actor: person`, `purpose: show_again` — the review-time run
 * #1530 names as one of the three that post back — against the branch, and
 * `ran`, because a press that captured nothing keeps no set at all.
 */
export function caseRunsOf(detail: JobDetail): CaseRunView[] {
  return (detail.show_again?.shown ?? []).map(caseRunOf);
}

/**
 * The scope revision a Job's plan carries, where a person recorded the plan as
 * it stands.
 *
 * **At most one, and usually none.** `WorkPlan.recorded_by` says who recorded
 * the plan whole, and a person doing so is the nearest thing today's wire has
 * to a scope revision. The paths are empty because the wire keeps no before
 * and after of them.
 */
export function scopeRevisionsOf(detail: JobDetail): ScopeRevisionView[] {
  const plan = detail.work_plan;
  if (plan === undefined || plan.recorded_by.by !== "person") {
    return [];
  }
  return [
    {
      at: plan.recorded_at,
      paths_added: [],
      paths_removed: [],
      cases_dropped: [],
      cases_added: [],
      by: "person",
      outcome: "the plan was recorded whole",
    },
  ];
}

// A case has no id on the wire, so one is composed from the facts that identify
// the spec: the step that named it, the run of that step, and the path. Stable
// while the record is, and it changes when a later run names a different path —
// which is a different case.
function caseIdOf(named: NamedSpec): string {
  return `${named.step_id}#${named.attempt}:${named.spec}`;
}

function lastRunOf(
  named: NamedSpec,
  shown: readonly ShownSet[],
): CaseRunView | undefined {
  // `shown` is oldest first, so the last matching press is the latest one.
  for (let index = shown.length - 1; index >= 0; index -= 1) {
    const set = shown[index];
    if (set !== undefined && set.spec === named.spec) {
      return caseRunOf(set);
    }
  }
  return undefined;
}

function caseRunOf(set: ShownSet): CaseRunView {
  return {
    id: `press-${set.press}`,
    case: `${set.step_id}#${set.attempt}:${set.spec ?? ""}`,
    coord: { step: set.step_id, step_attempt: set.attempt },
    actor: "person",
    purpose: "show_again",
    tree: "branch",
    outcome: "ran",
    frames: set.frames.length,
    ran_at: set.pressed_at,
  };
}
