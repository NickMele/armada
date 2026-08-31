// The wire vocabulary, as TypeScript sees it.
//
// **These are hand-written, and that is a gap rather than a design.**
// `crates/ipc/src/lib.rs` says a codegen step emits TypeScript from the Rust
// DTOs and that both generated outputs are checked in, so a cross-language
// breaking change is a build failure. That codegen does not exist yet, so the
// shapes below are a second statement of the ones in `crates/ipc/src/` and they
// drift the day a field moves. Nothing else in Bridge restates them.
//
// Every closed set is left as `string`. The Rust side refuses an arriving
// spelling the registry does not have; a union spelled here would be a third
// copy of a roster that already has two, and an unknown status renders as
// itself rather than as a guess.

import type { ProtocolVersion } from "./version";

/** A Job, as a list row. `crates/ipc/src/job.rs`. */
export type JobSummary = {
  id: string;
  /** What the Job is called. The one field on a row a person actually reads. */
  title: string;
  status: string;
  reason?: Reason;
  /**
   * Why an approved Job has not started. Its own field because it is derived
   * from the board at read time rather than recorded by a transition, which is
   * what `reason` carries — absent on every status but `queued`, and absent on
   * a queued Job that nothing is holding.
   */
  queued_reason?: string;
  workflow_id: string;
  owner_manifest_id: string;
  origin: string;
  urgency: string;
  atomic: boolean;
  model: string;
  /** Which step the Job is on. */
  current_step_id?: string;
  /** Presence, not state: absent is a Job no process is on. */
  assigned_drone?: string;
  /**
   * The Job this one replaces. A redispatch mints a new Job rather than
   * reopening the old one, so without this a board reads every second failure
   * as a first one.
   */
  redispatched_from?: string;
  /**
   * When the Job was created. On the row rather than only on the detail,
   * because elapsed is what answers "is this stuck" without opening it, and
   * reading it per row would be one request per row.
   */
  created_at: string;
  /** Absent until a worktree exists. A Job at the approval gate has none. */
  branch?: string;
};

/**
 * One Job, whole. The answer to `GET /jobs/:job_id`. `crates/ipc/src/detail.rs`.
 *
 * **Every optional field is omitted, never null.** Absent and empty are
 * different sentences on screen, and `write_targets` is the one that shows why:
 * absent is scope undetermined, present and empty is determined to write
 * nothing.
 *
 * Evidence, per-step Check results, the log file and spend are not here and are
 * not invented. Nothing serves them.
 */
