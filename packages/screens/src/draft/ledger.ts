// One line of the Record: what happened, where, and what it came to. Draft,
// for `crates/ipc/src/history.rs`.
//
// Source of truth today: `JobHistory.moves` — `Recorded` with its `seq`,
// `status`, `actor`, `at` and one of three `Movement` shapes.
//
// **`kind` stays an opaque string** (#1532). A new kind of row is then a minor
// bump rather than a major one: nothing here branches on the value, a surface
// looks it up and renders the spelling itself where it finds no word for it.
// That is the same argument `FleetCapacity.held_by` is made on in
// `docs/practices/protocol.md`.

import type {
  EvidenceSubmitted,
  JobDetail,
  JobFilesChanged,
  JobHistory,
  Recorded,
  StepAttempt,
  StepDetail,
  Submitted,
} from "@armada/protocol";

import { caseRunsOf } from "./cases";
import { coordOfStep, type RunCoord } from "./coord";
import { taskViewsOf } from "./task";

/**
 * Who a row is about. The wire's `Actor`, plus the three the new shape needs.
 *
 * `contributor` is somebody who is not you, on their own machine. **Nothing
 * derives one today** — it needs the store between Armada instances that
 * `#1530` files separately — so the value exists for the shape's sake and the
 * derivation below never mints it.
 */
export type LedgerActor =
  | "person"
  | "contributor"
  | "fleet"
  | "drone"
  | "judge"
  | "check";

/**
 * One row of the Record.
 *
 * **`coord` and `actor` replace `step` and `task?`** (#1532). A row inside a
 * group could not be placed by a step id alone, and a row about no step at all
 * — the Job's own machine moving — needs to say so rather than carry a blank.
 */
export type LedgerRow = {
  at: string;
  /**
   * Where it happened. **`null` is a fact about the Job and not about any
   * step** — its own machine moving, a person approving it. Not a gap.
   */
  coord: RunCoord | null;
  actor: LedgerActor;
  /** An opaque string. A surface renders the spelling where it has no word. */
  kind: string;
  /** What happened, in words. */
  what: string;
  /** What it came to, in words. Empty where the row states no outcome. */
  outcome: string;
  /**
   * What a reader asks for the next page with.
   *
   * **The log's own `seq`, which is monotonic and never reused** — and never
   * `at`, which is injected rather than read from a clock, so two rows inside
   * one millisecond carry the same instant.
   */
  cursor: number;
};

/**
 * The seven the Record's filters divide rows into, beside All.
 *
 * **They partition the ledger**, which is the whole point: `#1537`'s own
 * finding is a board whose All said 34 while its filters summed to 35, so a
 * row belongs to exactly one of these and All is their sum. `familyOf` is the
 * one place a row is placed.
 */
export type LedgerFamily =
  | "evidence"
  | "files"
  | "checks"
  | "judges"
  | "drones"
  | "tasks"
  | "tests";

/** The seven, in the order the filters draw them. */
export const LEDGER_FAMILIES: readonly LedgerFamily[] = [
  "evidence",
  "files",
  "checks",
  "judges",
  "drones",
  "tasks",
  "tests",
];

// The spellings each family answers to. `kind` stays opaque on the wire (the
// header says why), so this is a lookup a surface does and never a branch the
// type forces — a kind nothing here names still draws, under `tasks`.
const FAMILY_OF: Readonly<Record<string, LedgerFamily>> = {
  evidence_submitted: "evidence",
  handed_in: "evidence",
  deliverable_kept: "evidence",
  file_written: "files",
  task_files: "files",
  checked: "checks",
  judged: "judges",
  flagged: "judges",
  drone_spawned: "drones",
  drone_exited: "drones",
  case_run: "tests",
  cases_rerun: "tests",
  shown_again: "tests",
  frames_kept: "tests",
};

/**
 * Which filter a row answers to.
 *
 * **`tasks` is the residual, and that is the one thing here the issue did not
 * decide.** Six of the seven name a producer — an evidence claim, a file, a
 * Check, a Judge, a Drone, a case run — and the Job's own machine moving
 * produces none of them. The seven have to sum to All, so a Job-machine row
 * cannot sit outside them: it is filed under the Job's own work, which is
 * `tasks`. A kind this module has never heard of lands there too, rather than
 * vanishing from every filter while still counting in All.
 */
export function familyOf(kind: string): LedgerFamily {
  return FAMILY_OF[kind] ?? "tasks";
}

