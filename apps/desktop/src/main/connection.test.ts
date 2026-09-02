// What reopens the transcript when a Job advances, against a Fleet that is not
// Fleet.
//
// **One listener, HTTP and both sockets, because that is what Fleet is.** The
// runtime file is written for real and the pid in it is this process's, so the
// connection takes the same path it takes against a running daemon: read the
// file, probe the pid, open `/events`, and only then open a Job's turns.
//
// What is proved here is the half of #324 that lives on this side: Fleet closes
// a Job's transcript socket when the step's Drone exits, and nothing reopened
// it, so the next step ran for ten minutes behind a panel reading `Nothing has
// happened on this step yet.`

import { once } from "node:events";
import { createServer, type IncomingMessage, type Server } from "node:http";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { AddressInfo, Socket } from "node:net";

import { afterEach, expect, it } from "vitest";
import { WebSocketServer, type WebSocket } from "ws";

import { PROTOCOL_VERSION } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { FleetConnection } from "./connection";
import { holderOf } from "./runtime-file";

const A_JOB = "01M1HQZAKN001AJ5MT3PT09KKY";

/** The Board row the resync carries, so an event about it is not a full reread. */
const A_ROW = {
  id: A_JOB,
  title: "Make the parser take it",
  status: "running",
  workflow_id: "bug",
  owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
  origin: "dispatched",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  created_at: "2026-09-02T19:00:00Z",
};

/** Everything a case opened, given back in reverse. */
const opened: (() => void | Promise<void>)[] = [];

afterEach(async () => {
  while (opened.length > 0) await opened.pop()?.();
});

/** A Fleet on one port: `/events`, `/jobs/:id/observe`, and the reads between. */
async function serving(): Promise<{
  port: number;
  events: Promise<WebSocket>;
  /** Each connection to this Job's transcript, in arrival order. */
  observing: WebSocket[];
  /** The next transcript connection past the ones already in hand. */
  watched: (past: number) => Promise<WebSocket>;
}> {
  const stream = new WebSocketServer({ noServer: true });
  const turns = new WebSocketServer({ noServer: true });
  const observing: WebSocket[] = [];
  let arrived: (() => void) | null = null;
  turns.on("connection", (socket) => {
    observing.push(socket);
    arrived?.();
  });

  const server = createServer((request, answer) => {
    // Only the reads this case's path takes. Everything else answers a refusal,
    // which every reader here already renders rather than throwing on.
    answer.writeHead(request.url === `/jobs/${A_JOB}` ? 200 : 404, {
      "content-type": "application/json",
    });
    answer.end(request.url === `/jobs/${A_JOB}` ? JSON.stringify(whole()) : "{}");
  });
  server.on("upgrade", (request: IncomingMessage, socket: Socket, head: Buffer) => {
    const to = (request.url ?? "").endsWith("/observe") ? turns : stream;
    to.handleUpgrade(request, socket, head, (client) => to.emit("connection", client, request));
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  opened.push(() => closing(server));

  return {
    port: (server.address() as AddressInfo).port,
    events: once(stream, "connection").then(([socket]) => socket as WebSocket),
    observing,
    watched: (past: number) =>
      new Promise<WebSocket>((keep) => {
        const look = (): boolean => {
          const found = observing[past];
          if (found === undefined) return false;
          keep(found);
          return true;
        };
        if (!look()) arrived = () => void look();
      }),
  };
}

/** One Job read whole, as `GET /jobs/:id` answers it. */
function whole() {
  return {
    ...A_ROW,
    steps: [],
    acceptance_criteria: [],
    workflow_steps: [],
  };
}

/** Every state main published, and a wait for the one a case is about. */
function publishing() {
  const seen: BridgeState[] = [];
  const wanted: { holds: (state: BridgeState) => boolean; keep: () => void }[] = [];
  return {
    publish(state: BridgeState): void {
      seen.push(state);
      for (const [at, want] of [...wanted.entries()].reverse()) {
        if (!want.holds(state)) continue;
        wanted.splice(at, 1);
        want.keep();
      }
    },
    until(holds: (state: BridgeState) => boolean): Promise<void> {
      if (seen.some(holds)) return Promise.resolve();
      return new Promise((keep) => wanted.push({ holds, keep }));
    },
  };
}

/** The runtime file, naming this process, so the pid probe passes for real. */
async function runtimeFile(port: number): Promise<string> {
  const home = await mkdtemp(join(tmpdir(), "bridge-"));
  const dir = join(home, "Library", "Application Support", "Armada");
  await mkdir(dir, { recursive: true });
  const held = holderOf(process.pid);
  await writeFile(
    join(dir, "fleet.json"),
    JSON.stringify({
      protocol_version: PROTOCOL_VERSION,
      pid: process.pid,
      port,
      started_at: held.held === true ? held.startedAt : "",
    }),
  );
  return home;
}

function closing(server: Server): Promise<void> {
  return new Promise((done) => server.close(() => done()));
}

it("reopens a Job's transcript on the event that says its next step is running", async () => {
  const fleet = await serving();
  const home = await runtimeFile(fleet.port);
  const published = publishing();
  const connection = new FleetConnection({
    home,
    publish: (state) => published.publish(state),
    now: () => 1_756_840_000_000,
  });
  opened.push(() => connection.stop());

  connection.start();
  const stream = await fleet.events;
  stream.send(
    JSON.stringify({
      message: "resync",
      protocol_version: PROTOCOL_VERSION,
      cursor: 1,
      jobs: { jobs: [A_ROW], unreadable: [] },
    }),
  );
  await published.until((state) => state.connection.state === "connected");

  await connection.observeJob(A_JOB);
  const plan = await fleet.watched(0);
  plan.send(
    JSON.stringify({
      message: "opened",
      protocol_version: PROTOCOL_VERSION,
      job_id: A_JOB,
      live: true,
      skipped: 0,
    }),
  );
  // The step's Drone exits. Fleet says so and closes, which is right for a Job
  // that has finished and wrong for one that is on its next step.
  plan.send(JSON.stringify({ message: "closed", because: "drone_ended" }));
  await published.until((state) => state.observed.state === "ended");

  // The Job moves. **The only thing that says a new Drone exists** — nothing on
  // the stream announces a spawn, and the transcript socket is not polled.
  stream.send(
    JSON.stringify({
      message: "event",
      cursor: 2,
      event: {
        kind: "job.state_changed",
        job_id: A_JOB,
        from: "running",
        to: "running",
        actor: "fleet",
        at: "2026-09-02T19:09:59.615Z",
      },
    }),
  );

  const implement = await fleet.watched(1);
  implement.send(
    JSON.stringify({
      message: "opened",
      protocol_version: PROTOCOL_VERSION,
      job_id: A_JOB,
      live: true,
      skipped: 0,
    }),
  );
  implement.send(
    JSON.stringify({
      message: "row",
      ts: "2026-09-02T19:10:04.000Z",
      step: "implement",
      by: "drone",
      event: "said",
      text: "reading the parser",
    }),
  );

  await published.until(
    (state) => state.observed.state === "watching" && state.observed.turns.rows.length === 1,
  );
  expect(fleet.observing).toHaveLength(2);
});