export type JobDetail = {
  /** The board row, unchanged. A field added to the row reaches here for free. */
  job: JobSummary;
  /** What a whole-Job elapsed is measured from. Creation is not a transition. */
  created_at: string;
  /** Absent until a worktree exists. A Job at the gate has no branch. */
  branch?: string;
  /** One entry per step of the frozen WorkflowDef, in order. */
  steps: StepDetail[];
  acceptance_criteria: Criterion[];
  /** Context the Job was given. Absent where none was, rather than `""`. */
  facts?: string;
  /** Absent is scope undetermined; present and empty is writing nothing. */
  write_targets?: string[];
  subject?: Subject;
  /** The DAG edges this Job sits on. Empty until something writes one. */
  dependencies: Dependency[];
  /**
   * Other unfinished Jobs claiming to write where this one says it will. Since
   * protocol 5.3.
   *
   * **A fact, never a verdict.** Nothing here refuses a dispatch and no surface
   * may read it as one — `docs/concepts/fleet.md`, "Surfaced, never serialised".
   *
   * **Absent is not empty.** Absent is a Job that has claimed nothing yet, so
   * no comparison was made; that is every Job at its approval gate, because
   * the proposer does not fill `write_targets` in. An empty array is a
   * comparison that ran and found nobody. Drawing them the same way would say
   * "no overlap" about a Job nothing had looked at.
   */
  write_scope_overlaps?: ScopeOverlap[];
  /**
   * What the worktree held when the job stopped. Since protocol 4.12.
   *
   * **Absent on every job that is still going**, which is not a gap — a job
   * with a drone on it has a live reading, published as `job.files_changed`.
   * Absent is also a job that finished before Fleet kept these, and one whose
   * worktree would not open when it did. Present with no files is a worktree
   * that was read and held no change, which is a different sentence.
   */
  footprint?: JobFootprint;
  /**
   * What the job's branch came to: the commit, the push, the pull request.
   * Since protocol 5.3.
   *
   * **Absent is two different facts.** A job still running has nothing here.
   * So does a job that finished before Fleet wrote this down — which is most of
   * the jobs on any machine that has been running Armada a while, because the
   * result was assembled and dropped for a long time before it was stored.
   */
  delivery?: JobDelivery;
  /**
   * What the job has spent, against what it is allowed to. Since protocol 5.4.
   *
   * **Present on every job, including one that has spent nothing.** That is
   * what makes a job which cost nothing legible as such rather than as a job
   * Fleet has not measured, and it is why `drones` is on the payload: a cost of
   * zero across one drone and a cost of zero across none are different facts.
   */
  spend?: JobSpend;
  /**
   * The redirect this job's drone has been sent and has not answered yet.
   * Since protocol 4.14.
   *
   * **Absent is the second reading, not a gap.** Where a step had stopped, the
   * job went back to `running` on the send and there was never anything to wait
   * for; where none had, the job stays `escalated` until the drone takes a
   * turn, and this is the only thing on the wire that says so. A window that
   * remembered having sent one would lose it on a reload and never have it in a
   * second window, which is why the fact is here and not in Bridge.
   */
  redirecting?: RedirectInFlight;
  /**
   * The note a person wrote at a human gate that no drone has opened with yet.
   * Since protocol 5.2. **The other half of `redirecting`** — that one is an
   * instruction with a session to go into, this one is one with none, and until
   * a slot frees the job sits at `queued` looking exactly like a job nobody
   * typed anything into.
   *
   * **Absent is the ordinary case, and absent is also delivered.** Fleet clears
   * it the instant a drone's brief is built from it, so nothing here can be a
   * badge that goes stale: the delivery is the move to `running`, which is an
   * event this window already re-reads the open job on.
   */
  redirect_waiting?: RedirectWaiting;
  /**
   * What kind of stuck this job is, and what moves it. Since protocol 4.16.
   *
   * **Absent is "this job did not stop"**, and it is the whole of the second
   * reading: a queued, running, reviewing, piloted, superseded or landed job
   * carries nothing here, because a classification on one of those would offer
   * acts against a job nothing is wrong with. It is never an older fleet — one
   * behind this bridge is refused at the socket.
   */
  stuck?: Stuck;
};

/**
 * Why a job stopped, and what moves it. `crates/ipc/src/detail.rs`.
 *
 * **Fleet decides the acts and Bridge draws them.** Bridge used to derive them
 * from `status`, `current_step_id` and `assigned_drone` and reached four of the
 * five refusals `adrift.rs` carries; the fifth is whether the worktree is on
 * disk, which is a `path.is_dir()` no renderer can make. So a restart was
 * offered on a job that had none and the refusal arrived on the press.
 *
 * **It does not claim the trigger is true.** A drone whose worktree was deleted
 * escalates as `stalled`, the nearest trigger and the wrong condition; what
 * crosses is the escalation as recorded, beside the worktree fact.
 */
export type Stuck = {
  /**
   * The escalation trigger, in the registry's spelling. **Absent is a job that
   * recorded none** — one killed by hand stops no step and its transition
   * carries no reason.
   */
  stopped_by?: string;
  /**
   * The step that stopped, where a step-level trigger named one. **Absent on
   * every job-level escalation**, which is what makes a restart incoherent
   * there rather than merely refused.
   */
  step_id?: string;
  /**
   * The acts fleet will take on this job **now**, each spelled as the operation
   * that performs it, ordered by how much each takes away.
   *
   * **Empty is a dead end and says so**: nothing resumes this job and nothing
   * replaces it either, which is not the same as the field being absent. Left
   * as `string[]` like every other closed set, and here the rule earns itself
   * twice — the set is declared by the acts fleet implements rather than by a
   * registry, and `rerun_gate` was added to it after the concept doc had
   * written a table of five.
   */
  recourse: string[];
  /**
   * Whether the job's worktree is still on disk.
   *
   * **The fact that decides between a restart and a redispatch**, and the one
   * no surface can compute for itself. It rides beside the acts so a screen can
   * say *why* a restart is not offered rather than only that it is missing.
   */
  worktree_on_disk: boolean;
};

