// A question a Job is holding open for a person. Its own file, without JSX, because main publishes
// it on `BridgeState` and main's build reads no `.tsx`.

import type { CommandInFlight, HelmCallInFlight, JudgeQuestion, QuestionInFlight } from "@armada/protocol";

/**
 * One question a Job is holding open for a person, of the three kinds Fleet asks. **Wire shapes
 * whole**, so #936 answers from what was asked rather than from what a card drew.
 */
export type Outstanding =
  | { kind: "drone"; job_id: string; asking: QuestionInFlight }
  | { kind: "command"; job_id: string; waiting: CommandInFlight }
  | { kind: "judge"; job_id: string; question: JudgeQuestion }
  /**
   * A call helm made that the person's own settings do not cover, held open
   * while they answer. **No `job_id`, and that is the difference** — nothing is
   * queued and no step is running while it waits, so the card names the
   * repository where the others name a job. #1389.
   */
  | { kind: "helm"; call: HelmCallInFlight };

/** Which job a question stands on, where it stands on one. A helm call stands on none. */
export function questionsJob(question: Outstanding): string | undefined {
  return question.kind === "helm" ? undefined : question.job_id;
}

/** Unique across the list: a Job can hold a question and a command at once. #936's wiring keys on it too. */
export function outstandingId(question: Outstanding): string {
  return question.kind === "helm" ? `helm:${question.call.call}` : `${question.job_id}:${question.kind}`;
}
