// The two bulk acts, read off a real listener rather than asserted against the
// code that sends them.
//
// **`#570`'s whole defect was this seam sending the wrong route.** `clearTerminal`
// called `forget_job` where `reclaim_worktree` already existed and was unused —
// so what is under test here is which path crosses the wire, the same way
// `command.test.ts` asserts bytes rather than trusting the function that built
// them.

import { createServer, type IncomingMessage, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { WorktreeReclaimed } from "@armada/protocol";
import type { Board } from "./command";
import { Clearing } from "./clearing";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

/** One receipt, with a branch a base already reaches — the ordinary case. */
function reclaimed(jobId: string): WorktreeReclaimed {
  return {
    job_id: jobId,
    worktree: { path: `/repo/.armada/worktrees/${jobId}`, removed: true },
    branch: { branch: `armada/${jobId}`, deleted: true, tip: "abc123" },
  };
}

/**
 * A listener that answers `reclaim_worktree` with a receipt and `forget_job`
 * with an empty body, and records every path it was asked for.
 */
async function fleetRecording(into: string[]): Promise<number> {
  const server = createServer((request: IncomingMessage, response) => {
    let body = "";
    request.on("data", (chunk) => (body += String(chunk)));
    request.on("end", () => {
      const path = request.url ?? "";
      into.push(path);
      response.writeHead(200, { "content-type": "application/json" });
      if (path.endsWith("/reclaim_worktree")) {
        const jobId = decodeURIComponent(path.split("/")[2] ?? "");
        response.end(JSON.stringify(reclaimed(jobId)));
      } else {
        response.end(JSON.stringify({ job_id: path.split("/")[2] ?? "" }));
      }
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

/** The board `Clearing` folds and forgets into. Nothing here is under test. */
function boardOn(port: number, forgotten: string[]): Board {
  return {
    port: () => port,
    fold: () => {},
    forget: (jobId) => forgotten.push(jobId),
    reread: async () => {},
    refresh: () => {},
    publish: () => {},
    watchProposal: () => {},
    proposalOut: () => null,
  };
}

/**
 * **The defect, pinned.** `clearTerminal` used to loop `forget_job` — real
 * deletion — where the board's `Clear` button meant to reclaim disk and keep
 * the record. This is the route it must send instead.
 */
it("clearTerminal sends reclaim_worktree, never forget_job", async () => {
  const asked: string[] = [];
  const forgotten: string[] = [];
  const clearing = new Clearing(boardOn(await fleetRecording(asked), forgotten));

  const result = await clearing.clearTerminal(["01A", "01B"]);

  expect(asked).toEqual([
    "/jobs/01A/reclaim_worktree",
    "/jobs/01B/reclaim_worktree",
  ]);
  expect(forgotten).toEqual([]);
  expect(result.failed).toEqual([]);
  expect(result.reclaimed.map((r) => r.job_id)).toEqual(["01A", "01B"]);
});

/** The record's own bulk act, and it is a different route from the reclaim's. */
it("forgetTerminal sends forget_job, and folds each id out of the board", async () => {
  const asked: string[] = [];
  const forgotten: string[] = [];
  const clearing = new Clearing(boardOn(await fleetRecording(asked), forgotten));

  const result = await clearing.forgetTerminal(["01A", "01B"]);

  expect(asked).toEqual(["/jobs/01A/forget_job", "/jobs/01B/forget_job"]);
  expect(forgotten).toEqual(["01A", "01B"]);
  expect(result.cleared).toEqual(["01A", "01B"]);
  expect(result.failed).toEqual([]);
});

/**
 * The per-Job door to the same delete, now that a job detail can reach it on
 * its own rather than only through the bulk sweep.
 */
it("forget, called singly, is the same act forgetTerminal sends in bulk", async () => {
  const asked: string[] = [];
  const forgotten: string[] = [];
  const clearing = new Clearing(boardOn(await fleetRecording(asked), forgotten));

  const outcome = await clearing.forget("01A");

  expect(outcome.ok).toBe(true);
  expect(asked).toEqual(["/jobs/01A/forget_job"]);
  expect(forgotten).toEqual(["01A"]);
});

/** A single reclaim never reaches the board — the row is untouched. */
it("reclaim, called singly, never forgets the row", async () => {
  const asked: string[] = [];
  const forgotten: string[] = [];
  const clearing = new Clearing(boardOn(await fleetRecording(asked), forgotten));

  const outcome = await clearing.reclaim("01A");

  expect(outcome.ok).toBe(true);
  expect(asked).toEqual(["/jobs/01A/reclaim_worktree"]);
  expect(forgotten).toEqual([]);
});

/** A failed id does not stop the rest — the safety net Fleet's 409 provides. */
it("clearTerminal keeps going past a refusal", async () => {
  const server = createServer((request, response) => {
    request.on("data", () => {});
    request.on("end", () => {
      if ((request.url ?? "").startsWith("/jobs/01BAD/")) {
        response.writeHead(409, { "content-type": "application/json" });
        response.end(JSON.stringify({ code: "fleet.not_reclaimable", message: "not terminal" }));
        return;
      }
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(reclaimed("01GOOD")));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  const port = (server.address() as AddressInfo).port;
  const forgotten: string[] = [];
  const clearing = new Clearing(boardOn(port, forgotten));

  const result = await clearing.clearTerminal(["01BAD", "01GOOD"]);

  expect(result.failed.map((f) => f.jobId)).toEqual(["01BAD"]);
  expect(result.reclaimed.map((r) => r.job_id)).toEqual(["01GOOD"]);
});