/**
 * A redirect that has gone into the drone's session and has not been answered.
 * `crates/ipc/src/detail.rs`.
 *
 * **A fact about the last act, not a status.** The job is `escalated` and stays
 * there — it returns to `running` on the drone taking a turn, which is evidence
 * it resumed rather than evidence somebody pressed a button. It arrives one way
 * only, on the open job's detail: the wait ends with the job's own move to
 * `running`, and that move is already an event.
 *
 * It says Fleet wrote to the pipe and no more than that. Whether the drone read
 * the instruction is answered by the next turn it takes, so there is no
 * delivery flag here and there is nothing on this seam that could set one.
 */
export type RedirectInFlight = {
  /**
   * When the instruction went into the session, by Fleet's clock. **A surface
   * subtracts; nothing ticks on the wire**, as `JudgeInFlight.since` does.
   */
  sent_at: string;
};

/**
 * A person's note written where no drone was there to take it, still waiting
 * for the one that comes next. `crates/ipc/src/detail.rs`.
 *
 * **It is the note or it is nothing**: the record holds the words and clears
 * them on delivery, so this value's presence *is* the fact that one is waiting,
 * and there is no delivered flag and no instant because there is no state
 * between the two.
 *
 * **The words cross, and a count would not do.** `RedirectInFlight` serves no
 * text because that instruction went into a live session and the move back to
 * `running` is the answer; this one has gone nowhere, and a field saying only
 * that *some* note waits leaves a person who wrote two no better off.
 */
export type RedirectWaiting = {
  /**
   * What the person typed, verbatim. **Never blank** — the record refuses a
   * note with nothing in it, so a present value always has words in it.
   */
  note: string;
};

/**
 * What a finished job's branch came to.
 *
 * **Three independent absences, and a surface must not fold them.** A commit
 * with no push is a repository that names no remote; a push with no pull
 * request is a machine with nothing that can open one. Neither is a failure,
 * and a row that treated them as one would say "unknown" about a branch that is
 * sitting on a remote right now.
 */
/**
 * What one job has spent and what it is allowed to spend.
 *
 * **Four numbers and no verdict**, deliberately. Whether the job is over is the
 * pair being compared, and a boolean could not say by how much or which of the
 * two ceilings it was — which is exactly what `queued_reason: "over_budget"`
 * leaves out.
 *
 * `cost_micros` and `cost_cap_micros` are millionths of a dollar, and they are
 * **notional**. The figure is what the run would have cost at list price, which
 * is not what a subscription account is billed; a surface that presents it as
 * money owed is presenting a currency nothing here spends. What it is for is
 * telling a runaway from a job that started with a cold cache.
 *
 * `ran_ms` has no cap beside it on purpose. Wall clock is bounded by a
 * different setting at a different scope, which nothing enforces yet, so the
 * figure is here to be read and there is no ceiling to draw it against.
 */
export type JobSpend = {
  /** What every drone of this job has cost, added up, in millionths of a dollar. */
  cost_micros: number;
  /** What it may cost before Fleet stops starting drones on it. */
  cost_cap_micros: number;
  /** How many turns every drone of this job has taken, added up. */
  turns: number;
  /** How many it may take before Fleet stops starting drones on it. */
  turn_cap: number;
  /** How long those drones ran, in milliseconds. No cap beside it — see above. */
  ran_ms: number;
  /** How many drones this is the sum of. Zero is a job nothing has run for. */
  drones: number;
};

export type JobDelivery = {
  /** The commit Fleet wrote over the job's work, by its id. */
  commit?: string;
  /** Where it was pushed, as `remote/branch`, or that there was no remote. */
  pushed?: string;
  /** The address a person clicks. */
  pull_request?: string;
};

