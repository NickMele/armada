// One Job read whole, and the step it is on. `crates/ipc/src/detail.rs`.
//
// **Split out of `protocol.ts` at the 900-line line, and the cut is one the
// Rust side already draws.** That file was a second statement of two modules at
// once: `job.rs`, which is a Job as a row and a list carry it, and `detail.rs`,
// which is the answer to `GET /jobs/:job_id`. Everything here is `detail.rs`'s
// half, down to which module `Dependency` and `Settled` belong to — so the two
// languages now name the same seam, and a field that moves in one has one file
// to move in on the other.
//
// **`protocol.ts` does not re-export these**, for the reason `proposal.ts` and
// `proposing.ts` give: nothing outside this package imports a module by path,
// every caller reaches the wire through the package index, and a re-export
// block would spend the lines that cut was made to free.
//
// `JobSummary` is imported from `protocol.ts` and `Settled` is imported back
// from here. That is a cycle of `import type` alone, which the compiler erases
// and no bundler ever sees — the alternative is a copy of one of the two, and a
// copy is the thing this file exists to stop.
//
// The header rules there hold here: these are hand-written, they drift the day
// a field moves, and every closed set is left as `string`.

import type { JobFootprint } from "./footprint";
import type { Flagged, Judged, KeptDeliverable } from "./judged";
import type { KeptFrame, ShowAgain } from "./showing";
import type { StepAttempt } from "./attempt";
import type { ChecksUnderway } from "./underway";
import type {
  CheckRun,
  DeclaredCheck,
  DeclaredJudge,
  JobSummary,
  ScopeOverlap,
  Subject,
} from "./protocol";
import type { QuestionInFlight, RedirectInFlight, RedirectWaiting } from "./waiting";
import type { CommandAnswer, CommandInFlight, WhenBlocked } from "./commanding";

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
   * What the job has spent, against what it is allowed to. Since protocol 5.5.
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
   * The question this job's drone asked and nobody has answered yet. Since
   * protocol 5.7.
   *
   * **Absent is the ordinary case**, and it covers three things the job's own
   * status tells apart: a drone that never asked, a drone whose question has
   * been answered, and a job with no drone on it. A job that is not `running`
   * has nothing waiting on an answer.
   *
   * **It is not a status and there is no seventh step state.** The job is
   * `running` and its step is `running` while it waits, exactly as they are
   * while a judge call is out, and the question stops being outstanding without
   * either moving.
   */
  asking?: QuestionInFlight;
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
  /**
   * Whether a person can ask this Job to show its work again, and every time
   * somebody did. Since 10.1. **Absent from an older Fleet**, which draws as
   * no control at all rather than a refusal.
   */
  show_again?: ShowAgain;
  /**
   * What this job does when its drone reaches for a command it was not given.
   * Since protocol 10.7. **Absent from an older fleet**, which draws as no
   * setting at all rather than as the default.
   */
  when_blocked?: WhenBlocked;
  /**
   * The command this job's drone is waiting on a person to allow, right now.
   * Since protocol 10.7.
   *
   * **Absent is the ordinary case**, and it is every job at `refuse_and_hold`:
   * nothing waits there, and a refused command is answered on `stuck.refused`.
   */
  command_waiting?: CommandInFlight;
  /**
   * The review Fleet composed at this job's gate — the same text a pull
   * request carries, where this job has one. Since protocol 10.10.
   *
   * **One builder.** `crates/fleet/src/review.rs` composes this and the pull
   * request's Markdown body from the one reading of the record; the review
   * area draws these sections instead of assembling its own copy of them —
   * `#665`.
   *
   * **Absent is a job that has not reached a gate yet**, not an empty review:
   * a job still running, or one that finished with no `human_always` step at
   * all, carries nothing here. It is also every job read from a Fleet older
   * than 10.10, which draws the same as one that has not reached a gate.
   */
  review?: JobReview;
};

/**
 * The review Fleet composed, in the four parts it is made of. No heading
 * crosses — a heading is how a surface draws a section, not what the section
 * is, and the pull request's own Markdown adds its headings back at render
 * time from the same four parts. `crates/ipc/src/detail.rs`.
 */
