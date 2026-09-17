// One call a helm session made that the person's own settings do not cover,
// held open while they answer it in the dock. `crates/ipc/src/helm_call.rs`,
// #1389.
//
// **A `CommandInFlight` one subject over, and deliberately not that type.**
// Every field of that one is about a job — `step_id`, `allow_for_job`, a rule
// written into `armada.yml` on the job's branch — and a helm call has no job.
// The two sets of answers are different sets, and a shared union would offer a
// person an answer fleet cannot take.
//
// The header rules in `protocol.ts` hold here, with the exception `Settled`
// makes: these closed sets are the seam's own, with no registry behind them.

/**
 * What a person may answer about one helm call. Since protocol 14.18.
 *
 * `allow_once` runs it and writes nothing. `allow_and_remember` runs it and
 * writes `rule` into the repository's own `.claude/settings.local.json`, so the
 * agent CLI allows it without asking again — in helm and in their terminal
 * alike. `refuse` tells the session no.
 *
 * **Offered, never assumed.** `offers` carries the subset fleet will take, and
 * an answer outside it is a 409.
 */
export type HelmCallAnswer = "allow_once" | "allow_and_remember" | "refuse";

/**
 * One call a helm session is waiting on a person to answer, right now. Since
 * protocol 14.18.
 *
 * **Not a status, and no job.** Nothing is queued and no step is running while
 * it waits — what is waiting is one process, inside one tool call, for the
 * length of one reply. `manifest_id` is what names where it came from.
 */
export type HelmCallInFlight = {
  /** Fleet's id for this ask, and what an answer names. */
  call: string;
  manifest_id: string;
  asked_at: string;
  /** The tool reached for, in the harness's own spelling. */
  tool: string;
  /** The command or the argument, one line. Empty is a tool with no named argument. */
  detail: string;
  truncated: boolean;
  length?: number;
  /** The rule that would have to allow it, in the CLI's settings spelling — `Bash(gh issue list:*)`. */
  rule: string;
  /** In the order to draw them. */
  offers: HelmCallAnswer[];
  /** How long fleet holds the call open, from `asked_at`. Fleet's bound, said rather than derived. */
  holding_for_seconds: number;
};

/** The request half of `answer_helm_call`. Since protocol 14.18. */
export type AnswerHelmCall = {
  call: string;
  answer: HelmCallAnswer;
  /** Why, in the person's own words. **Only a refusal reads it.** */
  note?: string;
};

/** What `list_helm_calls` and `answer_helm_call` answer. Since protocol 14.18. */
export type HelmCallsWaiting = {
  waiting: HelmCallInFlight[];
};

/** `helm.asking_to_run`. Since protocol 14.18. */
export type HelmAskingToRun = {
  waiting: HelmCallInFlight;
};

/**
 * How one ask ended. Since protocol 14.18.
 *
 * `allowed_but_not_remembered` is a person's allow whose rule would not write —
 * **the call still ran**, because the answer was theirs and a settings file is
 * not a reason to refuse them. `unanswered` is fleet's bound running out, which
 * answers deny: silence is not consent.
 */
export type HelmCallSettled =
  | "allowed_once"
  | "allowed_and_remembered"
  | "allowed_but_not_remembered"
  | "refused"
  | "unanswered"
  | "session_gone";

/** `helm.call_answered`. Since protocol 14.18. */
export type HelmCallAnswered = {
  call: string;
  manifest_id: string;
  tool: string;
  detail: string;
  rule: string;
  settled: HelmCallSettled;
  at: string;
};
