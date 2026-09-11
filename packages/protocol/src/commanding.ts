// A command a drone reached for and was not given, and what a person answers.
// `crates/ipc/src/commanding.rs`.
//
// **Two paths, one set of answers.** A job set to `ask_me` holds its drone
// inside the permission call and a person answers `CommandInFlight` while it
// waits; a job at `refuse_and_hold` stops at `blocked_by_policy` and a person
// answers the `Refusal` row instead. `answer_command` takes both, and the call
// id says which.
//
// The header rules in `protocol.ts` hold here, with the exception `Settled`
// already makes: these two sets are the seam's own, with no registry behind
// them, so a union is the second copy of each and not a third. Bridge matches
// on both to choose which controls to draw, and sends both back.

/**
 * What a job does when its drone reaches for a command it was not given.
 * Since protocol 10.7.
 *
 * `refuse_and_hold` refuses the call and stops the job at `blocked_by_policy`
 * — **where every job starts**, because it asks nobody to be watching.
 * `ask_me` holds the drone inside the call and asks a person now.
 * `allow_all`, since protocol 11.0, runs every command without asking
 * **except two**, which still stop for a person: one `armada.yml` declares
 * destructive, and one the harness cannot grant — a push.
 *
 * **A third value was a major bump**, because a `switch` or a
 * `Record<WhenBlocked, …>` over this union is how a surface picks its control.
 */
export type WhenBlocked = "refuse_and_hold" | "ask_me" | "allow_all";

/**
 * What a person may answer about one refused command. Since protocol 10.7.
 *
 * `allow_for_job` runs it and lets this job run it again without asking.
 * `always_allow` does that and writes it into `armada.yml` under `commands`,
 * as its own commit on the job's branch. `reject` tells the drone no.
 *
 * **Offered, never assumed.** Each place a person answers carries the subset
 * fleet will take, and an answer outside it is a 409.
 */
export type CommandAnswer = "allow_for_job" | "always_allow" | "reject";

/**
 * One command a drone is waiting on a person to allow or reject, right now.
 * Since protocol 10.7.
 *
 * **Not a status.** The job and its step are `running` while the drone waits,
 * exactly as they are while a question is out, and the wait ends without
 * either moving. Present only under `ask_me`: under the default nothing waits,
 * and a person answers a `Refusal` on a stopped job instead.
 *
 * **`detail` is one line of the argument**, as a `Refusal`'s is. `truncated`
 * and `length` say how much was cut, and `get_call` serves the rest by `call`
 * — which matters more here, because what a person allows is the whole
 * command.
 */
export type CommandInFlight = {
  /** The harness's id for the call, and what an answer names. */
  call: string;
  /** Which step's drone is waiting. */
  step_id: string;
  /** When the harness asked, by fleet's clock. A surface ages it itself. */
  asked_at: string;
  /** The tool reached for, in the harness's own spelling. */
  tool: string;
  /** The command or argument, one line. Empty is a tool with no named argument. */
  detail: string;
  /** Whether `detail` is less than what was sent. */
  truncated: boolean;
  /** How many characters there were before the cut. Absent where unmeasured. */
  length?: number;
  /** What a person may answer, in the order to draw them. */
  offers: CommandAnswer[];
};

/**
 * The body of `answer_command`. Since protocol 10.7.
 *
 * **One body for both paths**, because the call id already says which: a call
 * a drone is waiting on is answered in place, and a refused row on a stopped
 * job restarts the step. There is no field for prose.
 */
export type AnswerCommand = {
  /** `CommandInFlight.call`, or `Refusal.call`. */
  call: string;
  /** One of that command's `offers`. */
  answer: CommandAnswer;
};

/**
 * The body of `set_when_blocked`. Since protocol 10.7. A live setting on one
 * job: the next permission question reads it, and no drone is respawned.
 */
export type SetWhenBlocked = {
  when_blocked: WhenBlocked;
};

/**
 * How far a person's allow reaches. Since protocol 11.0.
 *
 * `job` is this job only — `allow_for_job`. `repository` also wrote the
 * command into `armada.yml` under `commands` — `always_allow` — and **that line
 * outlives the row**: removing the allow from this job leaves the file as it is.
 */
export type Reach = "job" | "repository";

/**
 * One command a person allowed for this job. Since protocol 11.0.
 * `crates/ipc/src/commanding.rs`.
 */
export type AllowedCommandRow = {
  /** The command, whole, as the person allowed it. What `remove_allowed_command` names. */
  run: string;
  reach: Reach;
  /** When it was allowed, by fleet's clock. */
  allowed_at: string;
  /** Who allowed it, in the envelope's spelling. Left as `string` like `actor`. */
  by: string;
};

/**
 * The body of `set_model`. Since protocol 11.0. Answered with the job's
 * summary.
 *
 * A name `list_models` offers, or `null` to clear the choice so each later
 * step runs on the model its workflow gives it. **`null` is sent, never
 * implied**: fleet refuses a body with no `model` key rather than reading it as
 * a clear. The step running now keeps its model; the next step's spawn reads
 * this. A name `list_models` does not offer is a 409.
 */
export type SetModel = {
  model: string | null;
};

/**
 * The body of `remove_allowed_command`. Since protocol 11.0. Answered with the
 * job's summary.
 *
 * `run` is `AllowedCommandRow.run`, exactly. The next reach for the command is
 * answered by the job's `when_blocked` again, and **an always-allow already in
 * `armada.yml` stays there**. A command the job holds no allow for is a 409.
 */
export type RemoveAllowedCommand = {
  run: string;
};
