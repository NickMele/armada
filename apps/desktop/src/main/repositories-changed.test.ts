// `repositories.changed`, folded: an add made anywhere reaches this Bridge's list and picker without
// a round trip, and the pick stays where it is still served. #887.

import { once } from "node:events";
import { createServer, type IncomingMessage, type Server } from "node:http";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { AddressInfo, Socket } from "node:net";

import { afterEach, expect, it } from "vitest";
import { WebSocketServer, type WebSocket } from "ws";

import { PROTOCOL_VERSION, type RepositorySummary } from "@armada/protocol";
import type { BridgeState, PickedView } from "../shared/bridge";
import { FleetConnection } from "./connection";
import { holderOf } from "./runtime-file";

const NO_PICK: PickedView = { repository: null, manifestReading: null };

const ARMADA: RepositorySummary = {
  root: "/Users/user/armada",
  records_root: "/records/armada",
  manifest: { id: "armada", repository: "armada", path: "/Users/user/armada/armada.yml", records_root: "/records/armada", version: 1, checks: [] },
};
const SCRATCH: RepositorySummary = { root: "/Users/user/scratch", records_root: "/records/scratch" };

const opened: (() => void | Promise<void>)[] = [];
afterEach(async () => {
  while (opened.length > 0) await opened.pop()?.();
});

/** A Fleet serving `listing` over HTTP and `/events`, counting each route read. */
async function fleetServing(listing: RepositorySummary[]) {
  const reads = new Map<string, number>();
  const stream = new WebSocketServer({ noServer: true });
  const server: Server = createServer((request, answer) => {
    const route = request.url ?? "";
    reads.set(route, (reads.get(route) ?? 0) + 1);
    const body: Record<string, unknown> = {
      "/repositories": { repositories: listing },
      "/workflows": [],
      "/manifests": [],
      "/models": { choices: [] },
    };
    const found = body[route];
    answer.writeHead(found === undefined ? 404 : 200, { "content-type": "application/json" });
    answer.end(JSON.stringify(found ?? {}));
  });
  server.on("upgrade", (request: IncomingMessage, socket: Socket, head: Buffer) =>
    stream.handleUpgrade(request, socket, head, (client) => stream.emit("connection", client, request)),
  );
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  opened.push(() => new Promise<void>((done) => server.close(() => done())));
  const port = (server.address() as AddressInfo).port;
  return { port, socket: once(stream, "connection").then(([client]) => client as WebSocket), read: (route: string) => reads.get(route) ?? 0 };
}

/**
 * Connected to that Fleet through a runtime file naming this process, and every state it
 * published — the shared half, and each window's own pick apart from it. **Window `1` exists
 * from the start**; a case that wants a second opens it with `open`.
 */
async function bridgeOn(port: number) {
  const home = await mkdtemp(join(tmpdir(), "bridge-"));
  const dir = join(home, "Library", "Application Support", "Armada");
  await mkdir(dir, { recursive: true });
  const held = holderOf(process.pid);
  const startedAt = held.held === true ? held.startedAt : "";
  await writeFile(join(dir, "fleet.json"), JSON.stringify({ protocol_version: PROTOCOL_VERSION, pid: process.pid, port, started_at: startedAt }));
  let latest: BridgeState | null = null;
  const waits: { holds: (state: BridgeState) => boolean; keep: () => void }[] = [];
  const windows = new Set<number>([1]);
  const views = new Map<number, PickedView>();
  const viewWaits: { windowId: number; holds: (view: PickedView) => boolean; keep: () => void }[] = [];
  const connection = new FleetConnection({
    home,
    now: () => 1_757_000_000_000,
    publish: (state) => {
      latest = state;
      for (const [at, wait] of [...waits.entries()].reverse()) if (wait.holds(state)) waits.splice(at, 1) && wait.keep();
    },
    publishToWindow: (windowId, change) => {
      const view = { ...(views.get(windowId) ?? NO_PICK), ...change };
      views.set(windowId, view);
      for (const [at, wait] of [...viewWaits.entries()].reverse()) {
        if (wait.windowId === windowId && wait.holds(view)) viewWaits.splice(at, 1) && wait.keep();
      }
    },
    windowIds: () => [...windows],
  });
  opened.push(() => connection.stop());
  connection.start();
  const until = (holds: (state: BridgeState) => boolean) =>
    latest !== null && holds(latest) ? Promise.resolve() : new Promise<void>((keep) => waits.push({ holds, keep }));
  const untilView = (windowId: number, holds: (view: PickedView) => boolean) =>
    views.has(windowId) && holds(views.get(windowId)!)
      ? Promise.resolve()
      : new Promise<void>((keep) => viewWaits.push({ windowId, holds, keep }));
  return {
    until,
    untilView,
    latest: () => latest!,
    viewOf: (windowId: number) => views.get(windowId) ?? NO_PICK,
    open: (windowId: number) => windows.add(windowId),
    pick: (windowId: number, root: string | null) => connection.repositories.pick(windowId, root),
  };
}

