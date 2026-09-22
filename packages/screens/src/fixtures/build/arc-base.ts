// The Job the arc is twelve moments of, and the parts every arc builder shares.
//
// **One narrative, the way `base.ts` is one narrative.** That one is the Bug
// Job the roster of states is cut from; this one is the Feature Job the new
// boards were drawn against — #1162, on the real `feature` steps its own file
// declares, with four groups and eight tasks under `implement`.
//
// **Two halves, and the seam is the point.** Everything Fleet serves today is
// a `@armada/protocol` value and lands on `JobFixture`. Everything the new
// boards draw and the wire cannot carry is a `packages/screens/src/draft`
// value and lands on `ArcDraft` beside it. A board built on the second half
// renders against the real Fleet the moment `#1545` promotes a shape.

import type {
  Criterion,
  DeclaredCheck,
  Held,
  JobDetail,
  JobResources,
  JobSummary,
  ManifestSummary,
  PlanTask,
  ProposalInFlight,
  StepDetail,
  Watched,
  WorkPlan,
  WorkflowSummary,
} from "@armada/protocol";
import type {
  CaseRunView,
  CaseView,
  CriterionView,
  GroupView,
  JobMembersView,
  LandingRule,
  LedgerRow,
  PeerOverlapAnswer,
  ProposalView,
  PulseView,
  ScopeRevisionView,
  SketchAttachment,
  TaskView,
  WaveView,
} from "../../draft";
import type { Outstanding } from "../../outstanding";
import type { JobFixture } from "../fixture";
import { manifest, MANIFEST_ID } from "./base";

/** The Job every arc moment is a moment of. */
export const ARC_JOB_ID = "01M2D4YQK80011620DRONEST";
export const ARC_HANDLE = "3-show-what-s-running-in-the-drones-stat";
export const ARC_BRANCH = `armada/${ARC_HANDLE}`;
export const ARC_WORKTREE = `.armada/worktrees/${ARC_HANDLE}`;
export const ARC_TITLE = "Show what's running in the Drones stat";
/** The issue the words were frozen out of. */
export const ARC_ISSUE = "armada/1162";
export const ARC_CREATED_AT = "2026-09-22T09:02:00Z";
export const ARC_APPROVED_AT = "2026-09-22T09:14:00Z";
/** `now`, a few minutes past the latest instant any moment carries. */
export const ARC_NOW = Date.parse("2026-09-22T11:20:00Z");

/**
 * One moment of the arc: what Fleet serves, and what only the new boards draw.
 *
 * **`fixtures` is a list because a moment is not always one Job.** Dispatch has
 * no Job at all and draws the Board it would join; the landing moments are
 * several Jobs and the order between them.
 */
export type ArcMoment = {
  /** The builder's own name, which becomes `arc/<name>` in the picker. */
  name: string;
  /** What a person sees, as a sentence. */
  says: string;
  fixtures: JobFixture[];
  /** The Job the moment opens on. Absent where no Job exists yet. */
  opens?: string;
  /** A proposer call still out — `BridgeState.proposing`, which is on the wire. */
  proposing?: ProposalInFlight;
  /**
   * Every question this moment's Jobs are holding open, as main gathers them
   * across every repository (`apps/desktop/src/main/questions.ts`). Wire
   * shapes, so a card answers from what was asked — which is why they are
   * here and not on the draft.
   */
  questions?: Outstanding[];
  draft: ArcDraft;
};

/**
 * What the new boards draw and today's wire cannot carry.
 *
 * Every field is optional and every one is a draft type. A moment fills what
 * its board draws and leaves the rest absent, which is the honest answer —
 * absent here is "this moment says nothing about that", never zero.
 */
export type ArcDraft = {
  /** What a person typed, before a Job exists to hold it. */
  prompt?: string;
  sketch?: SketchAttachment;
  peers?: PeerOverlapAnswer;
  proposal?: ProposalView;
  landing?: LandingRule;
  criteria?: CriterionView[];
  /** The plan, as groups of tasks. The Plan and Implement boards' whole input. */
  groups?: GroupView[];
  /** The Record's rows, a Judge and a Check each on their own. */
  record?: LedgerRow[];
  cases?: CaseView[];
  runs?: CaseRunView[];
  scope_revisions?: ScopeRevisionView[];
  pulse?: PulseView;
  members?: JobMembersView;
  /** The wave this Job dispatched, and which of its Jobs waits on which. */
  wave?: WaveView;
};

