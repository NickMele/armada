// The one Job every fixture is a moment of, and the parts every builder shares.
//
// **One narrative, not six unrelated Jobs.** The Bug workflow Job Storybook
// already knows — "Split the settings reducer so the selectors can be tested
// alone", branch `fix/settings-split-selectors` — from
// `InsideAJobOneArrangementAtEveryState/fixtures.tsx`. Each state file in this
// directory freezes it at a different moment; sharing the id, the workflow and
// the step labels here is what keeps a reader able to recognise the same Job
// across the roster rather than reading six strangers.
//
// **Shaped the way Fleet writes it, not the way it is convenient to type.**
// `crates/fleet/src/briefing.rs` is what `brief()` below reproduces the block
// structure of, and `crates/core-model/src/job/step.rs` is where
// `StepVerdict::as_wire` fixes `verdict()`'s three spellings — `passed`,
// `failed`, `not_reached` — a fixture that invented `"pass"` would compile and
// never match what the gate actually sends.

import type {
  ChangedFile,
  CheckRun,
  Criterion,
  DeclaredCheck,
  Diff,
  Evidence,
  Footprint,
  Held,
  Holds,
  JobDetail,
  JobLog,
  JobResources,
  JobSpend,
  JobSummary,
  Journalled,
  ManifestSummary,
  Noted,
  Observed,
  Remarks,
  StepAttempt,
  StepDetail,
  Stuck,
  Submitted,
  Turn,
  Turns,
  Verdict,
  Watched,
  WorkflowSummary,
} from "@armada/protocol";
import type { FoldedReads } from "../../JobDetail";

/** The Job every fixture is a moment of. Reused across states on purpose. */
export const JOB_ID = "01M2C1TJ8G0016YK5SPLITSEL";
export const JOB_HANDLE = "77-split-the-settings-reducer";
export const DRONE_ID = "01M1HHJ6XB001BZJZ4BE2SPLIT";
export const WORKFLOW_ID = "bug";
export const MANIFEST_ID = "01M1CNPKTV0018H2M1CXDNBK06";
export const BRANCH = "fix/settings-split-selectors";
export const WORKTREE = `.armada/worktrees/${JOB_HANDLE}`;
export const TITLE = "Split the settings reducer so the selectors can be tested alone";
export const CREATED_AT = "2026-09-10T14:11:00Z";

/** The step ids, in workflow order — the one list every builder walks. */
export const STEP_IDS = ["repro", "root_cause", "fix", "regression_verify", "consumers", "land"];

/** The two acceptance criteria Regression check's Judge answers against. */
export const CRITERIA: Criterion[] = [
  {
    criterion_id: "c1",
    text: "The selectors module has no import of the store",
    source: "workflow",
  },
  { criterion_id: "c2", text: "Every existing settings test still passes", source: "workflow" },
];

/** The Manifest Check both Checks-gated steps declare. */
export const BUILD_CHECK: DeclaredCheck = {
  kind: "manifest_check",
  name: "cargo_build",
  run: "cargo build --workspace --locked",
  expect_exit_code: 0,
};

/** The regression suite Regression check declares, and Fix does not. */
export const NEXTEST_CHECK: DeclaredCheck = {
  kind: "manifest_check",
  name: "cargo_nextest",
  run: "cargo nextest run --workspace",
  expect_exit_code: 0,
};

/** `WorkflowSummary` for `bug`, six steps, matching `STEP_IDS` in order. */
export function workflow(): WorkflowSummary {
  return {
    id: WORKFLOW_ID,
    name: "Bug",
    version: 3,
    manifest_id: MANIFEST_ID,
    steps: [
      { step_id: "repro", label: "Reproduction", checks: [], judge_checks: [], advance_gate: "auto" },
      {
        step_id: "root_cause",
        label: "Root cause",
        checks: [],
        judge_checks: [],
        advance_gate: "auto",
      },
      {
        step_id: "fix",
        label: "Fix",
        checks: [BUILD_CHECK],
        judge_checks: [],
        advance_gate: "auto_if_judge_passes",
      },
      {
        step_id: "regression_verify",
        label: "Regression check",
        checks: [NEXTEST_CHECK],
        judge_checks: [{ criteria: 2, gaming_check: true }],
        advance_gate: "human_always",
      },
      {
        step_id: "consumers",
        label: "Check the consumers still compile",
        checks: [BUILD_CHECK],
        judge_checks: [],
        advance_gate: "auto",
      },
      { step_id: "land", label: "Land", checks: [], judge_checks: [], advance_gate: "auto" },
    ],
  };
}

