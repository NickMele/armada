// How many times a step was worked, and what each of those runs came to.
// `crates/ipc/src/attempt.rs`.
//
// Its own file for the reason the Rust side gives it one: it is not a row and
// not the workflow's, it is folded out of the job's own log — and `protocol.ts`
// was at the gate's ceiling.
//
// Hand-written like `protocol.ts`, and a second statement of the Rust shapes
// for the same reason: the codegen that would emit both does not exist yet.

/**
 * One run of one step: which run it was, when it began, and what it came to.
 *
 * **The outcome is a step state rather than a word of its own.** The inner
 * machine already names the six places a run can be — `advanced` is the run
 * that passed, `retrying` is the one that failed inside its budget, `stopped`
 * is the one that spent it, `awaiting_human` is the one holding for a person —
 * and a second vocabulary here would be a set no registry declares.
 *
 * **It is the only place an earlier run's outcome survives.** `StepDetail.state`
 * and `StepDetail.last_verdict` are both the latest, so a step that passed on
 * its third try and one that passed on its first were the same message.
 */
export type StepAttempt = {
  /** Which run this was, counted from one. */
  attempt: number;
  /** Where the run got to. `running` while it is still going. */
  outcome: string;
  /**
   * The escalation trigger the run carried out of `running`, where it carried
   * one. Absent on a run that advanced, which is what makes `Attempt 2
   * advanced` and `Attempt 1 refused` different sentences rather than one with
   * a blank in it.
   */
  why?: string;
  started_at: string;
  /** Absent is the run still going, and there is at most one of those. */
  ended_at?: string;
};
