// What a person's own add and drop put on the wire, read off a real listener —
// `command.test.ts`'s own shape. `#897`.

import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { WorkPlan } from "@armada/protocol";
import { PlanEdits, type PlanBoard } from "./plan-edits";

const A_PLAN: WorkPlan = {
  approach: "Split the module.",
  recorded_by: { by: "step", step_id: "fix", attempt: 1 },
  recorded_at: "2026-09-13T14:00:00Z",
  tasks: [{ id: "T1", title: "Extract the selector", state: "open" }],
};
const JOB_ID = "01M1HQZAKN001AJ5MT3PT09KKY";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

/** What one request carried: its path, and its body, parsed. */
type Asked = { path: string; body: unknown };

/** A listener that answers every request with `A_PLAN` and records what arrived. */
async function fleetRecording(into: Asked[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    let body = "";
    request.on("data", (chunk) => (body += String(chunk)));
    request.on("end", () => {
      into.push({ path: request.url ?? "", body: body === "" ? null : JSON.parse(body) });
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(A_PLAN));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

/** The board an edit folds into, and what it folded. Nothing else here is under test. */
function boardOn(port: number): { board: PlanBoard; folded: { jobId: string; plan: WorkPlan }[] } {
  const folded: { jobId: string; plan: WorkPlan }[] = [];
  return {
    board: {
      port: () => port,
      foldPlan: (jobId, plan) => folded.push({ jobId, plan }),
    },
    folded,
  };
}

it("reaches add_task with the title, the detail and where it goes", async () => {
  const asked: Asked[] = [];
  const { board, folded } = boardOn(await fleetRecording(asked));
  const edits = new PlanEdits(board);

  const answer = await edits.add(JOB_ID, { title: "Add a regression test", note: "", scope: [], expects: "", after: "" });

  expect(answer.ok).toBe(true);
  expect(asked).toHaveLength(1);
  expect(asked[0]?.path).toBe(`/jobs/${JOB_ID}/add_task`);
  expect(asked[0]?.body).toEqual({ title: "Add a regression test", note: "", scope: [], expects: "", after: "" });
  // The plan the answer carries is folded straight in — no second read.
  expect(folded).toEqual([{ jobId: JOB_ID, plan: A_PLAN }]);
});

it("reaches drop_task with the task id and the reason", async () => {
  const asked: Asked[] = [];
  const { board, folded } = boardOn(await fleetRecording(asked));
  const edits = new PlanEdits(board);

  const answer = await edits.drop(JOB_ID, { task: "T1", reason: "Already covered elsewhere." });

  expect(answer.ok).toBe(true);
  expect(asked[0]?.path).toBe(`/jobs/${JOB_ID}/drop_task`);
  expect(asked[0]?.body).toEqual({ task: "T1", reason: "Already covered elsewhere." });
  expect(folded).toEqual([{ jobId: JOB_ID, plan: A_PLAN }]);
});

it("refuses a blank title before anything is sent", async () => {
  const asked: Asked[] = [];
  const { board } = boardOn(await fleetRecording(asked));
  const edits = new PlanEdits(board);

  const answer = await edits.add(JOB_ID, { title: "   ", note: "", scope: [], expects: "", after: "" });

  expect(answer).toEqual({ ok: false, outcome: { ok: false, why: "empty_task_title" } });
  expect(asked).toHaveLength(0);
});

it("refuses a blank reason before anything is sent", async () => {
  const asked: Asked[] = [];
  const { board } = boardOn(await fleetRecording(asked));
  const edits = new PlanEdits(board);

  const answer = await edits.drop(JOB_ID, { task: "T1", reason: "  " });

  expect(answer).toEqual({ ok: false, outcome: { ok: false, why: "empty_task_reason" } });
  expect(asked).toHaveLength(0);
});