/**
 * What one job's worktree held when the job stopped. On `JobDetail` rather than
 * a read of its own: it is a path and a word per file, and Fleet asks for it
 * only where a job has one, so an open of a running job pays nothing.
 */
export type JobFootprint = {
  /** Every file, in the order the reading found them. */
  files: TouchedFile[];
  /**
   * When the reading was taken. **The instant the job stopped**, not the
   * instant it was asked for — which is what makes this a record and lets a
   * surface say so.
   */
  recorded_at: string;
  /**
   * What each run of each step said its work would be, in declaration order.
   * Since protocol 4.17, and absent from a fleet older than that.
   *
   * **Empty is the whole of "there is nothing to be outside of."** Every
   * `TouchedFile.planned_by` is then absent rather than empty, so a surface
   * that never reads this list still cannot draw an unmeasured path as one
   * that stayed in scope.
   */
  plans?: DeclaredPlan[];
};

/**
 * What one run of one step promised its work would be. Since protocol 4.17.
 *
 * **The promise, beside the record of what was done.** A footprint is the job's
 * whole work and a plan belongs to one step, so the two arrive side by side
 * rather than folded into one mark. A step that never declared has no entry: it
 * is silent, not counted.
 */
export type DeclaredPlan = {
  step_id: string;
  /**
   * Which run of that step declared it, one-based. **A step may be worked twice
   * and then declares twice**, and without this the two entries would read as
   * one step contradicting itself.
   */
  attempt: number;
  /** When the declaration was taken, by fleet's clock. */
  declared_at: string;
  /**
   * The paths the drone named, each covering everything beneath it at a segment
   * boundary. **Empty is a declaration of nothing**, which every changed path is
   * outside of — not a step that never declared.
   */
  paths: string[];
};

/**
 * One file a finished job touched.
 *
 * **Not `ChangedFile`, and the drift mark is the reason.** A live reading
 * carries `outside_plan` as a boolean, because the step being watched declared
 * the plan it is measured against. A record is the job's whole work, and the
 * step holding the pen when a job stops is usually not the step that scoped it
 * — so one boolean here could only be right by accident. `planned_by` names
 * steps rather than asserting a verdict.
 */
export type TouchedFile = {
  /** Repository-relative, exactly as git spells it. */
  path: string;
  /** The same closed set `ChangedFile.change` carries, left as `string`. */
  change: string;
  /**
   * The steps whose declared plan covers this path, in `JobFootprint.plans`
   * order. Since protocol 4.17.
   *
   * **Three readings, and the absent one is why this is not a boolean.** Absent
   * is a job where no step declared anything, so nothing was measured. Present
   * and empty is a path outside every plan that was declared — the drift a
   * finished job could not state before. Present with steps in it is a path one
   * of those steps promised.
   */
  planned_by?: string[];
};

