// A Judge criterion that refused and a person is being asked about, rather
// than the step stopping over it. `crates/ipc/src/asking.rs`. Since protocol
// 11.1.
//
// A refusal on a criterion marked `refuse` still stops the step exactly as
// every refusal did before this existed; the rest hold it open at a question.
// The step is `awaiting_human` and the job `awaiting_review` while it waits —
// the same pair a `human_always` review gate already reaches — so this rides
// beside them rather than adding a new status either registry would grow.

/**
 * The question a person is being asked, right now. Since protocol 11.1.
 */
export type JudgeQuestion = {
  /** Which step this question is about. A job runs one step at a time. */
  step_id: string;
  /** The refused criterion. The join to `StepDetail.judged`. */
  criterion_id: string;
  /** The plain question the criterion asked. */
  question: string;
  /** What should be seen, returned or recorded if the work is right. */
  expected: string;
  /** What will be seen instead. */
  produced: string;
  /** What that difference does to whoever consumes it — the field to triage on. */
  consequence: string;
  /** When the refusal was raised, by fleet's clock. */
  asked_at: string;
  /** Where the whole brief this verdict answers was written. Absent where it was not kept. */
  brief_path?: string;
};

/**
 * A person's answer to a `JudgeQuestion`. Since protocol 11.1.
 *
 * `agree` fails the step exactly as it would where the criterion is marked
 * `refuse`. `disagree_once` advances the step, and the next job's gate asks
 * about the same criterion again. `disagree_always` advances it and also
 * stands the criterion down for the repository, so no later job is asked
 * about it either.
 */
export type JudgeAnswer = "agree" | "disagree_once" | "disagree_always";

/**
 * The body of `answer_judge`. Since protocol 11.1. `note` is never required.
 */
export type JudgeAnswered = {
  answer: JudgeAnswer;
  note?: string;
};