/**
 * The Checks that run where this work writes, named with the globs
 * `armada.yml` selects them by.
 *
 * **The globs decide the count, not a drawing.** A group touching `crates/**`
 * runs four; one touching `packages/**` runs seven. No `run` is carried: what
 * a Check does is its Manifest's, and a command line in a fixture is text the
 * tooling has to read past.
 */
function declared(name: string, when: string[]): DeclaredCheck {
  return { kind: "manifest_check", name, when, expect_exit_code: 0 };
}

const RUST = ["crates/**", "xtask/**", "Cargo.toml", "Cargo.lock"];
const NODE = ["apps/**", "packages/**", "package.json", "pnpm-lock.yaml"];

/** What a group writing Rust runs at its boundary. */
export const RUST_CHECKS: DeclaredCheck[] = [
  declared("build", RUST),
  declared("test", [...RUST, "apps/**", "packages/**"]),
  declared("acceptance", ["crates/**", "Cargo.toml", "Cargo.lock"]),
  declared("format", ["**/*.rs", "Cargo.toml", "rustfmt.toml"]),
];

/** What a group writing Bridge runs at its boundary. */
export const BRIDGE_CHECKS: DeclaredCheck[] = [
  declared("test", [...RUST, "apps/**", "packages/**"]),
  declared("typecheck", NODE),
  declared("bridge_build", NODE),
  declared("storybook", ["packages/**", "package.json", "pnpm-lock.yaml"]),
  declared("desktop_test", NODE),
  declared("screens_test", ["packages/**", "package.json", "pnpm-lock.yaml"]),
  declared("components_test", ["packages/**", "package.json", "pnpm-lock.yaml"]),
];

/** The Check names alone, which is what a group carries. */
export const checkNames = (checks: DeclaredCheck[]): string[] =>
  checks.map((check) => check.name ?? check.kind);

/** The four steps `feature.json` declares, with the gates it declares them at. */
export function featureWorkflow(): WorkflowSummary {
  return {
    id: "feature",
    name: "feature",
    version: 1,
    manifest_id: MANIFEST_ID,
    steps: [
      {
        step_id: "plan",
        label: "Plan the change",
        checks: [],
        judge_checks: [{ criteria: 2, gaming_check: false }],
        advance_gate: "auto_if_judge_passes",
        delivers: false,
      },
      {
        step_id: "implement",
        label: "Implement",
        checks: [...RUST_CHECKS, ...BRIDGE_CHECKS],
        judge_checks: [{ criteria: 4, gaming_check: false }],
        advance_gate: "auto_if_judge_passes",
        delivers: false,
      },
      {
        step_id: "tests",
        label: "Write tests",
        checks: BRIDGE_CHECKS,
        judge_checks: [{ criteria: 1, gaming_check: true }],
        advance_gate: "auto_if_judge_passes",
        delivers: false,
      },
      {
        step_id: "handoff",
        label: "Review the change",
        checks: [],
        judge_checks: [],
        advance_gate: "human_always",
        delivers: true,
      },
    ],
  };
}

/** The two things the Job is held to, out of the issue it was cut from. */
export const ARC_CRITERIA: Criterion[] = [
  {
    criterion_id: "a1",
    text: "The rail's Drones stat reads one running beside the machine's most",
    source: "check",
  },
  {
    criterion_id: "a2",
    text: "Pressing the stat lists the Drone's Job and step, and any Check or Judge call out",
    source: "judge",
  },
];

/** The same two, with where the words came from — which the wire does not say. */
export function arcCriterionViews(movedAt?: string): CriterionView[] {
  const first: CriterionView = {
    criterion_id: "a1",
    text: ARC_CRITERIA[0]!.text,
    verified_by: "check",
    origin: { origin: "issue", ref: ARC_ISSUE },
  };
  if (movedAt !== undefined) first.origin_moved_at = movedAt;
  return [
    first,
    {
      criterion_id: "a2",
      text: ARC_CRITERIA[1]!.text,
      verified_by: "judge",
      origin: { origin: "issue", ref: ARC_ISSUE },
    },
  ];
}

/** The Job row. `status` is the one field every moment sets for itself. */
export function arcJob(status: string, over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: ARC_JOB_ID,
    handle: ARC_HANDLE,
    title: ARC_TITLE,
    status,
    workflow_id: "feature",
    owner_manifest_id: MANIFEST_ID,
    origin: "manual",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: ARC_CREATED_AT,
    branch: ARC_BRANCH,
    ...over,
  };
}