export function manifest(): ManifestSummary {
  return {
    id: MANIFEST_ID,
    repository: "armada",
    path: "armada.yml",
    version: 3,
    checks: ["cargo_build", "cargo_nextest"],
  };
}

/**
 * The Job row. Every state file calls this with the status it is a moment of
 * — the one field a fixture always overrides — and whatever else the wire
 * would carry alongside it there.
 */
export function job(status: string, over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: JOB_ID,
    handle: JOB_HANDLE,
    title: TITLE,
    status,
    workflow_id: WORKFLOW_ID,
    owner_manifest_id: MANIFEST_ID,
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: CREATED_AT,
    branch: BRANCH,
    assigned_drone: DRONE_ID,
    ...over,
  };
}

/**
 * The two steps behind every fixture: Reproduction and Root cause both
 * advanced before any of the rostered moments begin. `root_cause` keeps a
 * deliverable, the way a real root-cause step does — its evidence type is
 * `document`, per `workflowdef-fields.toml`'s roster.
 */
export function reproStep(): StepDetail {
  return {
    step_id: "repro",
    label: "Reproduction",
    ordinal: 1,
    state: "advanced",
    checks: [],
    check_runs: [],
    judge_checks: [],
    judged: [],
    flagged: [],
    overridden: false,
    attempts: [
      { attempt: 1, outcome: "advanced", started_at: "2026-09-10T14:11:15Z", ended_at: "2026-09-10T14:12:27Z" },
    ],
    verdicts: [{ attempt: 1, named: "passed" }],
    entered_at: "2026-09-10T14:11:15Z",
    updated_at: "2026-09-10T14:12:27Z",
  };
}

export function rootCauseStep(): StepDetail {
  return {
    step_id: "root_cause",
    label: "Root cause",
    ordinal: 2,
    state: "advanced",
    checks: [],
    check_runs: [],
    judge_checks: [],
    judged: [],
    flagged: [],
    overridden: false,
    deliverables: [{ attempt: 1, path: `.armada/deliverables/${JOB_HANDLE}/root_cause.1.plan.md` }],
    attempts: [
      { attempt: 1, outcome: "advanced", started_at: "2026-09-10T14:12:27Z", ended_at: "2026-09-10T14:16:07Z" },
    ],
    verdicts: [{ attempt: 1, named: "passed" }],
    entered_at: "2026-09-10T14:12:27Z",
    updated_at: "2026-09-10T14:16:07Z",
  };
}

/** The two steps ahead of every rostered moment: neither has been entered. */
export function consumersStep(): StepDetail {
  return freshStep("consumers", "Check the consumers still compile", 5, [BUILD_CHECK]);
}

export function landStep(): StepDetail {
  return freshStep("land", "Land", 6);
}

/**
 * A step frozen `advanced`, one clean attempt — the light fixtures' shorthand
 * for "this step is behind us and nothing about it is the reason you are
 * here". Full-depth fixtures build their own, because what a lead step looked
 * like on the way through is usually the point.
 */
export function advancedStep(id: string, label: string, ordinal: number, checks: DeclaredCheck[] = []): StepDetail {
  return {
    ...freshStep(id, label, ordinal, checks),
    state: "advanced",
    check_runs: checks.map((check) => ({ attempt: 1, name: check.name ?? check.kind, outcome: "passed" })),
    attempts: [{ attempt: 1, outcome: "advanced", started_at: CREATED_AT, ended_at: CREATED_AT }],
    verdicts: [{ attempt: 1, named: "passed" }],
  };
}

/** A bare, not-yet-entered step — `fix` and `regression_verify` before either runs. */
export function freshStep(
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
    entered_at: CREATED_AT,
    updated_at: CREATED_AT,
  };
}