/** One step: which, where in the order, and where it got to. */
export type StepDetail = {
  step_id: string;
  /**
   * What a person reads — `Plan the change`, not `plan`.
   *
   * **Never absent and never blank.** Where the workflow declares no label, or
   * Fleet cannot say which workflow this is, Fleet substitutes the id, so no
   * client picks its own fallback and no two surfaces pick different ones.
   */
  label: string;
  /** Position in the frozen WorkflowDef, so a rail draws past and future. */
  ordinal: number;
  /** `job_steps.state`, served rather than inferred from the Job's status. */
  state: string;
  /**
   * The Checks this step declares, in the workflow's order.
   *
   * **Empty is "this step is ungated"; absent is "Fleet cannot say."** Those
   * are two different sentences on screen and the rail says each of them in
   * words — the key being missing means the Job named a workflow this Fleet
   * does not hold, which is not the same as a step that gates on nothing.
   */
  checks?: DeclaredCheck[];
  /** What each declared Check did. Empty until the gate has run them. */
  check_runs: CheckRun[];
  /**
   * The semantic tier this step declares, in the workflow's order. **Empty is
   * "the Judge will not look here"; absent is "Fleet cannot say"** — the two
   * sentences `checks` has. Neither is "nothing happens here", which is what
   * `advance_gate` answers.
   */
  judge_checks?: DeclaredJudge[];
  /**
   * What it takes to advance past this step — `auto`, `auto_if_judge_passes` or
   * `human_always`, left as `string` like every other closed set. **This is
   * what lets a step say it will stop before it stops**: `human_always` holds
   * the Job at `awaiting_review`, which six of the seven shipped workflows now
   * do. Absent on the same grounds as `checks`.
   */
  advance_gate?: string;
  /** Absent until a gate has ruled on the step. */
  last_verdict?: Verdict;
  /**
   * **The step advanced because a person overruled the gate, not because it
   * passed.**
   *
   * Served as a field rather than left as a rule a client applies, and that is
   * the point: the fact is already on the wire as `state: advanced` beside
   * `last_verdict: failed`, and every surface drawing a rail would otherwise
   * have to spell the same pair — the first one that forgot would draw a Judge
   * that had been overruled as a Judge that had cleared the work.
   *
   * Never absent, because it is a `bool` on the wire: `false` on every ordinary
   * advance. What was overruled is on `last_verdict`, which still names the
   * trigger, and the person's reason is in the Job's own log rather than here.
   */
  overridden: boolean;
  /**
   * Every criterion the Judge answered on this step, in the order asked.
   *
   * **Always present, empty on a step that asks nothing** — which is most of
   * them, and also every step the Judge never reached. This is where a
   * refusal's citation arrives, and it is the only thing that says what was
   * wrong with the work: the trigger says the gate stopped.
   */
  judged: Judged[];
  /**
   * Every gaming pattern this step's evidence tripped, with what each cites.
   *
   * **This is what `evidence_suspect` does not say** — the trigger says the
   * evidence is not to be trusted, and only these say which shape of gaming
   * was found and where. The same relation `judged` has to a `gate_failure`,
   * and the reason a person deciding whether to overrule a flag can be shown
   * what the flag was about. Empty on every step nothing was flagged on.
   */
  flagged: Flagged[];
  /**
   * The Judge call out on this step **right now**, where one is.
   *
   * **Absent is the ordinary case, and it is the point of the field.** A step
   * waiting on a model call, a step whose Drone is thinking and a step that has
   * quietly become unreachable were the same pixels on this side of the seam,
   * and nothing on the wire told them apart. `since` is what keeps a surface
   * from being a spinner: ninety seconds and two seconds are different facts,
   * the budget is two minutes, and the elapsed time is subtracted here rather
   * than pushed as an event a second.
   *
   * It is not a step state. `state` still says `running` while a gate asks, and
   * the six values it may take are unchanged.
   */
  judging?: JudgeInFlight;
  /** Entered, then moved on entering `running`. To `updated_at` is how long. */
  entered_at: string;
  updated_at: string;
};

/**
 * One Judge call, while it is still out. `crates/ipc/src/detail.rs`.
 *
 * Arrives two ways and means the same thing both times: on the open Job's
 * `StepDetail`, which is what a Bridge opened mid-call reads, and as the
 * `job.judging` event, which is what moves it without a reload.
 */
export type JudgeInFlight = {
  /**
   * `criterion`, `drift`, `gaming` or `convergence` — the four looks Fleet
   * makes. Left as `string` like every other closed set on this side: no
   * registry declares this one, so a union here would be a roster with no
   * authority behind it.
   */
  look: string;
  /**
   * Which criterion is being asked. **Joins to `judged`**, where the same id
   * reappears with a verdict once the answer lands. Absent on `gaming`, which
   * is about a pattern, and on `convergence`, which is about neither.
   */
  criterion_id?: string;
  /** Which gaming pattern is being asked about. Joins to `flagged`. */
  pattern?: string;
  /** Which model is out. What the wait costs, and roughly how long it is. */
  model: string;
  /** Which call of how many this pass is making. Counted from one. */
  call: number;
  /** Criteria times panel size, plus the drift look where the work drifted. */
  of: number;
  /** When the call went out. **A surface subtracts; nothing ticks on the wire.** */
  since: string;
  /** How long it may take before Fleet calls it a failed call. */
  budget_ms: number;
};

