// One proposal, while the Job proposer is still reading it. Mirrors
// `crates/ipc/src/proposing.rs`.
//
// **Its own file rather than another block in `protocol.ts`**, which is where
// it started and which the 900-line refusal ended. The split is the one the
// Rust side already draws: these types are about the interval before a Job
// exists, and every other DTO in that file is about a Job that does.

/**
 * One Job proposer call, while it is still out. `crates/ipc/src/proposing.rs`.
 *
 * **`JudgeInFlight` one step earlier, and the step is the difference.** That one
 * rides on a step of a Job and arrives on the open Job's detail; a proposal is
 * the interval before any Job exists, so it arrives only as `proposal.moved` and
 * there is nothing to read it off.
 *
 * **This one ticks.** A person is waiting in front of a form, and an elapsed
 * count draws "thinking hard" and "never reached the vendor" identically.
 */
export type ProposalInFlight = {
  /** What a stop names. Fleet minted it; a client learns it from here. */
  proposal_id: string;
  /**
   * The caller's own token, echoed unchanged. **How a client recognises its own
   * call** — `proposal_id` is minted after the request arrives, so there is
   * nothing else to match on, and matching the request's text would match the
   * wrong call when two people dispatch the same words.
   */
  client_ref?: string;
  /** Which model is reading the request. What the wait costs, roughly. */
  model: string;
  /** When the call went out. A surface subtracts for itself. */
  since: string;
  /**
   * How long Fleet will wait before giving up. **What makes the elapsed count
   * mean something** — against nothing it can only say "slow"; against the
   * ceiling it can say how much of the decision is left.
   */
  budget_ms: number;
  /** How far the call has got. */
  reached: ProposalReach;
  /**
   * The harness's own running estimate of how much the model has thought.
   * Cumulative within this call and **an estimate** — drawn as an approximation,
   * never as a billed figure. Absent before it starts, and on a model that does
   * not think.
   */
  thinking_tokens?: number;
  /**
   * How much of the answer has arrived, in characters. **A count and never the
   * text**: what the proposer decided arrives as the Jobs it minted.
   */
  answered_characters?: number;
};

/**
 * How far a proposal has got. `crates/ipc/src/proposing.rs`.
 *
 * **A union here where `JudgeInFlight.look` is a bare `string`**, and the
 * difference is who decides. A look is decided by the call sites that make one,
 * so a roster here would have no authority behind it. These five are decided by
 * the shape of a model call, and a surface draws a different sentence for each.
 */
export type ProposalReach =
  /**
   * Started and has said nothing yet. **The one worth telling apart**: ninety
   * seconds here never reached the vendor at all, which will not resolve by
   * waiting — the opposite decision from every other value.
   */
  | "starting"
  /** The harness is up. It has not asked yet. */
  | "started"
  /** The question is at the vendor. Everything after this is the model's time. */
  | "requesting"
  /** Thinking. `thinking_tokens` says how much. */
  | "thinking"
  /**
   * The answer is arriving. `answered_characters` says how much. **Nearly
   * over** — stopping here throws away work about to land.
   */
  | "answering";