export type JobReview = {
  /** The brief, in the requester's own words. */
  why: string;
  /** What the job's worktree changed, as far as a diff can say it. */
  outcome: string;
  /**
   * What nothing checked, and what the base carries that this job did not
   * write, where there was a base to ask.
   */
  risks: string;
  /** Every step and every Check that ran against it, with its outcome. */
  evidence: string;
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
   * The sentence Fleet logged when the gate could not read what it needed to
   * rule, in Fleet's own words — e.g. "the Judge did not answer inside its
   * budget".
   *
   * **Present only where `stopped_by` is `gate_undecided`.** Every other
   * trigger names its own trouble in words this build already has, off the
   * registry; this is the one whose cause is an artifact the gate could not
   * obtain, and there is no closed set of those to give a row of their own.
   */
  undecided?: string;
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
  /**
   * Whether a drone is standing on this job that fleet cannot hear.
   *
   * **The other fact no surface can compute**, and what a restart is about to
   * do turns on it: a drone that outlived the fleet holding its pipes is alive
   * and unreachable at once, so the button said it had gone beside a step in
   * flight. The trigger does not answer this — here it reads `gate_failure`.
   * False is every ordinary stopped job, including one whose drone really is
   * gone. Both are real, and this is what tells them apart.
   */
  drone_unheard: boolean;
  /**
   * What the drone reached for and was refused, oldest first. Since protocol
   * 7.5.
   *
   * **The trigger's evidence, and nothing carried it.** `blocked_by_policy`
   * named a policy and no surface named what it stopped, so a person was told
   * to widen an allowlist without being told what to widen it to — the tool and
   * the command sat on two transcript rows joined by a call id that nothing
   * joined.
   *
   * **Empty is a drone that was refused nothing**, which is most of them. It
   * rides on every trigger and not only `blocked_by_policy`: a drone denied the
   * command it needed goes on to escalate as `stalled` or `silent` just as
   * often.
   */
  refused: Refusal[];
  /**
   * How many calls were refused altogether, counting the ones `refused` left
   * out. Since protocol 7.5.
   *
   * **A size rather than a flag**, so a surface says *showing 50 of 137*
   * instead of reporting that something was taken away. Equal to
   * `refused.length` on every job whose refusals all fit, which is the ordinary
   * case.
   */
  refusals: number;
};

/**
 * One call the drone reached for and was refused. `crates/ipc/src/detail.rs`.
 *
 * **`detail` is the field a person reads.** The harness usually sends no reason
 * — the observed `permission_denied` line carried an empty `decision_reason` —
 * so a row drawn from `because` alone draws nothing, which is the whole of what
 * was wrong.
 *
 * **The whole argument stays in fleet's file.** A heredoc is a whole file and
 * this crosses on every open of a stopped job, so `detail` is one line of it
 * and `truncated` and `length` are how a row says so. `get_call` is the
 * operation that serves the rest.
 */
