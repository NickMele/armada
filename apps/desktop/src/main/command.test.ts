// What a restart and a raise put on the wire, read off a real listener.
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
//
// **The raise is here for the same reason at a different scale.** It carries a
// figure rather than a sentence, in millionths of a dollar where a person typed
// dollars, and neither side would notice a factor of a million: what it puts on
// the wire is bytes, and bytes are what is asserted.

import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { JobSummary } from "@armada/protocol";
import { JobCommands, type Board } from "./command";

const A_JOB: JobSummary = {
  id: "01M1HQZAKN001AJ5MT3PT09KKY",
  handle: "12-a-job",
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

/**
 * **The unit crosses as micros.** The renderer hands over millionths of a
 * dollar because `JobSpend` reads in them, and the body carries that integer
 * unchanged — a raise sent in dollars would be a cap below a hundredth of a
 * cent, refused as no raise at all.
 */
it("sends the new cost cap in micros, with the surface that asked", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.raiseCostCap(A_JOB.id, 20_000_000);

  expect(answer.ok).toBe(true);
  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/raise_cost_cap`);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    cost_cap_micros: 20_000_000,
    raised_by: "person",
  });
});

/**
 * **Bridge never sends the other value.** `raised_by` is provenance rather than
 * a credential, filled in by the surface that composed the request — and this
 * app is the one a person presses, so nothing here chooses.
 */
it("never claims to be Helm", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.raiseCostCap(A_JOB.id, 12_500_000);

  expect(JSON.parse(asked[0]?.body ?? "null").raised_by).toBe("person");
});

/**
 * A figure that cannot be a cap is refused before anything goes out. **Not the
 * comparison against the cap in force** — that one is the dialog's, which has
 * both numbers — this is the floor no ceiling can be under.
 */
it("sends nothing at all for a cap of nothing", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.raiseCostCap(A_JOB.id, 0);

  expect(answer).toEqual({ ok: false, why: "cap_not_raised" });
  expect(asked).toHaveLength(0);
});

/**
 * **The unit does not cross, which is this act's own way of going wrong.** The
 * cost cap converts dollars to micros and the case above pins the factor; a
 * turn cap is a turn count on both sides, so what has to be pinned here is that
 * nothing multiplies it. A raise to 600 sent as 600000000 is a ceiling no job
 * ever reaches, and neither side would notice.
 */
it("sends the new turn cap as a turn count, on its own route", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.raiseTurnCap(A_JOB.id, 600);

  expect(answer.ok).toBe(true);
  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/raise_turn_cap`);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({
    turn_cap: 600,
    raised_by: "person",
  });
});

/**
 * A figure that cannot be a cap is refused before anything goes out, and it
 * says which ceiling it was about. **Its own refusal beside `cap_not_raised`**:
 * a message naming money would send somebody to the control that cannot start
 * this job.
 */
it("sends nothing at all for a turn cap of nothing", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.raiseTurnCap(A_JOB.id, 0);

  expect(answer).toEqual({ ok: false, why: "turn_cap_not_raised" });
  expect(asked).toHaveLength(0);
});

/**
 * **The clear is a key with nothing in it, never a missing key.** Fleet refuses
 * a `set_model` body with no `model` rather than reading it as a clear, so a
 * body built with `undefined` — which `JSON.stringify` drops — would earn a 422
 * for the one press that hands the choice back to the workflow.
 */
it("sends a cleared model as null, with the key present", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  const answer = await commands.setModel(A_JOB.id, null);

  expect(answer.ok).toBe(true);
  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/set_model`);
  expect(asked[0]?.body).toBe('{"model":null}');
});

/** The command goes back exactly as it was allowed: it is what Fleet matches on. */
it("names the allow it takes back by the whole command", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(boardOn(await fleetRecording(asked)));

  await commands.removeAllowedCommand(A_JOB.id, "pnpm add -D reselect@5.1.1");

  expect(asked[0]?.path).toBe(`/jobs/${A_JOB.id}/remove_allowed_command`);
  expect(JSON.parse(asked[0]?.body ?? "null")).toEqual({ run: "pnpm add -D reselect@5.1.1" });
});

/** A listener that answers `search_files` with a fixed list, and records the path asked. */
async function fleetListing(paths: string[], into: Asked[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    into.push({ path: request.url ?? "", body: "" });
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify({ paths }));
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

/**
 * The `@` mention popup's own read. What a person typed rides the querystring,
 * encoded — a bare `@` narrows with an empty `q` and a path or a space in the
 * typed text must survive the trip.
 */
it("asks search_files with the typed query, encoded, and answers its paths", async () => {
  const asked: Asked[] = [];
  const commands = new JobCommands(
    boardOn(await fleetListing(["src/log.rs", "README.md"], asked)),
  );

  const found = await commands.searchFiles("a query/with slashes");

  expect(found).toEqual(["src/log.rs", "README.md"]);
  expect(asked[0]?.path).toBe(`/manifest/files?q=${encodeURIComponent("a query/with slashes")}`);
});

/**
 * Fires on every keystroke against a form that may not be connected to
 * anything yet — see `JobCommands.searchFiles`'s own note. Nowhere to ask is
 * answered empty, not as a refusal a popup would have to render.
 */
it("answers no paths at all where there is nowhere to ask", async () => {
  const commands = new JobCommands({ ...boardOn(0), port: () => null });

  const found = await commands.searchFiles("anything");

  expect(found).toEqual([]);
});