const resync = JSON.stringify({ message: "resync", protocol_version: PROTOCOL_VERSION, cursor: 1, jobs: { jobs: [], unreadable: [] } });
const changed = (repositories: RepositorySummary[]) =>
  JSON.stringify({ message: "event", cursor: 2, event: { kind: "repositories.changed", repositories } });
const rootsOf = (state: BridgeState) => (state.holds.repositories ?? []).map((one) => one.root);

it("lists the first repository added to a Fleet that served none, from the event alone, and stays on All", async () => {
  const fleet = await fleetServing([]);
  const bridge = await bridgeOn(fleet.port);
  (await fleet.socket).send(resync);
  await bridge.until((state) => state.holds.models !== null);
  expect(rootsOf(bridge.latest())).toEqual([]);
  expect(bridge.viewOf(1).repository).toBeNull();

  (await fleet.socket).send(changed([SCRATCH]));
  await bridge.until((state) => rootsOf(state).length === 1);
  expect(rootsOf(bridge.latest())).toEqual([SCRATCH.root]);
  expect(bridge.viewOf(1).repository).toBeNull();
  expect(fleet.read("/repositories")).toBe(1);
});

it("keeps the pick where it is still served when another repository is added elsewhere", async () => {
  const fleet = await fleetServing([ARMADA]);
  const bridge = await bridgeOn(fleet.port);
  (await fleet.socket).send(resync);
  await bridge.until((state) => rootsOf(state).length === 1);
  expect(bridge.viewOf(1).repository).toBeNull();
  // Picking reads the listing again, so what the event costs is counted from here.
  await bridge.pick(1, ARMADA.root);
  await bridge.untilView(1, (view) => view.repository === ARMADA.root);
  const read = fleet.read("/repositories");

  // Listed ahead of the pick, which would take the first place if the pick were reset.
  (await fleet.socket).send(changed([SCRATCH, ARMADA]));
  await bridge.until((state) => rootsOf(state).length === 2);
  expect(rootsOf(bridge.latest())).toEqual([SCRATCH.root, ARMADA.root]);
  expect(bridge.viewOf(1).repository).toBe(ARMADA.root);
  expect(fleet.read("/repositories")).toBe(read);
});

it("moves only the window that picks — Open Setup in one leaves another's pick", async () => {
  const fleet = await fleetServing([ARMADA, SCRATCH]);
  const bridge = await bridgeOn(fleet.port);
  bridge.open(2);
  (await fleet.socket).send(resync);
  await bridge.until((state) => rootsOf(state).length === 2);
  expect(bridge.viewOf(1).repository).toBeNull();
  expect(bridge.viewOf(2).repository).toBeNull();

  // Window 1 picks a repository, the way Open Setup picks in the window it was pressed in.
  await bridge.pick(1, SCRATCH.root);
  await bridge.untilView(1, (view) => view.repository === SCRATCH.root);

  // Window 2 never picked. Its own view still names armada — All, rather.
  expect(bridge.viewOf(2).repository).toBeNull();

  // Window 2 picks armada for itself; window 1 keeps storefront.
  await bridge.pick(2, ARMADA.root);
  await bridge.untilView(2, (view) => view.repository === ARMADA.root);
  expect(bridge.viewOf(1).repository).toBe(SCRATCH.root);
  expect(bridge.viewOf(2).repository).toBe(ARMADA.root);
});
