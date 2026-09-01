// What is outstanding on a live drone, and what a person sends it back.
// `crates/ipc/src/waiting.rs`.
//
// **Split out of `protocol.ts` for `events.ts`'s reason and by the same rule.**
// That file reached the 900 lines the gate refuses again. The cut is one subject
// and the Rust side makes it too: a redirect in flight and a question in flight
// are the same shape — a fact about a drone that is alive and not moving, read
// off a working slot rather than off the record, gone the instant the thing it
// names is answered. Both exist because "nothing is happening" and "something is
// happening this seam could not state" were the same pixels.
//
// `RedirectWaiting` is the third because it is the same act with no session to
// go into, and `AskedOption` and `ChosenAnswer` are the two halves of answering.
//
// The header rules in `protocol.ts` hold here: hand-written, they drift the day
// a field moves, and every closed set is left as `string`.

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
