// Every per-repository call names the picked repository — `picked.ts`. An absent parameter is
// not refused by Fleet; it is the repository Fleet started in, so a call that forgot would act
// on the wrong one silently. So each is sent against a Fleet that records what arrived.

import { once } from "node:events";
import { readdirSync, readFileSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import type { RepositorySummary } from "@armada/protocol";
import { CheckoutRunCommands, CheckoutRunSocket, CheckoutSheetReader, DriftReader } from "./checkout-runs";
import { JobCommands, type Board } from "./command";
import { ManifestFileCommands } from "./editing";
import { Picked } from "./picked";
import { RepositoryAllowsCommands } from "./repository-allows";
import { holdingsOf, manifestReadingOf } from "./request";
import { ServerCommands } from "./servers";

const FIRST: RepositorySummary = {
  root: "/Users/user/armada",
  records_root: "/records/armada",
  manifest: { id: "armada", repository: "armada", path: "/Users/user/armada/armada.yml", records_root: "/records/armada", version: 1, checks: [] },
};
const SET_UP: RepositorySummary = {
  root: "/Users/user/store front",
  records_root: "/records/store",
  manifest: { id: "store-01", repository: "store front", path: "/Users/user/store front/armada.yml", records_root: "/records/store", version: 1, checks: [] },
};
const NOT_SET_UP: RepositorySummary = { root: "/Users/user/scratch", records_root: "/records/scratch" };

function pickedAt(root: string): Picked {
  const picked = new Picked();
  picked.hold([FIRST, SET_UP, NOT_SET_UP]);
  picked.pick(root);
  return picked;
}

describe("the pick", () => {
  it("falls to the first repository Fleet lists, and sends nothing before any is listed", () => {
    const picked = new Picked();
    expect(picked.manifest("/manifest/file")).toBe("/manifest/file");
    expect(picked.hold([FIRST, NOT_SET_UP])).toBe(true);
    expect(picked.picked).toBe(FIRST.root);
    expect(picked.manifest("/manifest/files?q=src")).toBe("/manifest/files?q=src&manifest_id=armada");
  });

  it("ignores a root Fleet does not list, and keeps a pick still served when the list is read again", () => {
    const picked = pickedAt(SET_UP.root);
    expect(picked.pick("/somewhere/else")).toBe(false);
    expect(picked.hold([FIRST, SET_UP])).toBe(false);
    expect(picked.picked).toBe(SET_UP.root);
  });

  it("names a repository nobody set up by root for Scan, and refuses a Manifest route for it", () => {
    const picked = pickedAt(NOT_SET_UP.root);
    expect(picked.scan("/repository/scan")).toBe("/repository/scan?repository=%2FUsers%2Fuser%2Fscratch");
    expect(picked.manifest("/manifest/run_sheet")).toBeNull();
  });

  it("moves when Write gives the picked repository its Manifest", () => {
    const picked = pickedAt(NOT_SET_UP.root);
    const written = { ...NOT_SET_UP, manifest: { ...SET_UP.manifest!, id: "scratch" } };
    expect(picked.hold([FIRST, SET_UP, written])).toBe(true);
    expect(picked.manifest("/manifest/start_verify")).toBe("/manifest/start_verify?manifest_id=scratch");
  });

  it("reads only the picked repository's `manifest.reread`", () => {
    const picked = pickedAt(SET_UP.root);
    expect(picked.reads("/Users/user/store front/armada.yml")).toBe(true);
    expect(picked.reads("/Users/user/armada/armada.yml")).toBe(false);
  });
});

let listening: Server | null = null;
afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise<void>((done) => server.close(() => done()));
});