/** How many rows each filter holds. The seven sum to the length of `rows`. */
export function countsOf(rows: readonly LedgerRow[]): Record<LedgerFamily, number> {
  const counts = {
    evidence: 0,
    files: 0,
    checks: 0,
    judges: 0,
    drones: 0,
    tasks: 0,
    tests: 0,
  };
  for (const row of rows) counts[familyOf(row.kind)] += 1;
  return counts;
}

/** Every row of a Job's Record, oldest first. */
export function ledgerRowsOf(history: JobHistory): LedgerRow[] {
  return history.moves.map(ledgerRowOf);
}

/** One recorded move, as a Record row. */
export function ledgerRowOf(move: Recorded): LedgerRow {
  return {
    at: move.at,
    coord: coordOf(move),
    actor: actorOf(move.actor),
    kind: kindOf(move),
    what: whatOf(move),
    outcome: outcomeOf(move),
    cursor: move.seq,
  };
}

// The Job's own machine moving names no step, so it is `null`. A step move and
// a Drone arriving both name one, and neither names a group or a task — the
// wire has neither, so the coordinate stops at the step.
function coordOf(move: Recorded): RunCoord | null {
  if (move.moved.kind === "status") {
    return null;
  }
  return { step: move.moved.step_id, step_attempt: 1 };
}

// `movement_kind` is the registry's own vocabulary for the three shapes. A
// status move is qualified by where it went, and a Drone move by its
// `presence`, so one Record can hold `drone_spawned` and `drone_exited` as
// different rows rather than as one row a reader has to open.
function kindOf(move: Recorded): string {
  switch (move.moved.kind) {
    case "status":
      return `status_${move.moved.to}`;
    case "step":
      return "step";
    case "drone":
      return move.moved.presence;
  }
}

function whatOf(move: Recorded): string {
  switch (move.moved.kind) {
    case "status":
      return `${move.status} to ${move.moved.to}`;
    case "step":
      return `${move.moved.step_id}: ${move.moved.from} to ${move.moved.to}`;
    case "drone":
      return `${move.moved.drone_id} on ${move.moved.step_id}`;
  }
}

// The reason a row carries, where it carries one. A status move's `reason` is
// absent on the eight destinations that store none, and a step move's `why` is
// present on the one move that stops it — so an empty outcome is the ordinary
// case rather than a value that failed to load.
function outcomeOf(move: Recorded): string {
  switch (move.moved.kind) {
    case "status":
      return move.moved.reason?.named ?? "";
    case "step":
      return move.moved.why ?? "";
    case "drone":
      return "";
  }
}

// The wire's `Actor` is `human`, `fleet` or `drone`. `judge` and `check` are
// the draft's own — a Judge call and a Check run are what the new Record draws
// most of, and today they arrive as Fleet acting. So neither can be derived,
// and a row that would be one of them reads as `fleet` rather than as a guess.
function actorOf(actor: string): LedgerActor {
  switch (actor) {
    case "drone":
      return "drone";
    case "human":
      return "person";
    default:
      return "fleet";
  }
}

// ------------------------------------------------- the whole Record, composed
//
// **`GET /jobs/:job_id/events` is three kinds of row and the Record draws
// seven.** A Check run, a Judge verdict, a file written, an evidence claim, a
// task finishing and a case run are none of them a `Movement`, and every one of
// them is already on a read Bridge takes to draw a Job. So the ledger is
// composed here from those reads, and the server-side ledger read replaces this
// wholesale when the backend lands (`#1537`).
//
// **Where the history was read, it owns the moves it carries** — the Job's own
// machine, the steps and the Drones — and the detail supplies the rest. Where
// it was not, those rows are derived from the detail instead, so a Job opened
// before its history answers still draws a Record rather than a blank.

/** What the Record is composed from. Every field but `detail` is optional. */
export type LedgerReads = {
  /** `GET /jobs/:job_id`, for the Job on screen. */
  detail: JobDetail;
  /** `GET /jobs/:job_id/events`, where it was read. */
  history?: readonly Recorded[];
  /** `GET /jobs/:job_id/evidence`, where it was read. */
  evidence?: readonly Submitted[];
  /** The last `job.files_changed` heard for this Job. */
  footprint?: JobFilesChanged;
  /** The last `evidence.submitted` heard for this Job. */
  handed?: EvidenceSubmitted;
};

/**
 * One Job's Record, **newest first**.
 *
 * `at` orders it and `cursor` breaks a tie, which is the inverse of the wire's
 * rule — the wire's `seq` exists for every row it carries, and most rows here
 * are derived from a read that has no `seq` at all.
 */
