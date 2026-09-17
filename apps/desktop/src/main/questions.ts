// Every question waiting on a person, from every repository Fleet serves — #935.
//
// **Not scoped to the rail's pick, and not to the open Job.** A question is one Job's, but the dock
// lists every Job's on every surface, so this holds them apart from `watched`.
//
// **Folded where the event carries the question, read where it does not.** `job.asking` and
// `job.command_waiting` carry theirs whole, and their absence is the answer landing. A Judge
// refusal is carried by nothing on `/events`: `job.judging` names the call, not the question, so
// a Job moving into `awaiting_review` has its detail read — one read per such move.
//
// **A resync reads every Job that can be holding one**: `JobSummary.asking`, which covers a held
// command too, and every `awaiting_review` Job. Events arrive only for what happens next, so a
// Bridge started while a Drone waits would otherwise list nothing. Bounded by the Jobs waiting on
// a person, which is the list this draws.

import type {
  CommandInFlight,
  HelmCallInFlight,
  HelmCallsWaiting,
  JobDetail,
  JobSummary,
  QuestionInFlight,
} from "@armada/protocol";
import { questionsJob, type Outstanding } from "@armada/screens/src/outstanding";
import type { BridgeState } from "../shared/bridge";
import { ask } from "./request";

export type QuestionsWiring = {
  current: () => BridgeState;
  publish: (change: Partial<BridgeState>) => void;
};

export class Questions {
  private readonly wiring: QuestionsWiring;
  /**
   * Bumped by every event about a Job. **A read publishes only where nothing was heard about its
   * Job while it was out**, so a detail taken before an answer landed cannot put the answered
   * question back.
   */
  private readonly heard = new Map<string, number>();

  constructor(wiring: QuestionsWiring) {
    this.wiring = wiring;
  }

  /** `job.asking`. Absent is the question answered. */
  asking(jobId: string, asking: QuestionInFlight | undefined): void {
    this.replace(jobId, ["drone"], asking === undefined ? [] : [{ kind: "drone", job_id: jobId, asking }]);
  }

  /** `job.command_waiting`. Absent is the command answered. */
  commandWaiting(jobId: string, waiting: CommandInFlight | undefined): void {
    this.replace(jobId, ["command"], waiting === undefined ? [] : [{ kind: "command", job_id: jobId, waiting }]);
  }

  /**
   * `helm.asking_to_run`. **Replaces the ask under the same call id**, so a
   * re-delivered event draws one card and not two. #1389.
   */
  helmAsking(waiting: HelmCallInFlight): void {
    const held = this.wiring.current().questions;
    const others = held.filter((question) => question.kind !== "helm" || question.call.call !== waiting.call);
    this.wiring.publish({ questions: [...others, { kind: "helm", call: waiting }] });
  }

  /** `helm.call_answered`, and the answer a person's own press came back with. */
  helmAnswered(call: string): void {
    const held = this.wiring.current().questions;
    const kept = held.filter((question) => question.kind !== "helm" || question.call.call !== call);
    if (kept.length !== held.length) this.wiring.publish({ questions: kept });
  }

  /**
   * Every helm call waiting, on a resync. **Read rather than folded**: the
   * events arrive only for what happens next, so a Bridge started while one was
   * already waiting would otherwise list nothing. A failed read keeps what is
   * held, `read`'s rule.
   */
  async readHelmCalls(port: number): Promise<void> {
    const answer = await ask(port, "GET", "/helm/calls");
    if (answer.ok !== true) return;
    const { waiting } = answer.body as HelmCallsWaiting;
    const others = this.wiring.current().questions.filter((question) => question.kind !== "helm");
    this.wiring.publish({
      questions: [...others, ...waiting.map((call) => ({ kind: "helm" as const, call }))],
    });
  }

  /** A Job's row moved. What its status or step can no longer be holding leaves. */
  moved(job: JobSummary): void {
    this.bump(job.id);
    const held = this.wiring.current().questions;
    const kept = held.filter(
      (question) => questionsJob(question) !== job.id || standsOn(question, job),
    );
    if (kept.length !== held.length) this.wiring.publish({ questions: kept });
  }

  /** `job.forgotten`: nothing of the Job is left to answer. */
  forgotten(jobId: string): void {
    this.replace(jobId, ["drone", "command", "judge"], []);
  }

  /** What one Job holds open, from its detail. A failed read keeps what is held. */
  async read(port: number, jobId: string): Promise<void> {
    const from = this.heard.get(jobId) ?? 0;
    const answer = await ask(port, "GET", `/jobs/${encodeURIComponent(jobId)}`);
    if (answer.ok !== true || (this.heard.get(jobId) ?? 0) !== from) return;
    const others = this.wiring.current().questions.filter((question) => questionsJob(question) !== jobId);
    this.wiring.publish({ questions: [...others, ...outstandingOf(jobId, answer.body as JobDetail)] });
  }

  /** Every Job that can be holding one, on a resync or a Refresh. Jobs no longer listed drop theirs. */
  async readAll(port: number): Promise<void> {
    const { jobs, questions } = this.wiring.current();
    const listed = new Set(jobs.map((job) => job.id));
    // A helm call is on no board, so nothing here can drop it; `readHelmCalls`
    // is its own resync.
    const kept = questions.filter((question) => {
      const jobId = questionsJob(question);
      return jobId === undefined || listed.has(jobId);
    });
    if (kept.length !== questions.length) this.wiring.publish({ questions: kept });
    const wanted = new Set([
      ...jobs.filter((job) => job.asking === true || job.status === "awaiting_review").map((job) => job.id),
      ...kept.flatMap((question) => [questionsJob(question)].filter((id) => id !== undefined)),
    ]);
    await Promise.all([...wanted].map((jobId) => this.read(port, jobId)));
    await this.readHelmCalls(port);
  }

  private replace(jobId: string, kinds: Outstanding["kind"][], entries: Outstanding[]): void {
    this.bump(jobId);
    const others = this.wiring
      .current()
      .questions.filter((question) => questionsJob(question) !== jobId || !kinds.includes(question.kind));
    this.wiring.publish({ questions: [...others, ...entries] });
  }

  private bump(jobId: string): void {
    this.heard.set(jobId, (this.heard.get(jobId) ?? 0) + 1);
  }
}

/** The three a Job's detail can carry. */
export function outstandingOf(jobId: string, detail: JobDetail): Outstanding[] {
  return [
    ...(detail.asking === undefined ? [] : [{ kind: "drone" as const, job_id: jobId, asking: detail.asking }]),
    ...(detail.command_waiting === undefined
      ? []
      : [{ kind: "command" as const, job_id: jobId, waiting: detail.command_waiting }]),
    ...(detail.judge_question === undefined
      ? []
      : [{ kind: "judge" as const, job_id: jobId, question: detail.judge_question }]),
  ];
}

/**
 * Whether a question can still stand on a Job whose row reads so. A Drone's question and a held
 * command leave the Job `running` on the step that asked; a Judge refusal holds it at
 * `awaiting_review`, and any move off either is the question gone.
 */
export function standsOn(question: Outstanding, job: JobSummary): boolean {
  // Never reached: a helm call's job is `undefined`, so the filter above never
  // asks this about one. Answered rather than thrown, for a caller that does.
  if (question.kind === "helm") return true;
  if (question.kind === "judge") return job.status === "awaiting_review";
  const step = question.kind === "drone" ? question.asking.step_id : question.waiting.step_id;
  return job.status === "running" && (job.current_step_id === undefined || job.current_step_id === step);
}