export type Refusal = {
  /** The tool that was reached for, in the harness's own spelling. */
  tool: string;
  /**
   * The call id the transcript rows carried.
   *
   * **What makes a cut `detail` openable**: `get_call` serves the whole argument
   * by this id, so a refused heredoc shown to its bound is not a dead end.
   */
  call: string;
  /**
   * The argument as the transcript recorded it — the command, the path.
   *
   * Bounded by fleet at 200 characters, so it is one line. **Empty is a tool
   * whose arguments the decoder has no name for**, and never an invented one.
   */
  detail: string;
  /**
   * Whether `detail` is less than what the drone sent. Since protocol 7.6.
   *
   * **Said rather than implied**, because a command can legitimately end in an
   * ellipsis. A row that drew a cut command as the whole one would have
   * somebody paste a truncated command into an allowlist, which is the failure
   * this field exists to prevent — one step on from the failure `refused`
   * itself exists to prevent.
   */
  truncated: boolean;
  /**
   * How many characters the argument had, before anything was cut. Since
   * protocol 7.6.
   *
   * **A size rather than only a flag**, so a row reads *showing 200 of 14,320
   * characters* instead of reporting that something was taken away.
   *
   * **Absent is a transcript row written before the file recorded the size.** A
   * row holding `truncated: true` and no length has what there is and no way to
   * say how much is missing, and must say that rather than invent a size.
   */
  length?: number;
  /**
   * The harness's own wording, where it gave one.
   *
   * **Usually empty, and empty is the honest answer.** Nothing fills it in from
   * the trigger, so a surface must not either — `detail` is what carries the
   * meaning.
   */
  because: string;
  /**
   * What a person may answer about this command. Since protocol 10.7.
   *
   * **Empty is nothing a person can allow here**; `withheld` says why where
   * fleet knows. Optional because fleet reads an absent list as empty, and a
   * reader here does the same rather than drawing a gap.
   */
  offers?: CommandAnswer[];
  /**
   * Why this command cannot be allowed from here — declared destructive, or a
   * push. Since protocol 10.7. Absent where it can be.
   */
  withheld?: string;
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
  /**
   * How many of those named no price, and so are counted in `drones` and in
   * none of the figures above.
   *
   * **`cost_micros` is a floor while this is non-zero.** Cost reaches Armada on
   * the terminating line of a session, so a drone signalled mid-run leaves
   * none — and a total drawn without saying so reports a job as having spent
   * less than it did.
   *
   * Absent on a Fleet built before it could tell the two apart, which reads as
   * nought: nobody was counting, so nothing is claimed.
   */
  unpriced?: number;
};

export type JobDelivery = {
  /** The commit Fleet wrote over the job's work, by its id. */
  commit?: string;
  /** Where it was pushed, as `remote/branch`, or that there was no remote. */
  pushed?: string;
  /** The address a person clicks. */
  pull_request?: string;
  /**
   * Live facts about it, off Fleet's own rotation rather than this call.
   * Since protocol 10.2.
   *
   * **Never fetched on this call.** Fleet already asks the forge about every
   * open pull request on a fixed rotation, to notice a merge; this is that
   * same reading, cached rather than fetched again.
   *
   * **Absent is two different facts, told apart by `landed`.** Where `landed`
   * is also absent, Fleet's rotation has not reached this pull request yet.
   * Where `landed` is present, the pull request has settled and Fleet has
   * stopped asking — read `landed` instead. Present always means `landed` is
   * absent: a reading taken while the pull request was still open, stale by
   * at most one rotation.
   */
  pull_request_detail?: PullRequestDetail;
  /**
   * What became of that pull request. Since protocol 6.6. **Absent is unasked
   * or still open** — one absence, because Armada opens a pull request and a
   * person merges it, so "still open" is the fact that nothing has happened.
   */
  landed?: Settled;
};

/**
 * What Fleet's rotation last read live off an open pull request. Since
 * protocol 10.2.
 *
 * **A snapshot, not a subscription.** Nothing pushes an update when one of
 * these changes; reopen the job after Fleet's rotation has had time to come
 * back around for a fresher one.
 */
export type PullRequestDetail = {
  /** The forge's own number for it, where the forge answered one. */
  number?: number;
  /**
   * Its title, as the forge holds it right now — not the title Armada opened
   * it with, because a person may have edited it since.
   */
  title?: string;
  /**
   * Whether the forge can merge it into its base as it stands. **Absent is
   * not "conflicting"** — it is the forge declining to say.
   */
  mergeable?: boolean;
  /**
   * Every review that has landed a verdict, oldest first. **Comments are not
   * here** — `get_remarks` serves everything anybody wrote, and a review's
   * note would cross twice if it were carried on both.
   */
  reviews: ReviewedBy[];
};

/** One reviewer's verdict on a pull request. Since protocol 10.2. */
export type ReviewedBy = {
  /** The login of whoever reviewed it, as the forge spells it. */
  by: string;
  /**
   * One word from the forge's own vocabulary: `approved` or
   * `changes_requested` — not every state a review can be in, only the two a
   * person acts on.
   */
  verdict: string;
};

/**
 * The two ends a pull request comes to, and the whole of the set. Since 6.6.
 *
 * **Neither open nor unknown is here**, which is what makes it closed: a pull
 * request that has not settled is the absence of the value, so no variant
 * means "no news" and nothing tells one kind of nothing from another.
 */