export function ledgerOf(reads: LedgerReads): LedgerRow[] {
  const { detail } = reads;
  const moves = reads.history ?? [];
  const rows: LedgerRow[] = moves.map(ledgerRowOf);
  const mint = minting(moves);

  if (moves.length === 0) rows.push(...jobRowsOf(detail, mint));
  for (const step of detail.steps) {
    rows.push(...checkRowsOf(step, mint));
    rows.push(...judgeRowsOf(detail, step, mint));
    rows.push(...evidenceKeptOf(step, mint));
    if (moves.length === 0) rows.push(...droneRowsOf(step, mint));
  }
  rows.push(...planRowsOf(detail, mint));
  rows.push(...taskRowsOf(detail, mint));
  rows.push(...fileRowsOf(detail, reads.footprint, mint));
  rows.push(...handedRowsOf(detail, reads.evidence, reads.handed, mint));
  rows.push(...testRowsOf(detail, mint));

  return rows.sort(newestFirst);
}

// A cursor for a row the log never carried. It starts past every `seq` the
// history holds, so a derived row can never collide with a recorded one, and it
// climbs in the order the rows are built — which is what settles two rows that
// share an instant.
function minting(moves: readonly Recorded[]): () => number {
  let next = moves.reduce((highest, move) => Math.max(highest, move.seq), 0) + 1;
  return () => next++;
}

function newestFirst(left: LedgerRow, right: LedgerRow): number {
  if (left.at === right.at) return right.cursor - left.cursor;
  return left.at < right.at ? 1 : -1;
}

// ---------------------------------------------------------------- the Job
//
// **Only where no history was read.** Creation is not a transition and the log
// carries no row for it (`JobHistory.moves` says so), so it is derived either
// way — but the approval and the end are status moves the log does carry, and
// deriving them beside the log's own would be the same fact twice.

function jobRowsOf(detail: JobDetail, mint: () => number): LedgerRow[] {
  const job = detail.job;
  const rows: LedgerRow[] = [
    {
      at: detail.created_at,
      coord: null,
      actor: job.origin === "manual" ? "person" : "fleet",
      kind: "created",
      what: "this Job was created",
      outcome: "",
      cursor: mint(),
    },
  ];
  // `started_at` is the first arrival at `running` and nothing else
  // (`JobSummary`), so a Job still at its gate has not reached it and draws no
  // such row rather than one dated from a field that means something else.
  if (job.started_at !== undefined && job.status !== "awaiting_approval" && job.status !== "queued") {
    rows.push({
      at: job.started_at,
      coord: null,
      actor: "fleet",
      kind: "started",
      what: "the Job's first Drone started",
      outcome: "",
      cursor: mint(),
    });
  }
  if (job.ended_at !== undefined) {
    rows.push({
      at: job.ended_at,
      coord: null,
      actor: "fleet",
      kind: `status_${job.status}`,
      what: "the Job ended",
      outcome: job.landed === undefined ? "" : `the pull request ${job.landed}`,
      cursor: mint(),
    });
  }
  return rows;
}

// -------------------------------------------------------------- the gates

/**
 * **A Check run is its own row, and `check` is who ran it.** Today's wire says
 * Fleet caused everything a gate does, so nothing could derive this actor from
 * an `Actor` field — it is read off which list the row came out of instead,
 * which is the only thing that separates a Check from a Judge on this seam.
 */
function checkRowsOf(step: StepDetail, mint: () => number): LedgerRow[] {
  return step.check_runs.map((run) => ({
    at: atOf(step, run.attempt),
    coord: { step: step.step_id, step_attempt: run.attempt },
    actor: "check" as const,
    kind: "checked",
    what: run.name,
    outcome: run.produced === undefined ? run.outcome : `${run.outcome} — ${run.produced}`,
    cursor: mint(),
  }));
}

/** A criterion the Judge answered, and a pattern it flagged. `judge` ran both. */
function judgeRowsOf(detail: JobDetail, step: StepDetail, mint: () => number): LedgerRow[] {
  const rows: LedgerRow[] = step.judged.map((answer) => ({
    at: atOf(step, answer.attempt),
    coord: { step: step.step_id, step_attempt: answer.attempt },
    actor: "judge" as const,
    kind: "judged",
    what:
      detail.acceptance_criteria.find((one) => one.criterion_id === answer.criterion_id)?.text ??
      answer.criterion_id,
    outcome: answer.produced === undefined ? said(answer.verdict) : `${said(answer.verdict)} — ${answer.produced}`,
    cursor: mint(),
  }));
  for (const flag of step.flagged) {
    rows.push({
      at: atOf(step, flag.attempt),
      coord: { step: step.step_id, step_attempt: flag.attempt },
      actor: "judge",
      kind: "flagged",
      what: flag.pattern,
      outcome: flag.cited,
      cursor: mint(),
    });
  }
  return rows;
}

