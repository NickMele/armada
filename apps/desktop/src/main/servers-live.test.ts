// A server's state reaching the sheet that draws it. Both run sheets draw a server off the sheet's
// own `servers[].instance`, and `server.*` was folded only into `BridgeState.servers`, so Run and
// Stop on a server changed nothing on screen until a reload re-read the sheet.

import { once } from "node:events";
import { createServer, type IncomingMessage, type Server } from "node:http";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { AddressInfo, Socket } from "node:net";

import { afterEach, expect, it } from "vitest";
import { WebSocketServer, type WebSocket } from "ws";

import { PROTOCOL_VERSION, type ServerState } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { FleetConnection } from "./connection";
import { holderOf } from "./runtime-file";

const A_JOB = "01M1HQZAKN001AJ5MT3PT09KKY";

const opened: (() => void | Promise<void>)[] = [];
afterEach(async () => {
  while (opened.length > 0) await opened.pop()?.();
});

function instance(phase: string, jobId?: string): ServerState {
  return {
    id: "01M2E5KFWY0023WBZZF1GF7K48",
    name: "storybook_dev",
    ...(jobId === undefined ? {} : { job_id: jobId }),
    phase,
    serve: "storybook dev -p 40000",
    ports: [],
    links: [],
    started_by: "person",
    started_at: "2026-09-13T19:58:42.590Z",
    stopped: false,
    log: ".armada/servers/main/output.log",
  };
}

/** A Fleet whose sheets carry whatever `held` is when they are read. */
async function fleet() {
  const held: { checkout?: ServerState; job?: ServerState } = {};
  const stream = new WebSocketServer({ noServer: true });
  const entry = (at?: ServerState) => ({ name: "storybook_dev", serve: "storybook dev", links: [], destructive: false, ...(at === undefined ? {} : { instance: at }) });
  const sheet = { setup: [], checks: [], commands: [] };
  const server: Server = createServer((request, answer) => {
    const route = request.url ?? "";
    const body: Record<string, unknown> = {
      "/manifest/run_sheet": { ...sheet, servers: [entry(held.checkout)] },
      [`/jobs/${A_JOB}/run_sheet`]: { ...sheet, job_id: A_JOB, worktree_on_disk: true, drone_working: false, servers: [entry(held.job)] },
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
  const client = once(stream, "connection").then(([socket]) => socket as WebSocket);
  return { port, held, client };
}

async function bridgeOn(port: number) {
  const home = await mkdtemp(join(tmpdir(), "bridge-"));
  const dir = join(home, "Library", "Application Support", "Armada");
  await mkdir(dir, { recursive: true });
  const holder = holderOf(process.pid);
  const startedAt = holder.held === true ? holder.startedAt : "";
  await writeFile(join(dir, "fleet.json"), JSON.stringify({ protocol_version: PROTOCOL_VERSION, pid: process.pid, port, started_at: startedAt }));
  let latest: BridgeState | null = null;
  const waits: { holds: (state: BridgeState) => boolean; keep: () => void }[] = [];
  const connection = new FleetConnection({
    home,
    now: () => 1_757_000_000_000,
    publish: (state) => {
      latest = state;
      for (const [at, wait] of [...waits.entries()].reverse()) if (wait.holds(state)) waits.splice(at, 1) && wait.keep();
    },
    publishToWindow: () => {},
    windowIds: () => [],
  });
  opened.push(() => connection.stop());
  connection.start();
  // Fails by name rather than on the runner's timeout, so a red run says what never arrived.
  const until = (what: string, holds: (state: BridgeState) => boolean) =>
    latest !== null && holds(latest)
      ? Promise.resolve()
      : Promise.race([
          new Promise<void>((keep) => waits.push({ holds, keep })),
          new Promise<void>((_, refuse) => setTimeout(() => refuse(new Error(`never published: ${what}`)), 2000)),
        ]);
  return { connection, until, latest: () => latest! };
}

function event(cursor: number, row: ServerState, kind = `server.${row.phase}`): string {
  return JSON.stringify({ message: "event", cursor, event: { kind, ...row } });
}

it("draws a main-checkout server as it moves, on the Manifest surface's sheet", async () => {
  const { port, held, client } = await fleet();
  const { connection, until } = await bridgeOn(port);
  const socket = await client;
  socket.send(JSON.stringify({ message: "resync", protocol_version: PROTOCOL_VERSION, cursor: 1, jobs: { jobs: [] } }));
  await until("connected", (state) => state.connection.state === "connected");

  await connection.rehearsal.watchCheckoutRunSheet(true);
  const phase = (state: BridgeState) =>
    state.checkoutRunSheet.state === "read" ? state.checkoutRunSheet.sheet.servers?.[0]?.instance?.phase : "unread";
  await until("the sheet, before Run", (state) => phase(state) === undefined);

  held.checkout = instance("starting");
  socket.send(event(2, held.checkout));
  await until("starting, on the sheet", (state) => phase(state) === "starting");

  held.checkout = { ...instance("exited"), stopped: true };
  socket.send(event(3, held.checkout));
  await until("exited, on the sheet", (state) => phase(state) === "exited");
});

it("draws a Job's server as it moves, on that Job's run sheet and no other", async () => {
  const { port, held, client } = await fleet();
  const { connection, until, latest } = await bridgeOn(port);
  const socket = await client;
  socket.send(JSON.stringify({ message: "resync", protocol_version: PROTOCOL_VERSION, cursor: 1, jobs: { jobs: [] } }));
  await until("connected", (state) => state.connection.state === "connected");

  await connection.rehearsal.watchRunSheet(A_JOB);
  const phase = (state: BridgeState) =>
    state.runSheet.state === "read" ? state.runSheet.sheet.servers?.[0]?.instance?.phase : "unread";
  await until("the Job's sheet, before Run", (state) => phase(state) === undefined);

  // Another Job's server moves this sheet not at all: what it re-reads still has no instance.
  held.job = instance("serving", A_JOB);
  socket.send(event(2, instance("serving", "01M1HQZAKN001AJ5MT3PT0OTHER")));
  await new Promise((settle) => setTimeout(settle, 200));
  expect(phase(latest())).toBe(undefined);

  socket.send(event(3, held.job));
  await until("serving, on the Job's sheet", (state) => phase(state) === "serving");
});
