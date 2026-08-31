// The question vocabulary, as TypeScript sees it. `crates/ipc/src/detail.rs`.
//
// **Split out of `protocol.ts` for `events.ts`'s reason and by the same rule.**
// That file reached the 900 lines the gate refuses again. The cut is a seam the
// Rust side already draws: `ask_question` is the Fleet/Drone tool and
// `answer_question` is the Fleet/Bridge command, and these three types are the
// only ones on the wire that belong to both halves of one act. `protocol.ts`
// re-exports every name here, so nothing that imported one had to change.
//
// The header rules there hold here: hand-written, they drift the day a field
// moves, and every closed set is left as `string`.

/**
 * One question a drone asked a person, while it is still unanswered.
 * `crates/ipc/src/detail.rs`.
 *
 * Arrives two ways and means the same thing both times: on the open job's
 * detail, which is what a bridge opened mid-question reads, and as the
 * `job.asking` event, which is what moves it without a reload.
 *
 * **A question is an event on a job, not a conversation.** Asked once, answered
 * once: one outstanding per job, the answer one of the options the drone
 * offered, and no field a reply could arrive in. A person who needs to say
 * something the options do not cover uses `redirectDrone`, which is the one
 * route their own words reach a drone by.
 *
 * `asked_at` crosses once and a surface subtracts for itself. **Nothing ticks**
 * — a question that waits an hour costs the stream two messages.
 */
export type QuestionInFlight = {
  /**
   * The id fleet minted for this question, and what an answer names.
   *
   * **This is what makes a stale answer a refusal.** A window left open across
   * an answered question would otherwise send a label that matched the *next*
   * question's options by coincidence, and what a wrong answer produces here is
   * dispatched jobs that run and spend.
   */
  question_id: string;
  /** Which step's drone is waiting. A job runs one step at a time. */
  step_id: string;
  /** When the drone asked, by fleet's clock. A surface ages it itself. */
  asked_at: string;
  /** What was asked, in the drone's own words. One question, not a thread. */
  question: string;
  /**
   * What the drone will accept as an answer. **Never fewer than two and never
   * more than four**, every label distinct — fleet refuses the tool call
   * otherwise, so these can be drawn as a closed set of controls without
   * checking.
   */
  options: AskedOption[];
};

/**
 * One answer a drone said it would accept. `crates/ipc/src/detail.rs`.
 *
 * **Two fields, because a label alone is not a decision.** The label is what a
 * control says and the consequence is what pressing it commits to, which is the
 * briefing register applied to the smallest surface there is.
 */
export type AskedOption = {
  /** What a person picks. **The answer's own name** — an answer names this. */
  label: string;
  /** What the drone will do if it is picked. Never blank. */
  consequence: string;
};

/**
 * A person's answer to one question. The request half of `answer_question`.
 * `crates/ipc/src/detail.rs`.
 *
 * **It carries no prose and there is no field for any.** An answer fleet cannot
 * match to one of the offered labels is refused rather than passed through.
 */
export type ChosenAnswer = {
  question_id: string;
  /** The `AskedOption.label` chosen, verbatim. */
  chose: string;
};
