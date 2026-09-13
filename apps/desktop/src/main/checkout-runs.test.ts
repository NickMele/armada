// A repository with no root Manifest runs a workspace's Command, and reads its history, output
// and diff and undoes it, by root — there is no Manifest id to name it by.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import type { RepositorySummary } from "@armada/protocol";
import { CheckoutRunCommands } from "./checkout-runs";
import { Picked } from "./picked";
import { HOST } from "./runtime-file";

const ALONE: RepositorySummary = { root: "/Users/user/alone", records_root: "/records/alone" };
const BY_ROOT = "repository=%2FUsers%2Fuser%2Falone";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise((done) => server.close(done));
});

async function recording(into: { url: string; body: string }[]): Promise<number> {
  const server = createServer((request, response) => {
    let body = "";
    request.on("data", (chunk: Buffer) => (body += String(chunk)));
    request.on("end", () => {
      into.push({ url: request.url ?? "", body });
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify({ id: "run-1", runs: [], unreadable: [] }));
    });
  });
  listening = server;
  server.listen(0, HOST);
  await once(server, "listening");
  return (server.address() as AddressInfo).port;
}

describe("a checkout run where the repository has no Manifest id", () => {
  it("names the repository by root on every run call, and carries the workspace on start", async () => {
    const asked: { url: string; body: string }[] = [];
    const port = await recording(asked);
    const picked = new Picked();
    picked.hold([ALONE]);
    picked.pick(ALONE.root);
    const runs = new CheckoutRunCommands({ port: () => port, picked, follow: () => {}, refreshSheet: async () => {} });

    expect(await runs.startRun({ name: "say", workspace: "apps/web" })).toEqual({ ok: true });
    expect((await runs.listRuns()).ok).toBe(true);
    expect((await runs.getRunOutput("run-1")).ok).toBe(true);
    expect((await runs.getRunDiff("run-1")).ok).toBe(true);
    expect(await runs.undoRun("run-1")).toEqual({ ok: true });

    expect(asked.map(({ url }) => url)).toEqual([
      `/manifest/start_run?${BY_ROOT}`,
      `/manifest/runs?${BY_ROOT}`,
      `/manifest/runs/run-1/output?${BY_ROOT}`,
      `/manifest/runs/run-1/diff?${BY_ROOT}`,
      `/manifest/undo_run?${BY_ROOT}`,
    ]);
    expect(JSON.parse(asked[0]?.body ?? "")).toEqual({ name: "say", workspace: "apps/web" });
  });
});