export type Settled = "merged" | "closed_unmerged";

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
  /**
   * What every declared Check did, every run of the step, oldest first.
   * Empty until the gate has run them. Since 7.0 this was the latest run
   * alone; join to `attempts` by `attempt`.
   */
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
  /**
   * Whether this is the step the frozen workflow sends the work out on. Since
   * protocol 10.2. **Absent means "Fleet cannot say"**, exactly as `checks`
   * does. Present is always the frozen workflow's own answer, so `false` is
   * as certain as `true` — a client can say "this workflow never opens a
   * pull request" from this field alone.
   */
  delivers?: boolean;
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
   * Every criterion the Judge answered, every run of the step, oldest first.
   *
   * **Always present, empty on a step that asks nothing** — which is most of
   * them, and also every step the Judge never reached. This is where a
   * refusal's citation arrives. Since 7.0 this was the latest run alone; join
   * to `attempts` by `attempt`.
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
   * The copies of this step's deliverable Fleet kept, oldest run first.
   *
   * **The third of the three records a verdict is argued with**, beside
   * `check_runs[].output_path` and `judged[].brief_path`: what the Judge read,
   * what the Checks printed, and what the Judge was asked. One without the
   * others cannot separate a bad Judge from a bad brief, which is why all three
   * are on the wire rather than the two that were.
   *
   * **Absent is the ordinary case** — a step that declares no deliverable keeps
   * none, and so does one whose Judge was never asked.
   */
  deliverables?: KeptDeliverable[];
  /**
   * The frames this step's harness produced, oldest run first. Since 9.2.
   *
   * **What a step whose point is not the code is reviewed by.** On a change
   * that should make something look different, the patch is the least useful
   * thing on the screen and it used to be the only thing offered — reviewing
   * meant reading a diff to infer an outcome you could have been shown. These
   * are the outcome.
   *
   * **Rows, never images.** Each says what it is called, what it weighs and
   * what to ask for; the bytes are fetched from the frame route, once, by
   * whoever opens one. A detail is re-read on every event naming the open Job,
   * and a frame is hundreds of kilobytes.
   *
   * **Absent rather than empty**, which is `deliverables`' shape and its
   * reason: Fleet drops the field where there are none, and a peer built before
   * 9.2 sends no such field at all. Both are *this step has no frames*, and a
   * reader that required the key would break on the second.
   *
   * That is the ordinary case — every step that declared no `shown` evidence,
   * which is most of them, and one whose harness ran and captured nothing.
   * **Absent never means the harness failed**: a repository with a broken
   * harness and a spec that photographed nothing look the same here, and what
   * happened is a line in the Job's own log.
   */
  frames?: KeptFrame[];
  /**
   * Every run of this step, oldest first. **`Attempt 1 refused`, `Attempt 2
   * advanced`** — the rows the run tree draws under a step that was worked
   * more than once.
   *
   * It is the only place an earlier run's outcome survives: `state` and
   * `last_verdict` are both the latest, so a step that passed on its third try
   * and one that passed on its first were the same message. Empty on a step
   * nothing has entered, which is every step a Job has not reached.
   */
  attempts: StepAttempt[];
  /**
   * What each closed run of this step came to, oldest first. Since 7.0.
   *
   * **What `attempts[].outcome` cannot say** — `passed` or `failed` in so
   * many words, not just where a run ended. Join to `attempts` by `attempt`.
   */
  verdicts: Verdict[];
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
  /** The step's Checks while the gate runs them. Since 10.3. `underway.ts`. */
  checking?: ChecksUnderway;
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

/** The last ruling against a step. `failed` carries its trigger; the rest do not. */
export type Verdict = {
  /** Which run of the step this was ruled on, counted from one. Since 7.0.
   * Joins to `StepDetail.attempts`. */
  attempt: number;
  named: string;
  trigger?: string;
};

/** One acceptance criterion, with the id a Judge citation references. */
export type Criterion = {
  criterion_id: string;
  text: string;
  source: string;
};

/** One DAG edge, sequencing peer Jobs. */
export type Dependency = {
  direction: string;
  peer: string;
};
