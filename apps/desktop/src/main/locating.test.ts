// Add and clone, as main sends them: the body, the wait, and what a person lands on after.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import { CLONE_MS, Locating, locateAnswerOf } from "./locating";
import { ask, COMMAND_MS } from "./request";

const ADDED = { root: "/Users/user/scratch", records_root: "/records/scratch" };

let listening: Server | null = null;
afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise<void>((done) => server.close(() => done()));
});

/** A Fleet answering one status and body to every request, recording what arrived. */
async function fleet(status: number, body: unknown, into: { url: string; body: string }[]): Promise<number> {
  const server = createServer((request, response) => {
    let text = "";
    request.on("data", (chunk) => (text += chunk));
    request.on("end", () => {
      into.push({ url: request.url ?? "", body: text });
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    });
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  return (server.address() as AddressInfo).port;
}

function wired(port: number, done: string[]): Locating {
  return new Locating({
    port: () => port,
    list: async () => void done.push("list"),
    pick: async (root) => void done.push(`pick ${root}`),
  });
}

describe("adding a folder", () => {
  it("posts the path, then lists and picks what Fleet now serves", async () => {
    const asked: { url: string; body: string }[] = [];
    const done: string[] = [];
    const port = await fleet(201, ADDED, asked);
    const answer = await wired(port, done).add("/Users/user/scratch");
    expect(asked).toEqual([{ url: "/repositories/add", body: JSON.stringify({ path: "/Users/user/scratch" }) }]);
    expect(answer).toEqual({ state: "located", repository: ADDED });
    expect(done).toEqual(["list", `pick ${ADDED.root}`]);
  });

  it("reads a refusal in Fleet's words, and picks nothing", async () => {
    const refusal = { code: "fleet.not_a_repository", message: "/tmp is not the root of a git repository", run_id: "r", fields: {}, chain: [] };
    const done: string[] = [];
    const port = await fleet(422, refusal, []);
    const answer = await wired(port, done).add("/tmp");
    expect(answer).toEqual({ state: "refused", code: "fleet.not_a_repository", saying: refusal.message });
    expect(done).toEqual([]);
  });

  it("sends nothing without a Fleet", async () => {
    const locating = new Locating({ port: () => null, list: async () => {}, pick: async () => {} });
    expect(await locating.add("/Users/user/scratch")).toEqual({ state: "failed", outcome: { ok: false, why: "not_connected" } });
  });
});

describe("cloning from a URL", () => {
  it("waits past Fleet's ten-minute bound, where an add waits a command's", async () => {
    const waits: { path: string; waitMs: number | undefined; body: unknown }[] = [];
    const recording: typeof ask = async (_port, _method, path, body, waitMs) => {
      waits.push({ path, waitMs, body });
      return { ok: true, body: ADDED };
    };
    const locating = new Locating({ port: () => 1, list: async () => {}, pick: async () => {} }, recording);
    await locating.clone("file:///tmp/remotes/scratch.git", "/Users/user/code");
    await locating.add("/Users/user/scratch");
    expect(CLONE_MS).toBeGreaterThan(10 * 60_000);
    expect(waits).toEqual([
      { path: "/repositories/clone", waitMs: CLONE_MS, body: { url: "file:///tmp/remotes/scratch.git", parent: "/Users/user/code" } },
      { path: "/repositories/add", waitMs: COMMAND_MS, body: { path: "/Users/user/scratch" } },
    ]);
  });

  it("does not send a second while one is out", async () => {
    let finish: () => void = () => {};
    let sent = 0;
    const slow: typeof ask = (_port, _method, _path) => {
      sent += 1;
      return new Promise((resolve) => (finish = () => resolve({ ok: true, body: ADDED })));
    };
    const locating = new Locating({ port: () => 1, list: async () => {}, pick: async () => {} }, slow);
    const first = locating.clone("git@host:scratch.git", "/Users/user/code");
    expect(await locating.clone("git@host:scratch.git", "/Users/user/code")).toEqual({ state: "busy" });
    finish();
    expect((await first).state).toBe("located");
    expect(sent).toBe(1);
  });

  it("carries a transport failure whole", () => {
    const outcome = { ok: false as const, why: "transport" as const, detail: "aborted", fault: { method: "POST" as const, path: "/repositories/clone", why: "timed_out" as const, waitedMs: CLONE_MS } };
    expect(locateAnswerOf({ ok: false, outcome })).toEqual({ state: "failed", outcome });
  });
});
