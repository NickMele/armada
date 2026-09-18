// Every question waiting on a person, held against a Fleet that serves each Job's detail: folded
// off the events that carry one, read where none does, and cleared when the Job moves on.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import type { JobDetail, JobSummary, JudgeQuestion, QuestionInFlight } from "@armada/protocol";
import { NOTHING_YET, type BridgeState } from "../shared/bridge";
import { questionsJob } from "@armada/screens/src/outstanding";
import { Questions } from "./questions";

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "a",
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
  options: [
    { label: "This", consequence: "Does this." },
    { label: "That", consequence: "Does that." },
  ],
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

let listening: Server | null = null;
afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise<void>((done) => server.close(() => done()));
});

/** A Fleet answering `/jobs/:id` from `details`, recording each route read and able to hold one. */
async function fleet(details: Record<string, Partial<JobDetail>>, asked: string[], hold?: Promise<void>) {
  const server = createServer((request, response) => {
    const url = request.url ?? "";
    asked.push(url);
    const id = decodeURIComponent(url.replace(/^\/jobs\//, ""));
    const answer = () => {
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(details[id] ?? {}));
    };
    if (hold === undefined) answer();
    else void hold.then(answer);
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  return (server.address() as AddressInfo).port;
}

function holding(jobs: JobSummary[]) {
  let state: BridgeState = { ...NOTHING_YET, jobs };
  const questions = new Questions({
    current: () => state,
    publish: (change) => (state = { ...state, ...change }),
  });
  return { questions, state: () => state };
}

describe("questions waiting on a person", () => {
  it("fold a Drone's question and a held command off their events, and clear each on its answer", () => {
    const { questions, state } = holding([job()]);
    questions.asking("a", ASKING);
    questions.commandWaiting("a", {
      call: "c1", step_id: "implement", asked_at: "2026-09-13T10:02:00Z", tool: "Bash",
      detail: "ls", truncated: false, offers: ["reject"], rules: [],
    });
    expect(state().questions.map((question) => question.kind)).toEqual(["drone", "command"]);
    questions.asking("a", undefined);
    expect(state().questions.map((question) => question.kind)).toEqual(["command"]);
  });

  it("drop a question when its Job moves off the status or step that holds it", () => {
    const { questions, state } = holding([job()]);
    questions.asking("a", ASKING);
    questions.moved(job({ current_step_id: "implement" }));
    expect(state().questions).toHaveLength(1);
    questions.moved(job({ current_step_id: "review" }));
    expect(state().questions).toEqual([]);

    state().questions.push({ kind: "judge", job_id: "a", question: JUDGED });
    questions.moved(job({ status: "awaiting_review" }));
    expect(state().questions).toHaveLength(1);
    questions.moved(job({ status: "queued" }));
    expect(state().questions).toEqual([]);
  });

  it("read a Judge refusal off the Job's detail, which is the only place it is served", async () => {
    const asked: string[] = [];
    const port = await fleet({ a: { judge_question: JUDGED } }, asked);
    const { questions, state } = holding([job({ status: "awaiting_review" })]);
    await questions.read(port, "a");
    expect(state().questions).toEqual([{ kind: "judge", job_id: "a", question: JUDGED }]);
  });

  it("on a resync, read only the Jobs that can be holding one, and every repository's", async () => {
    const asked: string[] = [];
    const port = await fleet({ a: { asking: ASKING }, b: { judge_question: JUDGED } }, asked);
    const { questions, state } = holding([
      job({ id: "a", asking: true }),
      job({ id: "b", status: "awaiting_review", owner_manifest_id: "shop-01" }),
      job({ id: "c" }),
      job({ id: "d", status: "completed" }),
    ]);
    await questions.readAll(port);
    // `/helm/calls` beside them: a helm call is on no board, so a resync reads it from its own
    // route rather than from any job. #1389.
    expect(asked.sort()).toEqual(["/helm/calls", "/jobs/a", "/jobs/b"]);
    expect(state().questions.map((question) => [questionsJob(question), question.kind]).sort()).toEqual([
      ["a", "drone"],
      ["b", "judge"],
    ]);
  });

  it("never put back a question answered while its read was out", async () => {
    const asked: string[] = [];
    let release = (): void => {};
    const held = new Promise<void>((done) => (release = done));
    const port = await fleet({ a: { asking: ASKING } }, asked, held);
    const { questions, state } = holding([job({ asking: true })]);
    const reading = questions.read(port, "a");
    await new Promise((done) => setTimeout(done, 20));
    questions.asking("a", undefined);
    release();
    await reading;
    expect(state().questions).toEqual([]);
  });

  it("drop a forgotten Job's questions, and a held one whose Job the Board no longer lists", async () => {
    const { questions, state } = holding([job()]);
    questions.asking("a", ASKING);
    questions.forgotten("a");
    expect(state().questions).toEqual([]);

    questions.asking("a", ASKING);
    state().jobs = [];
    await questions.readAll(1);
    expect(state().questions).toEqual([]);
  });
});

// #1389: a helm call has no job, so none of the machinery above reaches it — its own event puts it
// up, its own event takes it down, and a resync reads it from its own route.
describe("a helm call waiting on a person", () => {
  const WAITING = {
    call: "helm-1",
    manifest_id: "armada",
    asked_at: "2026-09-13T10:02:00Z",
    tool: "Bash",
    detail: "gh issue list",
    truncated: false,
    rule: "Bash(gh issue list:*)",
    offers: ["allow_once", "allow_and_remember", "refuse"] as const,
    holding_for_seconds: 300,
  };

  it("goes up on its ask and comes down on its answer", () => {
    const { questions, state } = holding([]);
    questions.helmAsking({ ...WAITING, offers: [...WAITING.offers] });
    expect(state().questions.map((question) => question.kind)).toEqual(["helm"]);

    questions.helmAnswered("helm-1");
    expect(state().questions).toEqual([]);
  });

  it("is drawn once when its ask arrives twice", () => {
    const { questions, state } = holding([]);
    questions.helmAsking({ ...WAITING, offers: [...WAITING.offers] });
    questions.helmAsking({ ...WAITING, offers: [...WAITING.offers] });
    expect(state().questions).toHaveLength(1);
  });

  // A board that holds no job takes every job's question down. A helm call is on no board, so it
  // survives what would clear one.
  it("survives a board that lists nothing", async () => {
    const { questions, state } = holding([]);
    questions.helmAsking({ ...WAITING, offers: [...WAITING.offers] });
    await questions.readAll(1);
    expect(state().questions.map((question) => question.kind)).toEqual(["helm"]);
  });
});