/** One Check a step declares. `crates/ipc/src/checks.rs`. */
export type DeclaredCheck = {
  /** `manifest_check` or `diff_nonempty`, as the WorkflowDef schema spells it. */
  kind: string;
  /** The Manifest Check's name. Absent on `diff_nonempty`, which names none. */
  name?: string;
  /** The command the Check resolved to, as the Job's frozen workflow holds it.
   * **Absent on `diff_nonempty`**, which runs nothing. Always the frozen
   * workflow's and never the live Manifest's — editing `armada.yml` mid-Job
   * must not change what a finished step says it ran. */
  run?: string;
  /** The exit code the step expects, where there is a command to return one. */
  expect_exit_code?: number;
  /**
   * Which paths this Check covers, as the Job's frozen workflow holds them.
   * A step that changes none of them does not run it.
   *
   * **Absent means always, and it is never `[]`.** Always and never are
   * opposite answers and Fleet sends no key at all for the first, so a client
   * has nothing to disambiguate. Draw it *before* the Check runs — that is the
   * only moment it says anything the `skipped` row will not say later.
   */
  when?: string[];
};

/**
 * One `judge_checks[]` entry a step declares, counted rather than quoted.
 * `crates/ipc/src/checks.rs`. **The declaration, never the answer** — what the
 * Judge said is `Judged`, one row per criterion. No question crosses: a
 * question is a prompt in a screenshot.
 */
export type DeclaredJudge = {
  /** How many yes/no questions this entry asks. Zero looks only for gaming. */
  criteria: number;
  /** How many judges answer each one. **Absent at one**, so present means a
   * panel — a client comparing against `1` would restate the domain's default. */
  panel_size?: number;
  /** Whether a second look asks whether the evidence was gamed. It does not
   * gate; what it found arrives as an escalation, not as a verdict. */
  gaming_check: boolean;
};

/**
 * One declared Check, as the gate found it.
 *
 * **`produced` is absent on a pass because a pass measured nothing** — the
 * outcome is the whole sentence. The other five each say something different
 * about why a step did not pass, and none of them is `failed`.
 *
 * **`skipped` is the one that did not stop the step.** The Check declares which
 * paths it covers and this step changed none of them, so it was not run — it is
 * not a pass and it is not a failure, and a surface that drew it as either
 * would be reporting a verification that never happened. `produced` names the
 * paths it covers.
 */
export type CheckRun = {
  /** The Manifest Check's name, or the built-in's kind. Joins to `checks`. */
  name: string;
  /** `check-outcomes.toml`: `passed`, `failed`, `signalled`, `timed_out`, `never_ran`, `skipped`. */
  outcome: string;
  /** What the Check was measured against. Absent on a pass, and on a skip. */
  expected?: string;
  /**
   * The exit code, the signal, the budget it outran, what is not installed —
   * or, on a skip, the paths the Check covers and this step did not touch.
   */
  produced?: string;
  /**
   * Where the Check's stdout and stderr were written, relative to the
   * repository root. **A reference, never the content**, and absent where there
   * is no file — a built-in assertion runs no command, and a Check that never
   * started printed nothing.
   */
  output_path?: string;
};

/**
 * One criterion the Judge answered.
 *
 * **A refusal is not a failed Check and does not read as one.** A Check says
 * the work is broken; a refusal says the work runs and is not what was asked
 * for, which is why one ends the Job and the other escalates it. The three
 * optional fields are what a refusal owes and a no-objection does not: there is
 * nothing to cite where nothing was refused, and `""` would read as a citation
 * somebody lost.
 */
export type Judged = {
  /** Which criterion was asked. Joins to `JobDetail.acceptance_criteria`. */
  criterion_id: string;
  /** `criterion_verdict_judge`: `met` or `not_met`. */
  verdict: string;
  /** What should be seen if the work were right. */
  expected?: string;
  /** What is seen instead. */
  produced?: string;
  /** What that difference does to whoever consumes it. The triage line. */
  consequence?: string;
};

/**
 * One gaming pattern found, and what it was found in. **Never a verdict** — a
 * flag says the evidence is suspect, not that the step failed. `pattern` is a
 * string because no registry declares the set: it comes from what a workflow's
 * `flag_if` names.
 */
