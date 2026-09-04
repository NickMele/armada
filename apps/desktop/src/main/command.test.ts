// What a restart puts on the wire, read off a real listener.
//
// **The subject is the body and nothing else.** `restart_step` took no body at
// all until #396, and the whole promise of the change is that restarting with
// nothing to say still sends none — so what is asserted here is bytes, not an
// outcome. A version that sent `{"note":""}` for an empty field would look
// identical from the renderer and would earn a 422 from fleet.
//
// A listener rather than a stubbed `fetch`, for `proposing.test.ts`'s reason:
// what is under test is what crosses, and a fake that records its arguments
// records what this side believed rather than what was sent.

import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { JobSummary } from "@armada/protocol";
import { JobCommands, type Board } from "./command";

const A_JOB: JobSummary = {
  id: "01M1HQZAKN001AJ5MT3PT09KKY",
  title: "Make the parser take it",
  status: "queued",
  workflow_id: "bug",
  owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
  origin: "dispatched",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  created_at: "2026-09-03T19:00:00Z",
};

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

/** What one request carried: its path, and its body as bytes. */
type Asked = { path: string; body: string };

/** A listener that answers every command with a Job and records what arrived. */
async function fleetRecording(into: Asked[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    let body = "";
    request.on("data", (chunk) => (body += String(chunk)));
    request.on("end", () => {
      into.push({ path: request.url ?? "", body });
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(A_JOB));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

/** The board an act folds into. Nothing here is under test. */
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
 * **The promise the whole change rests on.** Restarting with nothing to say is
 * exactly the request it was, so a fleet built before #396 answers it and a
 * person who never opens the field pays nothing for one existing.
 */
it("sends no body at all when a restart carries no note", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.restartStep(A_JOB.id);

  expect(answer.ok).toBe(true);
  expect(asked).toHaveLength(1);
  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/restart_step`);
  expect(asked[0]?.body).toBe("");
});

/** The words go over verbatim, on the route the restart already used. */
it("sends the note as the body when a restart carries one", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.restartStep(A_JOB.id, "Delete that test, it tests the old behaviour.");

  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/restart_step`);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    note: "Delete that test, it tests the old behaviour.",
  });
});

/**
 * A field somebody opened and typed nothing into is a restart with no note, and
 * **this is the one place it differs from `redirectDrone`.** There the note is
 * the act and blank is refused before anything is sent; here the act is the
 * restart, so the restart happens and fleet is never handed the 422 a blank
 * note earns.
 */
it("drops a blank note rather than refusing the restart", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.restartStep(A_JOB.id, "   \n  ");

  expect(answer.ok).toBe(true);
  expect(asked[0]?.body).toBe("");
});

/** Surrounding whitespace is not part of what a person said. */
it("trims what it sends", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.restartStep(A_JOB.id, "  start from the failing case  ");

  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({ note: "start from the failing case" });
});
