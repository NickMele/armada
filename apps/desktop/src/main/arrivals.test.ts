// Whether the dock's own copy of a question and the open Job's detail move together, off one
// event on `/events` — #937. `questions.ts` folds the dock's copy; `refresh` re-reads the open
// Job's detail, `job-focus.ts`'s own province. What this file answers is whether `arrivals.ts`
// calls both from the same arm — not what either does once called, which `questions.test.ts` and
// `job-focus.ts`'s own tests already cover.
//
// A real `Questions` proves the dock's half; `refresh` is a spy standing for the detail's, because
// what re-reads it lives behind Electron's IPC and a socket this file has no reason to open.

import { describe, expect, it, vi } from "vitest";

import { PROTOCOL_VERSION } from "@armada/protocol";
import type { CommandInFlight, JobSummary, JudgeQuestion, QuestionInFlight } from "@armada/protocol";
import { NOTHING_YET, type BridgeState } from "../shared/bridge";
import { applyArrival, type ArrivalHost } from "./arrivals";
import type { RehearsalConnection } from "./rehearsal";
import type { RepositoryReads } from "./repositories";
import { Questions } from "./questions";
import type { ReviewMaterial } from "./review";

const JOB_ID = "01M1HQZAKN001AJ5MT3PT09KKY";
const FLEET = { protocolVersion: PROTOCOL_VERSION, pid: 1, port: 1, startedAt: "" };

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: JOB_ID,
    handle: "12-a-job",
    title: "A job",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "armada",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-13T09:00:00Z",
    current_step_id: "implement",
    ...over,
  };
}

const ASKING: QuestionInFlight = {
  question_id: "q1",
  step_id: "implement",
  asked_at: "2026-09-13T10:00:00Z",
  question: "Which?",
  options: [{ label: "This", consequence: "Does this." }],
};

const JUDGED: JudgeQuestion = {
  step_id: "review",
  criterion_id: "cause",
  question: "Cause fixed?",
  expected: "Fixed.",
  produced: "Hidden.",
  consequence: "Still broken.",
  asked_at: "2026-09-13T10:01:00Z",
};

/**
 * A host reaching only what these arms touch: `questions`, real off its own wiring so the dock's
 * clearing is genuine rather than asserted by a spy; `refresh`, a spy standing in for the open
 * Job's detail. Every other region is never called by `job.asking`, `job.command_waiting` or
 * `job.state_changed`, so a cast stands in rather than a working fake of five other classes.
 */
function fakeHost(
  jobs: JobSummary[],
  questionsOver: BridgeState["questions"] = [],
): { host: ArrivalHost; refresh: ReturnType<typeof vi.fn>; questions: Questions; state: () => BridgeState } {
  let state: BridgeState = { ...NOTHING_YET, jobs, questions: questionsOver };
  const refresh = vi.fn();
  const questions = new Questions({
    current: () => state,
    publish: (change) => (state = { ...state, ...change }),
  });
  const host: ArrivalHost = {
    current: () => state,
    now: () => 0,
    greeted: () => true,
    setGreeted: () => {},
    proposalRef: () => null,
    setProposalRef: () => {},
    watchedJobId: () => null,
    repositories: {} as unknown as RepositoryReads,
    rehearsal: {} as unknown as RehearsalConnection,
    overviewAgain: async () => {},
    questions,
    helm: { reconnected: () => {} },
    material: {} as unknown as ReviewMaterial,
    socket: { close: () => {}, resetUnreachable: () => {} },
    publish: (change) => (state = { ...state, ...change }),
    fold: (row) => (state = { ...state, jobs: state.jobs.map((one) => (one.id === row.id ? row : one)) }),
    forget: () => {},
    settle: () => {},
    refresh,
    takeAgain: async () => {},
  };
  return { host, refresh, questions, state: () => state };
}

describe("a Drone's question, on the dock and on the open Job's detail together", () => {
  it("arrives on both from the one event", () => {
    const { host, refresh, state } = fakeHost([job()]);
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.asking", job_id: JOB_ID, step_id: "implement", asking: ASKING, actor: "drone", at: ASKING.asked_at },
      }),
      FLEET,
    );
    expect(state().questions).toEqual([{ kind: "drone", job_id: JOB_ID, asking: ASKING }]);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });

  it("leaves both on the answer, whichever surface sent it", () => {
    const { host, refresh, state } = fakeHost([job()], [{ kind: "drone", job_id: JOB_ID, asking: ASKING }]);
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.asking", job_id: JOB_ID, step_id: "implement", actor: "human", at: "2026-09-13T10:05:00Z" },
      }),
      FLEET,
    );
    expect(state().questions).toEqual([]);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });
});

describe("a held command, the same pairing", () => {
  const WAITING: CommandInFlight = {
    call: "call_1",
    step_id: "implement",
    asked_at: "2026-09-13T10:02:00Z",
    tool: "Bash",
    detail: "ls",
    truncated: false,
    offers: ["reject"],
    rules: [],
  };

  it("arrives on both from the one event", () => {
    const { host, refresh, state } = fakeHost([job()]);
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.command_waiting", job_id: JOB_ID, step_id: "implement", waiting: WAITING, actor: "drone", at: WAITING.asked_at },
      }),
      FLEET,
    );
    expect(state().questions).toEqual([{ kind: "command", job_id: JOB_ID, waiting: WAITING }]);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });

  it("leaves both on the answer", () => {
    const { host, refresh, state } = fakeHost([job()], [{ kind: "command", job_id: JOB_ID, waiting: WAITING }]);
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.command_waiting", job_id: JOB_ID, step_id: "implement", actor: "human", at: "2026-09-13T10:06:00Z" },
      }),
      FLEET,
    );
    expect(state().questions).toEqual([]);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });
});

describe("a Judge refusal, carried by no event, on the status move that opens or closes it", () => {
  it("entering awaiting_review wakes the dock's own read and re-reads the open Job together", () => {
    const { host, refresh, questions } = fakeHost([job({ status: "running" })]);
    const read = vi.spyOn(questions, "read").mockResolvedValue(undefined);
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.state_changed", job_id: JOB_ID, from: "running", to: "awaiting_review", actor: "fleet", at: "2026-09-13T10:03:00Z" },
      }),
      FLEET,
    );
    expect(read).toHaveBeenCalledWith(FLEET.port, JOB_ID);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });

  it("leaving awaiting_review drops the dock's copy and re-reads the open Job together, however it was answered", () => {
    const { host, refresh, state } = fakeHost(
      [job({ status: "awaiting_review" })],
      [{ kind: "judge", job_id: JOB_ID, question: JUDGED }],
    );
    applyArrival(
      host,
      JSON.stringify({
        message: "event",
        cursor: 2,
        event: { kind: "job.state_changed", job_id: JOB_ID, from: "awaiting_review", to: "queued", actor: "fleet", at: "2026-09-13T10:07:00Z" },
      }),
      FLEET,
    );
    expect(state().questions).toEqual([]);
    expect(refresh).toHaveBeenCalledWith(FLEET.port, JOB_ID);
  });
});