export type Flagged = {
  /** The pattern, spelled as `flag_if` spells it. */
  pattern: string;
  /** The file, line or assertion the flag is about. An uncited flag is unactionable. */
  cited: string;
};

/** The last ruling against a step. `failed` carries its trigger; the rest do not. */
export type Verdict = {
  named: string;
  trigger?: string;
};

/** One acceptance criterion, with the id a Judge citation references. */
export type Criterion = {
  criterion_id: string;
  text: string;
  source: string;
};

/** What a Job is about. Neither sequencing nor provenance. */
export type Subject = { kind: string; reference: string };

/** One DAG edge, sequencing peer Jobs. */
export type Dependency = {
  direction: string;
  peer: string;
};

/**
 * What a redispatch did. **Two Jobs, because a redispatch is two acts** — the
 * failed one is killed and a replacement is minted carrying
 * `redispatched_from`. Nothing here reopens anything.
 */
export type Redispatched = {
  /** The Job that failed, now `killed`. Its worktree is as its Drone left it. */
  replaced: JobSummary;
  /** The replacement, at the approval gate. What the caller opens next. */
  dispatched: JobSummary;
};

/**
 * The body of `redirect_drone`. `crates/ipc/src/job.rs`. The one string that
 * reaches a Drone without Fleet assembling it — blank is refused server-side.
 */
export type Redirection = {
  instruction: string;
};

/**
 * The body of `override_verdict`. `crates/ipc/src/work.rs`.
 *
 * **Its own type though it is structurally the same string as `Redirection`**,
 * for that type's own reason turned around: a redirect steers a Drone, and this
 * one goes nowhere near one. It is written for the record and for whoever later
 * asks how often the Judge was wrong. Blank is refused server-side with a 422,
 * and refused here before the press for the same reason.
 */
export type Overruled = {
  reason: string;
};

/** The reason a transition carried, where it stored one. */
export type Reason = {
  named?: string;
  criteria_owed?: string[];
};

/** A Job on disk that would not load. Never filtered away. */
export type UnreadableJob = {
  job_id?: string;
  fault: string;
};

/** Every Job, and every one that would not load. */
export type JobList = {
  jobs: JobSummary[];
  unreadable?: UnreadableJob[];
};

/** A Job drafted onto the approval gate. The request half of `propose_job`. */
export type ProposeJob = {
  /** Required. A proposal without one does not decode on the Rust side. */
  title: string;
  workflow_id: string;
  owner_manifest_id: string;
  /** One of the four top-level origins. `sub_dispatched` does not deserialise. */
  origin: string;
  urgency: string;
  atomic: boolean;
  /**
   * Optional, and absent is the ordinary case: Fleet fills it from
   * configuration. It used to be required, and the `""` that invited was
   * accepted, stored, drawn on the board and refused at spawn.
   */
  model?: string;
  acceptance_criteria?: ProposedCriterion[];
  subject?: { kind: string; reference: string };
  /** Context the Job needs to run. Append-only once the Job exists. */
  facts: string;
  /** Null is not empty: absent is scope not yet determined. */
  write_targets?: string[];
  /**
   * Files staged before the Job existed. Fleet reads the bytes itself, at
   * `staged_path`, on the same machine — nothing here carries a payload.
   */
  attachments?: AttachmentRef[];
};

/**
 * One other Job claiming paths this one claims. `crates/ipc/src/overlap.rs`.
 *
 * One entry per Job rather than per path: a person decides about the other Job,
 * so the Job is the row and the paths are its detail.
 */
export type ScopeOverlap = {
  job_id: string;
  /** What the other Job is called. Carried, so a card names it rather than an id. */
  title: string;
  /** Where the other Job is. `running` and `awaiting_review` are weighed differently. */
  status: string;
  /** Never empty. The entry exists because a path is shared. */
  paths: SharedPath[];
};

/**
 * One place both Jobs claim, and who claimed it on each side. A step id is a
 * Drone's own declaration; absent is the Job's `write_targets`, which the
 * requester stated before anything ran.
 */