// `criterion_verdict_judge` is `met` or `not_met`, and the underscore is the
// wire's spelling rather than a word. The generated vocabulary has no entry for
// it, so the one place it becomes English is here.
function said(verdict: string): string {
  return verdict === "not_met" ? "not met" : verdict;
}

// -------------------------------------------------------------- the Drones
//
// **One row per Drone per task, never one Drone for a whole step** (`#1530`).
// Today's wire holds one Drone per step attempt and no per-task Drone at all —
// `TaskView.drone_id` is the draft field that will carry it — so a step's own
// attempt is what a row is built from, and the row names the step it ran.
// It never collapses several Drones into one row, because there are not several
// to collapse yet.

function droneRowsOf(step: StepDetail, mint: () => number): LedgerRow[] {
  const rows: LedgerRow[] = [];
  for (const attempt of step.attempts) {
    rows.push({
      at: attempt.started_at,
      coord: { step: step.step_id, step_attempt: attempt.attempt },
      actor: "drone",
      kind: "drone_spawned",
      what: `a Drone opened ${step.label}`,
      outcome: "",
      cursor: mint(),
    });
    if (attempt.ended_at === undefined) continue;
    rows.push({
      at: attempt.ended_at,
      coord: { step: step.step_id, step_attempt: attempt.attempt },
      actor: "drone",
      kind: "drone_exited",
      what: `the Drone on ${step.label} stopped`,
      outcome: attempt.why === undefined ? attempt.outcome : `${attempt.outcome} — ${attempt.why}`,
      cursor: mint(),
    });
  }
  return rows;
}

// ---------------------------------------------------------- the plan's work

/** The plan being written down. **Fleet where a step recorded it**, a person where one did. */
function planRowsOf(detail: JobDetail, mint: () => number): LedgerRow[] {
  const plan = detail.work_plan;
  if (plan === undefined) return [];
  const by = plan.recorded_by;
  const tasks = plan.tasks.length;
  return [
    {
      at: plan.recorded_at,
      coord: by.by === "step" ? { step: by.step_id, step_attempt: by.attempt } : null,
      actor: by.by === "person" ? "person" : "fleet",
      kind: "plan_recorded",
      what: "the plan was recorded",
      // The approach is a paragraph and belongs to the Plan tab. What this row
      // owes a reader is how much work came out of it.
      outcome: `${tasks} ${tasks === 1 ? "task" : "tasks"}`,
      cursor: mint(),
    },
  ];
}

/**
 * What each task came to.
 *
 * **A task's own "done" is never Evidence** (`#1530`, 21 Sep) — it is one of
 * these, and `familyOf` files every one of them under Tasks.
 *
 * **The instant is the step's.** No field on the wire stamps a task, so a row
 * takes the end of the run of the step the task sits in; where that run is
 * still going it takes its start.
 */
function taskRowsOf(detail: JobDetail, mint: () => number): LedgerRow[] {
  const rows: LedgerRow[] = [];
  for (const task of taskViewsOf(detail)) {
    if (task.state === "open") continue;
    const step = detail.steps.find((one) => one.step_id === task.coord.step);
    rows.push({
      at: step === undefined ? detail.created_at : atOf(step, task.coord.step_attempt),
      coord: task.coord,
      actor: task.state === "dropped" ? "person" : "drone",
      kind: `task_${task.state}`,
      what: `${task.id} — ${task.title}`,
      outcome: task.reason ?? task.shown ?? "",
      cursor: mint(),
    });
  }
  return rows;
}

// ------------------------------------------------------------------- files

/**
 * What was written, and whether the plan covered it.
 *
 * Two sources and they say different things: the footprint is what git found on
 * the branch, and a finished task's scope is what the planner said that task
 * would touch. **A path outside the Job's declared write targets says so** on
 * either.
 */
function fileRowsOf(
  detail: JobDetail,
  footprint: JobFilesChanged | undefined,
  mint: () => number,
): LedgerRow[] {
  const rows: LedgerRow[] = [];
  if (footprint !== undefined) {
    for (const file of footprint.files) {
      rows.push({
        at: footprint.at,
        coord: { step: footprint.step_id, step_attempt: 1 },
        actor: "drone",
        kind: "file_written",
        what: file.path,
        outcome:
          file.outside_plan === true ? `${file.change}, outside the plan` : file.change,
        cursor: mint(),
      });
    }
  }
  const targets = detail.write_targets;
  for (const task of taskViewsOf(detail)) {
    if (task.state !== "done" || task.scope.length === 0) continue;
    const step = detail.steps.find((one) => one.step_id === task.coord.step);
    const outside = targets === undefined ? [] : task.scope.filter((path) => !within(path, targets));
    rows.push({
      at: step === undefined ? detail.created_at : atOf(step, task.coord.step_attempt),
      coord: task.coord,
      actor: "drone",
      kind: "task_files",
      what: task.scope.join(", "),
      outcome:
        targets === undefined
          ? "the Job declared no write targets"
          : outside.length === 0
            ? "inside the plan"
            : `outside the plan: ${outside.join(", ")}`,
      cursor: mint(),
    });
  }
  return rows;
}