/** A Fleet that answers every request with one body every caller can read, and records each URL. */
async function recording(into: string[]): Promise<number> {
  const server = createServer((request, response) => {
    into.push(request.url ?? "");
    request.resume();
    request.on("end", () => {
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify({ id: "run-1", steps: [], paths: [], jobs: [], repositories: [] }));
    });
  });
  server.on("upgrade", (request, socket) => {
    into.push(request.url ?? "");
    socket.destroy();
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  return (server.address() as AddressInfo).port;
}

function boardOn(port: number, picked: Picked): Board {
  return {
    port: () => port,
    picked,
    fold: () => {},
    forget: () => {},
    reread: async () => {},
    refresh: () => {},
    publish: () => {},
    watchProposal: () => {},
    proposalOut: () => null,
    rereadCapacity: async () => {},
  };
}

/** Every per-repository call in `src/main`, sent once. */
async function everyCall(port: number, picked: Picked): Promise<void> {
  const at = () => port;
  const editing = new ManifestFileCommands(at, picked, async () => {});
  await editing.readFile();
  await editing.saveFile({ read: "", text: "" });
  await editing.edit({ read: "", edits: [] });
  await editing.readSpend();
  await editing.setup.readScan();
  await editing.setup.readProposals();
  await editing.setup.edit({ dir: ".", edit: { edit: "id", id: "x" } });
  await editing.setup.write({ dir: "." });
  const allows = new RepositoryAllowsCommands(at, picked);
  await allows.list();
  await allows.remove("git status");
  await new CheckoutSheetReader(() => {}, picked).want(port, true);
  await new DriftReader(() => {}, picked).want(port, true);
  const runs = new CheckoutRunCommands({ port: at, picked, follow: () => {}, refreshSheet: async () => {} });
  await runs.startRun({ name: "test" });
  await runs.startVerify();
  await runs.stopRun("run-1");
  await runs.undoRun("run-1");
  await runs.listRuns();
  await runs.getRunOutput("run-1");
  await runs.getRunDiff("run-1");
  await new ServerCommands({ port: at, picked }).startServer("web");
  const commands = new JobCommands(boardOn(port, picked));
  await commands.searchFiles("src");
  await commands.proposeFromRequest("Fix the parser", []);
  await holdingsOf(port, { workflows: [], manifests: [], models: null }, picked);
  await manifestReadingOf(port, picked);
}

/** The three Fleet-wide holdings reads name nothing. */
const FLEETWIDE = new Set(["/workflows", "/manifests", "/models"]);

/** What `everyCall` reaches, by path alone — one entry per call. */
const EVERY_ROUTE = [
  "/jobs/from_request",
  "/manifest/allowed_commands",
  "/manifest/allowed_commands/remove",
  "/manifest/drift",
  "/manifest/edit",
  "/manifest/file",
  "/manifest/files",
  "/manifest/reading",
  "/manifest/run_sheet",
  "/manifest/runs",
  "/manifest/runs/run-1/diff",
  "/manifest/runs/run-1/observe",
  "/manifest/runs/run-1/output",
  "/manifest/save_file",
  "/manifest/spend",
  "/manifest/start_run",
  "/manifest/start_verify",
  "/manifest/stop_run",
  "/manifest/undo_run",
  "/repository/edit_proposal",
  "/repository/proposals",
  "/repository/scan",
  "/repository/write_proposal",
  "/servers/start",
  "/workflows/left_out",
].sort();

describe("every per-repository call", () => {
  it("names the picked repository", async () => {
    const asked: string[] = [];
    const port = await recording(asked);
    const picked = pickedAt(SET_UP.root);
    await everyCall(port, picked);
    const socket = new CheckoutRunSocket(() => {}, picked);
    socket.open(port, "run-1");
    for (let tries = 0; tries < 100 && !asked.some((url) => url.includes("/observe")); tries += 1) {
      await new Promise((later) => setTimeout(later, 10));
    }
    socket.close();

    const scoped = asked.filter((url) => !FLEETWIDE.has(url));
    expect(scoped.map((url) => url.split("?")[0]).sort()).toEqual(EVERY_ROUTE);
    for (const url of scoped) {
      const named = url.startsWith("/repository/")
        ? url.endsWith("repository=%2FUsers%2Fuser%2Fstore%20front")
        : url.endsWith("manifest_id=store-01");
      expect(named, url).toBe(true);
    }
  });

  it("sends no Manifest route for a repository nobody set up, and still names it to Scan", async () => {
    const asked: string[] = [];
    const port = await recording(asked);
    const picked = pickedAt(NOT_SET_UP.root);
    await everyCall(port, picked);
    const scoped = asked.filter((url) => !FLEETWIDE.has(url));
    expect(scoped.every((url) => url.startsWith("/repository/") && url.includes("repository=%2FUsers%2Fuser%2Fscratch"))).toBe(true);
    expect(scoped).toHaveLength(4);
    const editing = new ManifestFileCommands(() => port, picked, async () => {});
    expect(await editing.readFile()).toEqual({ ok: false, outcome: { ok: false, why: "not_set_up" } });
  });

  it("is built through the pick wherever `src/main` spells a per-repository route", () => {
    const routes = /["`](\/manifest\/|\/repository\/|\/workflows\/left_out|\/servers\/start|\/jobs\/from_request)/;
    const dir = __dirname;
    const unnamed = readdirSync(dir)
      .filter((file) => file.endsWith(".ts") && !file.endsWith(".test.ts") && file !== "picked.ts")
      .flatMap((file) =>
        readFileSync(join(dir, file), "utf8")
          .split("\n")
          .map((line, index) => ({ at: `${file}:${index + 1}`, line }))
          .filter(({ line }) => routes.test(line) && !line.trim().startsWith("*") && !line.trim().startsWith("//"))
          .filter(({ line }) => !line.includes("picked.manifest(") && !line.includes("picked.scan(")),
      )
      .map(({ at }) => at);
    expect(unnamed).toEqual([]);
  });
});