/** A step nothing has entered. */
export function arcStep(
  id: string,
  label: string,
  ordinal: number,
  checks: DeclaredCheck[] = [],
): StepDetail {
  return {
    step_id: id,
    label,
    ordinal,
    state: "not_started",
    checks,
    check_runs: [],
    judge_checks: [],
    judged: [],
    flagged: [],
    overridden: false,
    attempts: [],
    verdicts: [],
    entered_at: ARC_CREATED_AT,
    updated_at: ARC_CREATED_AT,
  };
}

/** The four steps, none of them entered. */
export function arcSteps(): StepDetail[] {
  return [
    arcStep("plan", "Plan the change", 1),
    arcStep("implement", "Implement", 2, [...RUST_CHECKS, ...BRIDGE_CHECKS]),
    arcStep("tests", "Write tests", 3, BRIDGE_CHECKS),
    arcStep("handoff", "Review the change", 4),
  ];
}

/** A step that ran once and advanced. */
export function arcAdvanced(step: StepDetail, from: string, to: string): StepDetail {
  return {
    ...step,
    state: "advanced",
    check_runs: (step.checks ?? []).map((check) => ({
      attempt: 1,
      name: check.name ?? check.kind,
      outcome: "passed",
    })),
    attempts: [{ attempt: 1, outcome: "advanced", started_at: from, ended_at: to }],
    verdicts: [{ attempt: 1, named: "passed" }],
    entered_at: from,
    updated_at: to,
  };
}

/** A step a Drone is inside. */
export function arcRunning(step: StepDetail, from: string, at: string): StepDetail {
  return {
    ...step,
    state: "running",
    attempts: [{ attempt: 1, outcome: "running", started_at: from }],
    entered_at: from,
    updated_at: at,
  };
}

/** The whole Job, as `GET /jobs/:job_id` answers it. */
export function arcDetail(
  jobRow: JobSummary,
  steps: StepDetail[],
  over: Partial<JobDetail> = {},
): JobDetail {
  return {
    job: jobRow,
    created_at: ARC_CREATED_AT,
    branch: ARC_BRANCH,
    steps,
    acceptance_criteria: ARC_CRITERIA,
    facts:
      "The stat reads its two numbers off get_capacity, and nothing on it leads to the one " +
      "Drone that is running or to anything else the machine has out.",
    write_targets: ["crates/api/src/", "crates/fleet/src/", "packages/screens/src/"],
    dependencies: [],
    when_blocked: "refuse_and_hold",
    ...over,
  };
}

export function arcWatched(whole: JobDetail): Watched {
  return { state: "read", jobId: ARC_JOB_ID, detail: whole };
}

export function arcManifests(): ManifestSummary[] {
  return [manifest()];
}

/** What the Job holds on the machine, with a process per Drone that is up. */
export function arcResources(
  held: Held,
  processes: JobResources["processes"] = [],
): JobResources {
  return {
    job_id: ARC_JOB_ID,
    read_at: "2026-09-22T11:18:00.000Z",
    held,
    processes,
    worktree: { path: ARC_WORKTREE, branch: ARC_BRANCH, bytes: 1_020_054_016 },
  };
}

/** The plan as the wire carries it — the part a Fleet today could send. */
export function arcWorkPlan(tasks: TaskView[]): WorkPlan {
  return {
    approach:
      "One read of everything running, then the stat's words, then the panel, then what " +
      "holds the next Drone back.",
    recorded_by: { by: "step", step_id: "plan", attempt: 1 },
    recorded_at: "2026-09-22T09:21:00Z",
    tasks: tasks.map(planTaskOf),
  };
}

/** One task, narrowed to the fields the wire has a home for. */
function planTaskOf(task: TaskView): PlanTask {
  const row: PlanTask = {
    id: task.id,
    title: task.title,
    scope: task.scope,
    state: task.state === "failed" ? "working" : task.state,
  };
  if (task.note !== undefined) row.note = task.note;
  if (task.expects !== undefined) row.expects = task.expects;
  if (task.shown !== undefined) row.shown = task.shown;
  if (task.reason !== undefined) row.reason = task.reason;
  return row;
}

/** Every task of every group, in plan order. */
export const tasksOf = (groups: GroupView[]): TaskView[] =>
  groups.flatMap((group) => group.tasks);