export type SharedPath = {
  /** The narrower of the two claims — where the collision is, not what contains it. */
  path: string;
  this_step?: string;
  other_step?: string;
};

export type ProposedCriterion = { text: string; source: string };

/**
 * One staged file, named to Fleet. `staged_path` is an absolute path on the
 * machine Fleet runs on — the same-machine assumption this seam already
 * makes (`docs/practices/protocol.md`). Fleet reads the bytes itself; Bridge
 * never sends a payload over this channel.
 */
export type AttachmentRef = { staged_path: string; filename: string; mime_type: string };

/**
 * What a person sends to say a Job failed in error. `crates/ipc/src/report.rs`.
 *
 * **`said` is the whole reason this exists.** Everything Fleet attaches around
 * it is already served by three other routes; the sentence is the one thing
 * that does not exist anywhere until somebody types it, and Fleet answers 422
 * without it.
 *
 * `claim` is left as `string` like every other closed set here — but it is the
 * one Bridge *writes* rather than renders, so the three values it may hold are
 * named where the picker offers them, in `renderer/src/Report.tsx`.
 */
export type FileReport = {
  claim: string;
  said: string;
  /**
   * The step the report is about. **Sendable without `criterion_id`**, which is
   * what a report about a step the gate judged nothing on looks like — an
   * undecided gate records no verdict, so there is none to name.
   */
  step_id?: string;
  /** Sent with `step_id` and never without it: a criterion id is unique inside a step. */
  criterion_id?: string;
};

/**
 * One filed report, as it reads afterwards. `crates/ipc/src/report.rs`.
 *
 * **`record` is the Job's own evidence rendered at filing time**, not a join to
 * rows that are still there: `armada clean` forgets a Job and takes every row
 * beneath it, and the report stays whole. `job_id` may therefore name a Job
 * that no longer exists, which is deliberate.
 */
export type Report = {
  id: string;
  filed_at: string;
  /** `human`. The column exists so the day Fleet files its own, it is a value. */
  origin: string;
  claim: string;
  job_id: string;
  job_title: string;
  step_id?: string;
  criterion_id?: string;
  /** The person's own words. The finding. */
  said: string;
  /** The record, as the body of an issue. The evidence. */
  record: string;
};

/** Every report filed, and the counts they are read beside. */
export type ReportList = {
  /** Newest first, bodies included. */
  reports: Report[];
  calibration: Calibration;
};

/**
 * What is known about whether the Judge has been right.
 *
 * **Four counts and not a rate.** A rate's denominator would count every Job
 * nobody read, and an unread Job is not a pass — so the gap between what the
 * Judge refused and what a person disputed is left visible rather than divided
 * away.
 */
export type Calibration = {
  refusals_recorded: number;
  refusals_disputed: number;
  /** Not the other half of the same number: a wrong pass is refused by nothing. */
  passes_disputed: number;
  reports_filed: number;
};

/**
 * What forgetting a Job leaves to say. `crates/ipc/src/job.rs`.
 *
 * **The id, and nothing else** — there is no row left for a summary to
 * describe. `forget_job`'s command response and `job.forgotten`'s event
 * payload are the same type on the Rust side, so this is used both ways here
 * too rather than declared twice for one fact.
 */
export type JobForgotten = { job_id: string };

// Every event shape, re-exported so `protocol.ts` stays the one import for the
// wire vocabulary. They live in `events.ts` because this file reached the 900
// lines the gate refuses; the cut is the socket seam `crates/ipc/src/event.rs`
// already draws.
export type {
  ChangedFile,
  Delivered,
  Event,
  JobCreated,
  JobFilesChanged,
  JobJudging,
  JobStateChanged,
  JobStepAdvanced,
  Missed,
  Resync,
  StreamMessage,
} from "./events";

/** A failure, flattened for the wire. `docs/contracts/error-contract.md`. */
export type WireError = {
  /** Opaque to Bridge: looked up, never parsed. */
  code: string;
  /** What renders when the lookup misses. */
  message: string;
  run_id: string;
  fields: Record<string, unknown>;
  chain: string[];
  job_id?: string;
  drone_id?: string;
  step_id?: string;
};