// `write_targets` are prefixes — a directory, or a file. Absent is scope
// undetermined and present-and-empty is writing nothing, which is why the
// caller checks for absence rather than treating an empty list as "anywhere".
function within(path: string, targets: readonly string[]): boolean {
  return targets.some((target) => path === target || path.startsWith(target));
}

// ---------------------------------------------------------------- evidence

/** The copies of a deliverable Fleet kept, one row each. */
function evidenceKeptOf(step: StepDetail, mint: () => number): LedgerRow[] {
  return (step.deliverables ?? []).map((kept) => ({
    at: atOf(step, kept.attempt),
    coord: { step: step.step_id, step_attempt: kept.attempt },
    actor: "fleet" as const,
    kind: "deliverable_kept",
    what: kept.path,
    outcome: "",
    cursor: mint(),
  }));
}

/** What a Drone claimed, and the moment one handed in. */
function handedRowsOf(
  detail: JobDetail,
  evidence: readonly Submitted[] | undefined,
  handed: EvidenceSubmitted | undefined,
  mint: () => number,
): LedgerRow[] {
  const rows: LedgerRow[] = (evidence ?? []).map((one) => {
    const step = detail.steps.find((candidate) => candidate.step_id === one.step_id);
    return {
      at: step === undefined ? detail.created_at : atOf(step, lastAttemptOf(step)),
      coord: step === undefined ? null : coordOfStep(step),
      actor: "drone" as const,
      kind: "evidence_submitted",
      what: one.claimed,
      outcome: one.shown_by,
      cursor: mint(),
    };
  });
  // The pushed moment, kept only where the asked-for read has not landed: the
  // two are the same submission, and drawing both would be one hand-in twice.
  if (handed !== undefined && rows.length === 0) {
    rows.push({
      at: handed.at,
      coord: { step: handed.step_id, step_attempt: 1 },
      actor: "drone",
      kind: "handed_in",
      what: `a Drone handed in on ${handed.step_id}`,
      outcome: handed.evidence_type,
      cursor: mint(),
    });
  }
  return rows;
}

// ------------------------------------------------------------------- tests

/**
 * Every run of a case. **A case run is never an Evidence row** (`#1530`), and
 * `familyOf` files these under Tests.
 *
 * **A run with no step reads as after the Job** — the coordinate carries the
 * `null` and the surface says the words.
 */
function testRowsOf(detail: JobDetail, mint: () => number): LedgerRow[] {
  return caseRunsOf(detail).map((run) => ({
    at: run.ran_at,
    coord: run.coord,
    actor: run.actor === "contributor" ? ("contributor" as const) : (run.actor as LedgerActor),
    kind: "case_run",
    what: run.case,
    outcome: `${run.outcome === "ran" ? "ran" : run.outcome === "run_failed" ? "the run failed" : "not covered"}, ${run.frames} frames`,
    cursor: mint(),
  }));
}

// --------------------------------------------------------------- instants
//
// **A run of a step is the nearest instant the wire has for anything inside
// it.** A Check run, a Judge answer and a kept deliverable all carry which
// attempt produced them and none of them carries a clock, so each takes the end
// of that run — or its start, where the run is the one still going.

function attemptOf(step: StepDetail, attempt: number): StepAttempt | undefined {
  return step.attempts.find((one) => one.attempt === attempt);
}

function atOf(step: StepDetail, attempt: number): string {
  const run = attemptOf(step, attempt);
  // A run still going has no end, and its start is the wrong instant for a
  // Check that has already answered — every row inside a live step would stack
  // on the moment the Drone arrived. The step's own `updated_at` is the last
  // thing anything moved on it, which is the nearest upper bound there is.
  return run?.ended_at ?? step.updated_at ?? run?.started_at ?? step.entered_at;
}

function lastAttemptOf(step: StepDetail): number {
  return step.attempts.length === 0 ? 1 : step.attempts[step.attempts.length - 1]!.attempt;
}