/** One run of `cargo_build`, whichever way it went. */
export function buildRun(over: Partial<CheckRun> = {}): CheckRun {
  return { attempt: 1, name: "cargo_build", outcome: "passed", ...over };
}

/** One run of `cargo_nextest`, whichever way it went. */
export function nextestRun(over: Partial<CheckRun> = {}): CheckRun {
  return { attempt: 1, name: "cargo_nextest", outcome: "passed", ...over };
}

export function verdict(attempt: number, named: string, trigger?: string): Verdict {
  return trigger === undefined ? { attempt, named } : { attempt, named, trigger };
}

export function attempt(
  entry: Partial<StepAttempt> & { attempt: number; outcome: string },
): StepAttempt {
  return { started_at: CREATED_AT, ...entry };
}

/**
 * The whole Job, as `GET /jobs/:job_id` answers. `jobRow` is the same
 * `JobSummary` the fixture's own `job` field carries — `detail.job` is "the
 * board row, unchanged" — so the two never disagree about status or reason.
 */
export function detail(jobRow: JobSummary, steps: StepDetail[], over: Partial<JobDetail> = {}): JobDetail {
  return {
    job: jobRow,
    created_at: CREATED_AT,
    branch: BRANCH,
    steps,
    acceptance_criteria: CRITERIA,
    facts:
      "The selectors cannot be tested without constructing the whole store, which makes every " +
      "settings test an integration test.",
    write_targets: ["packages/settings/src/"],
    dependencies: [],
    spend: spend(),
    ...over,
  };
}

export function watchedRead(whole: JobDetail): Watched {
  return { state: "read", jobId: JOB_ID, detail: whole };
}

/**
 * A brief exactly as `briefing.rs` writes one: `JOB BRIEF`, the title, what
 * done means, `WHERE YOU ARE`, the parts rail, then `STEP: <label>` — each a
 * block of its own, separated by one blank line. `headings` names which lines
 * of the joined text are block headings, zero-based, the way Fleet stamps it.
 */
export function brief(
  stepLabel: string,
  doneWhen: string,
  onPart: number,
): { text: string; headings: number[] } {
  const lines = [
    "JOB BRIEF",
    "",
    TITLE,
    "",
    "This is done when:",
    `  - ${doneWhen}`,
    "",
    "WHERE YOU ARE",
    "",
    `This task runs in ${PARTS.length} parts. You are on part ${onPart}.`,
    "",
    ...PARTS.map((part, at) => `  ${at + 1}. ${part}${at + 1 === onPart ? " — you are here" : ""}`),
    "",
    `STEP: ${stepLabel}`,
    "",
    "What you claim should be what the work now does, not that you finished.",
  ];
  const headings: number[] = [];
  lines.forEach((line, at) => {
    if (line === "JOB BRIEF" || line === "WHERE YOU ARE" || line.startsWith("STEP: ")) {
      headings.push(at);
    }
  });
  return { text: lines.join("\n"), headings };
}

const PARTS = [
  "Reproduce the bug",
  "Find the root cause",
  "Fix it",
  "Verify nothing else broke",
  "Check the consumers still compile",
  "Land the change",
];

// One counter for every Turn's `seq` across a whole fixture file's build, so
// two turns from the same import never collide. Reset per module load, which
// is once per test run — fine, because nothing here compares `seq` across
// fixtures.
let seq = 0;
function nextSeq(): number {
  seq += 1;
  return seq;
}

/** The opening turn of a step, carrying the whole brief. */
export function instructed(step: string, ts: string, onPart: number, doneWhen: string, stepLabel: string): Turn {
  const built = brief(stepLabel, doneWhen, onPart);
  return {
    ts,
    seq: nextSeq(),
    step,
    by: "armada",
    saw: { event: "instructed", occasion: "opening", text: built.text, headings: built.headings },
  };
}

export function said(step: string, ts: string, text: string): Turn {
  return { ts, seq: nextSeq(), step, by: "drone", saw: { event: "said", text } };
}

export function called(step: string, ts: string, callId: string, tool: string, detail: string): Turn {
  return {
    ts,
    seq: nextSeq(),
    step,
    by: "drone",
    saw: { event: "called", tool, call: callId, detail, truncated: false },
  };
}

