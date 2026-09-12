// What a proposal waits for, and what a failure on that seam carries.
//
// **Against a real listener, because the wait is the subject.** `ask` builds
// one `fetch`, and a fake that resolves on the next tick proves nothing about
// a budget — the defect being held down here is that this route had a
// five-second one while Fleet's own bound on the proposer's model call is two
// minutes, so every proposal a person made aborted on this side and all they
// saw was `Fleet did not answer: The operation was aborted due to timeout`.
//
// The route now has no Bridge-side wait at all. What bounds it is Fleet's
// budget and a person pressing stop, which is why the first case here is worth
// keeping: it fails against any version that reintroduces a guess.
//
// The second subject is what that sentence had under it, which was nothing. A
// transport failure carries no `WireError`, so what a person quoting one has is
// whatever `TransportFault` holds: which of the three it was and the route.
// These assert on that record rather than on any rendering of it.

import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { JobSummary, StagedAttachment } from "@armada/protocol";
import type { Board } from "./command";
import { proposeFromRequest } from "./proposing";

/** How long a case may hold a request before it is the test that hung. */
const A_LONG_WAIT = 20_000;

const A_JOB: JobSummary = {
  id: "01M1HQZAKN001AJ5MT3PT09KKY",
  handle: "12-a-job",
  title: "Make the parser take it",
  status: "awaiting_approval",
  workflow_id: "bug",
  owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
  origin: "proposed",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  created_at: "2026-09-02T19:00:00Z",
};

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

/** One listener answering `/jobs/from_request` however the case says. */
async function fleetThat(answer: (respond: Respond) => void): Promise<number> {
  const server = createServer((_, response) => answer(response));
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

type Respond = ServerResponse<IncomingMessage>;

/** The board a proposal folds into. Nothing here is under test. */
function boardOn(port: number): Board {
  return {
    port: () => port,
    fold: () => {},
    forget: () => {},
    reread: async () => {},
    refresh: () => {},
    publish: () => {},
    watchProposal: () => {},
    proposalOut: () => null,
  };
}

/**
 * The bug, held down. A proposer answering after six seconds is a proposer
 * answering — six seconds is unremarkable for a model call, and Fleet is
 * allowed two minutes of them.
 *
 * Bridge now waits without a budget of its own: what bounds the call is Fleet's
 * budget at one end and a person pressing stop at the other. Six seconds is
 * chosen because it outran the wait this route used to have, so the case still
 * fails against the version that had one.
 */
it(
  "waits out a proposer that takes longer than an ordinary command",
  async () => {
    const port = await fleetThat((response) => {
      setTimeout(() => {
        response.writeHead(201, { "content-type": "application/json" });
        response.end(JSON.stringify({ jobs: [A_JOB] }));
      }, 6000);
    });

    const answered = await proposeFromRequest(boardOn(port), "Make the parser take it");

    expect(answered.ok).toBe(true);
    expect(answered.ok === true && answered.jobs.map((job) => job.id)).toEqual([A_JOB.id]);
  },
  A_LONG_WAIT,
);

/**
 * A socket that fails names the route it failed on. **`unreachable` and not
 * `timed_out`**: nothing waited, and the two take different next steps — a
 * timeout may have been carried out and this was never read.
 */
it("names the route when the connection fails", async () => {
  const port = await fleetThat((response) => response.destroy());

  const answered = await proposeFromRequest(boardOn(port), "Make the parser take it");

  expect(answered.ok).toBe(false);
  if (answered.ok) return;
  expect(answered.why).toBe("faulted");
  const outcome = answered.outcome;
  expect(outcome.ok === false && outcome.why).toBe("transport");
  if (outcome.ok || outcome.why !== "transport") return;
  expect(outcome.fault).toEqual({
    why: "unreachable",
    method: "POST",
    path: "/jobs/from_request",
  });
  // The machine's own words survive beside the record rather than instead of
  // it: they are what a reader greps a log for.
  expect(outcome.detail).not.toBe("");
});

/**
 * Fleet answering a status with a body that is not a refusal is the two sides
 * disagreeing about the route, and it carries the status that says so.
 */
it("carries the status when the answer is not a refusal", async () => {
  const port = await fleetThat((response) => {
    response.writeHead(502, { "content-type": "text/plain" });
    response.end("upstream said no");
  });

  const answered = await proposeFromRequest(boardOn(port), "Make the parser take it");

  expect(answered.ok).toBe(false);
  if (answered.ok) return;
  const outcome = answered.outcome;
  if (outcome.ok || outcome.why !== "transport") throw new Error("expected a transport failure");
  expect(outcome.fault).toEqual({
    why: "unanswerable",
    method: "POST",
    path: "/jobs/from_request",
    status: 502,
  });
});

/** A listener that answers every proposal and records the body it carried. */
async function fleetRecording(bodies: string[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    let body = "";
    request.on("data", (chunk) => (body += String(chunk)));
    request.on("end", () => {
      bodies.push(body);
      response.writeHead(201, { "content-type": "application/json" });
      response.end(JSON.stringify({ jobs: [A_JOB] }));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

/**
 * A staged screenshot rides the request, renamed onto Fleet's own field names
 * — `ipc::JobRequest::attachments` takes `staged_path`, `filename` and
 * `mime_type`, not the camelCase `StagedAttachment` a picker or a paste built.
 */
it("carries a staged attachment on the request, renamed onto Fleet's fields", async () => {
  const bodies: string[] = [];
  const port = await fleetRecording(bodies);
  const attachment: StagedAttachment = {
    path: "/tmp/staged/before.png",
    filename: "before.png",
    mimeType: "image/png",
  };

  const answered = await proposeFromRequest(boardOn(port), "Fix the flicker", [attachment]);

  expect(answered.ok).toBe(true);
  const sent = JSON.parse(bodies[0] ?? "null") as { attachments: unknown };
  expect(sent.attachments).toEqual([
    { staged_path: "/tmp/staged/before.png", filename: "before.png", mime_type: "image/png" },
  ]);
});

/**
 * Nothing staged is sent as an empty list rather than left off — `proposing.ts`'s
 * own note says `ProposeJob.attachments` has no meaningful absent-vs-empty
 * reading for Fleet to fill in.
 */
it("sends an empty attachments list where nothing was staged", async () => {
  const bodies: string[] = [];
  const port = await fleetRecording(bodies);

  await proposeFromRequest(boardOn(port), "Make the parser take it");

  const sent = JSON.parse(bodies[0] ?? "null") as { attachments: unknown };
  expect(sent.attachments).toEqual([]);
});
