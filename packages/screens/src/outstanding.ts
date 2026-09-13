// A question a Job is holding open for a person. Its own file, without JSX, because main publishes
// it on `BridgeState` and main's build reads no `.tsx`.

import type { CommandInFlight, JudgeQuestion, QuestionInFlight } from "@armada/protocol";

/**
 * One question a Job is holding open for a person, of the three kinds Fleet asks. **Wire shapes
 * whole**, so #936 answers from what was asked rather than from what a card drew.
 */
export type Outstanding =
  | { kind: "drone"; job_id: string; asking: QuestionInFlight }
  | { kind: "command"; job_id: string; waiting: CommandInFlight }
  | { kind: "judge"; job_id: string; question: JudgeQuestion };

/** Unique across the list: a Job can hold a question and a command at once. #936's wiring keys on it too. */
export function outstandingId(question: Outstanding): string {
  return `${question.job_id}:${question.kind}`;
}