export function answered(step: string, ts: string, callId: string, failed = false): Turn {
  return { ts, seq: nextSeq(), step, by: "drone", saw: { event: "answered", call: callId, failed } };
}

export function checked(step: string, ts: string, run: CheckRun): Turn {
  return { ts, seq: nextSeq(), step, by: "fleet", saw: { event: "checked", run } };
}

export function producedTurn(step: string, ts: string, files: ChangedFile[]): Turn {
  return { ts, seq: nextSeq(), step, by: "fleet", saw: { event: "produced", files } };
}

export function droneEnded(step: string, ts: string, turns: number, costMicros: number): Turn {
  return {
    ts,
    seq: nextSeq(),
    step,
    by: "drone",
    saw: { event: "ended", turns, cost_micros: costMicros, refusals: 0 },
  };
}

export function note(at: string, msg: string, over: Partial<Noted> = {}): Noted {
  return { at, by: "fleet", level: "info", msg, seq: nextSeq(), ...over };
}

/** `Observed` at `watching`, holding whatever turns the fixture built. */
export function observedWatching(rows: Turn[], live = false): Observed {
  const turns: Turns = { live, skipped: 0, missed: 0, rows };
  return { state: "watching", jobId: JOB_ID, turns };
}

/** `Observed` at `ended` — the rows are kept, because a closed transcript is still a record. */
export function observedEnded(rows: Turn[], because: string): Observed {
  const turns: Turns = { live: false, skipped: 0, missed: 0, rows };
  return { state: "ended", jobId: JOB_ID, turns, because };
}

export const NO_OBSERVED: Observed = { state: "none" };

/** `Journalled` at `watching`, holding whatever notes the fixture built. */
export function journalledWatching(notes: Noted[]): Journalled {
  const log: JobLog = { skipped: 0, notes };
  return { state: "watching", jobId: JOB_ID, log };
}

export const NO_JOURNALLED: Journalled = { state: "none" };

/** Spend, present on every fixture — a Job that has spent nothing still carries it. */
export function spend(over: Partial<JobSpend> = {}): JobSpend {
  return {
    cost_micros: 1_800_000,
    cost_cap_micros: 20_000_000,
    turns: 18,
    turn_cap: 40,
    ran_ms: 663_000,
    drones: 1,
    ...over,
  };
}

export function resources(held: Held, over: Partial<JobResources> = {}): JobResources {
  return {
    job_id: JOB_ID,
    read_at: "2026-09-10T14:31:00.000Z",
    held,
    processes: [],
    worktree: { path: WORKTREE, branch: BRANCH, bytes: 1_288_490_188 },
    ...over,
  };
}

export function holdsRead(reading: JobResources): Holds {
  return { state: "read", jobId: JOB_ID, resources: reading };
}

export const NO_FOOTPRINT: Footprint = { state: "none" };
export const NO_EVIDENCE: Evidence = { state: "none" };
export const NO_DIFF: Diff = { state: "none" };
export const NO_REMARKS: Remarks = { state: "none" };

export function evidenceRead(steps: Submitted[]): Evidence {
  return { state: "read", jobId: JOB_ID, steps };
}

export function diffRead(files: ChangedFile[], patch?: string): Diff {
  return {
    state: "read",
    jobId: JOB_ID,
    work: { files, measured_from: "main", measured_whole: true, plan_declared: true, patch },
  };
}

/** What a person left on this Job's escalation — `Stuck`, read off `stopped_by`. */
export function stuck(over: Partial<Stuck> & { recourse: string[]; worktree_on_disk: boolean; drone_unheard: boolean }): Stuck {
  return { refused: [], refusals: 0, ...over };
}

/** The default folded reads — every read `JobDetail.recorded` needs, empty. */
export function foldedReads(over: Partial<FoldedReads> = {}): FoldedReads {
  return { footprint: NO_FOOTPRINT, evidence: NO_EVIDENCE, diff: NO_DIFF, remarks: NO_REMARKS, ...over };
}

/** `now`, fixed a few minutes after the narrative's latest timestamp. */
export const NOW = Date.parse("2026-09-10T14:31:00Z");
