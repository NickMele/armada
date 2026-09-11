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
//
// And the second case: a Drone's question reaching the Board row it belongs on.
//
// And the third and fourth, which are one rule from both sides: **a reconnection
// brings back every region of the open Job's screen, and a gap in the stream
// does not fetch the patch.** #472 was the detail being re-read on a resync and
// what the Job holds not being, so the panel left saying Fleet was not answering
// was the one reporting the outage. The pair here is what keeps the fix from
// being replaced by its own opposite — a resync that refetches everything.

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

/**
 * Every read one Job's screen is drawn from, by the route it is made on.
 *
 * **The list `screen.ts` classifies**, written out here so a
 * read that is added to one and not the other is a case that fails rather than
 * a region that never comes back.
 */
const SCREEN = {
  detail: `/jobs/${A_JOB}`,
  resources: `/jobs/${A_JOB}/resources`,
  history: `/jobs/${A_JOB}/events`,
  evidence: `/jobs/${A_JOB}/evidence`,
  diff: `/jobs/${A_JOB}/diff`,
} as const;

/**
 * **Deliberately off `SCREEN`.** `screen.ts` excludes the comments from every
 * occasion that table classifies — reading them costs a forge call, so
 * nothing takes them again on a reconnection or a gap. `job.remarks_changed`
 * is its own, narrower trigger, tested on its own route below.
 */
const REMARKS_ROUTE = `/jobs/${A_JOB}/remarks`;

/** What each of those answers. Every body is the smallest one that reads. */
function answering(route: string): unknown {
  switch (route) {
    case SCREEN.detail:
      return { ...A_ROW, steps: [], acceptance_criteria: [], workflow_steps: [] };
    case SCREEN.resources:
      return { job_id: A_JOB, read_at: "2026-09-02T19:00:00Z", held: "running", processes: [] };
    case SCREEN.history:
      return { job_id: A_JOB, moves: [] };
    case SCREEN.evidence:
      return { job_id: A_JOB, steps: [] };
    case SCREEN.diff:
      return { job_id: A_JOB };
    case REMARKS_ROUTE:
      return { job_id: A_JOB, pull_request: "https://forge.example/armada/pull/1", remarks: [] };
    default:
      return undefined;
  }
}

/** Each connection to one route, in arrival order, and a wait for the next. */
function arriving(server: WebSocketServer): {
  all: WebSocket[];
  past: (already: number) => Promise<WebSocket>;
} {
  const all: WebSocket[] = [];
  let waiting: (() => void) | null = null;
  server.on("connection", (socket: WebSocket) => {
    all.push(socket);
    waiting?.();
  });
  return {
    all,
    past: (already: number) =>
      new Promise<WebSocket>((keep) => {
        const look = (): boolean => {
          const found = all[already];
          if (found === undefined) return false;
          keep(found);
          return true;
        };
        if (!look()) waiting = () => void look();
      }),
  };
}

/** A Fleet on one port: `/events`, both per-Job sockets, and the reads between. */
async function serving(): Promise<{
  port: number;
  /** Each connection to `/events`. A reconnection is the second one. */
  stream: (past: number) => Promise<WebSocket>;
  /** Each connection to this Job's transcript, in arrival order. */
  observing: WebSocket[];
  /** The next transcript connection past the ones already in hand. */
  watched: (past: number) => Promise<WebSocket>;
  /** Each connection to this Job's own log. */
  logging: WebSocket[];
  /** How many times one route has been read. */
  read: (route: string) => number;
  /** Fleet goes away: every socket it holds closes. The port stays listening. */
  goesAway: () => void;
}> {
  const stream = new WebSocketServer({ noServer: true });
  const turns = new WebSocketServer({ noServer: true });
  const notes = new WebSocketServer({ noServer: true });
  const events = arriving(stream);
  const transcripts = arriving(turns);
  const logs = arriving(notes);

  const reads = new Map<string, number>();
  const server = createServer((request, answer) => {
    const route = request.url ?? "";
    reads.set(route, (reads.get(route) ?? 0) + 1);
    // Anything off the list answers a refusal, which every reader here already
    // renders rather than throwing on.
    const body = answering(route);
    answer.writeHead(body === undefined ? 404 : 200, { "content-type": "application/json" });
    answer.end(JSON.stringify(body ?? {}));
  });
  server.on("upgrade", (request: IncomingMessage, socket: Socket, head: Buffer) => {
    const path = request.url ?? "";
    const to = path.endsWith("/observe") ? turns : path.endsWith("/log") ? notes : stream;
    to.handleUpgrade(request, socket, head, (client) => to.emit("connection", client, request));
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  opened.push(() => closing(server));

  return {
    port: (server.address() as AddressInfo).port,
    stream: events.past,
    observing: transcripts.all,
    watched: transcripts.past,
    logging: logs.all,
    read: (route: string) => reads.get(route) ?? 0,
    goesAway: () => {
      for (const socket of [...events.all, ...transcripts.all, ...logs.all]) socket.close();
    },
  };
}

/** The first message on every connection, and one more after every drop. */
function resyncing(cursor: number): string {
  return JSON.stringify({
    message: "resync",
    protocol_version: PROTOCOL_VERSION,
    cursor,
    jobs: { jobs: [A_ROW], unreadable: [] },
  });
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
  const stream = await fleet.stream(0);
  stream.send(resyncing(1));
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

it("puts a Drone's question on the Board row, and takes it off again", async () => {
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
  const stream = await fleet.stream(0);
  stream.send(resyncing(1));
  await published.until((state) => state.connection.state === "connected");

  // The Job stays `running` for the whole of this. **The row's own flag is the
  // only thing that moves**, and without it the Needs-you tab — the surface
  // that exists so a question is not missed — never learns there is one.
  stream.send(
    JSON.stringify({
      message: "event",
      cursor: 2,
      event: {
        kind: "job.asking",
        job_id: A_JOB,
        step_id: "implement",
        asking: { question_id: "q1", question: "Which one?", options: ["a", "b"] },
        actor: "drone",
        at: "2026-09-02T19:09:59.615Z",
      },
    }),
  );
  await published.until((state) => state.jobs[0]?.asking === true);

  // The one coming back carries nothing, and that absence is the message.
  stream.send(
    JSON.stringify({
      message: "event",
      cursor: 3,
      event: {
        kind: "job.asking",
        job_id: A_JOB,
        step_id: "implement",
        actor: "human",
        at: "2026-09-02T19:11:00.000Z",
      },
    }),
  );
  await published.until((state) => state.jobs[0]?.asking === false);
});

it("brings back every region of the open Job when Fleet comes back", async () => {
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
  const first = await fleet.stream(0);
  first.send(resyncing(1));
  await published.until((state) => state.connection.state === "connected");

  // **Fleet goes away with nothing open**, so what follows is a person opening
  // a Job while it is down — which is the only way the reads that no event
  // feeds come to be showing a failure at all.
  fleet.goesAway();
  await published.until((state) => state.connection.state === "unreachable");

  // Every region of one Job's screen, opened the way opening the Job opens
  // them: the rail, the machine panel, the history, both halves of the review,
  // and the two sockets.
  await connection.watchJob(A_JOB);
  await connection.readResources(A_JOB);
  await connection.readHistory(A_JOB);
  await connection.readEvidence(A_JOB);
  await connection.readDiff(A_JOB);
  await connection.observeJob(A_JOB);

  // #472 as it was reported: the panel saying Fleet is not answering is the one
  // that stayed that way. Every region is a failure here, and nothing has been
  // read over HTTP, because there was no port to send to.
  await published.until(
    (state) => state.resources.state === "failed" && state.watched.state === "failed",
  );
  for (const route of Object.values(SCREEN)) expect(fleet.read(route)).toBe(0);
  expect(fleet.observing).toHaveLength(0);
  expect(fleet.logging).toHaveLength(0);

  // Fleet comes back on the same port, and the runtime file still names it.
  // Nothing below is a press: the retry reattaches on its own.
  const second = await fleet.stream(1);
  second.send(resyncing(2));

  await published.until(
    (state) =>
      state.watched.state === "read" &&
      state.resources.state === "read" &&
      state.history.state === "read" &&
      state.evidence.state === "read" &&
      state.diff.state === "read",
  );
  // Each read once, together, rather than a screen that half-recovered.
  for (const route of Object.values(SCREEN)) expect(fleet.read(route)).toBe(1);
  // And both per-Job sockets, which are reopened rather than re-read.
  await fleet.watched(0);
  await published.until((state) => state.journalled.state !== "failed");
  expect(fleet.observing).toHaveLength(1);
  expect(fleet.logging).toHaveLength(1);
}, 15_000);

it("does not fetch the patch again when the stream drops events under a live socket", async () => {
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
  const stream = await fleet.stream(0);
  stream.send(resyncing(1));
  await published.until((state) => state.connection.state === "connected");

  await connection.watchJob(A_JOB);
  await connection.readResources(A_JOB);
  await connection.readEvidence(A_JOB);
  await connection.readDiff(A_JOB);
  await published.until((state) => state.diff.state === "read");
  for (const route of [SCREEN.detail, SCREEN.resources, SCREEN.evidence, SCREEN.diff]) {
    expect(fleet.read(route)).toBe(1);
  }

  // A client that could not keep up. Fleet says how many it lost and follows
  // with current state, on the socket that never dropped —
  // `crates/api/src/sockets.rs`.
  stream.send(JSON.stringify({ message: "missed", dropped: 3 }));
  stream.send(resyncing(9));

  // **What events keep current comes back; what a press keeps does not.** HTTP
  // answered throughout, so the claims and the patch are still good — and the
  // patch is the megabyte the split in `crates/ipc/src/work.rs` exists to save.
  await published.until(
    (state) => state.connection.state === "connected" && state.connection.cursor === 9,
  );
  // **Both counters, because the two re-fetches are independent requests.**
  // Waiting on `resources` alone proved nothing about `detail`, and this
  // assertion failed three runs in three when the file ran alone — a test
  // synchronised on one thing and asserting another. #507.
  await published.until(
    () => fleet.read(SCREEN.detail) === 2 && fleet.read(SCREEN.resources) === 2,
  );
  expect(fleet.read(SCREEN.detail)).toBe(2);
  expect(fleet.read(SCREEN.evidence)).toBe(1);
  expect(fleet.read(SCREEN.diff)).toBe(1);
});

it("re-reads the open Job when a Drone comes on or off a step", async () => {
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
  const stream = await fleet.stream(0);
  stream.send(resyncing(1));
  await published.until((state) => state.connection.state === "connected");
  await connection.watchJob(A_JOB);
  await published.until((state) => state.watched.state === "read");

  // **The exit is the first message a Drone's cost can be read back on**: Fleet
  // writes the spend row before publishing it — `crates/fleet/src/allowance.rs`.
  // Both kinds carry the row rather than a `job_id`, so an unmatched one reached
  // the tail as a move about a Job this window had never seen — a full `GET /jobs`
  // apiece, and the open Job never re-read. The count is the cursor because each
  // event is one detail read, the reading the Job opened with being the first.
  const drone = { job: A_ROW, step_id: "scope", drone_id: "01M2224R7V001AYN6FX0", actor: "fleet" };
  for (const [cursor, kind] of [[2, "drone.exited"], [3, "drone.spawned"]] as const) {
    const event = { kind, ...drone, at: "2026-09-02T19:09:59.615Z" };
    stream.send(JSON.stringify({ message: "event", cursor, event }));
    await published.until(() => fleet.read(SCREEN.detail) === cursor);
  }
  expect(fleet.read(SCREEN.detail)).toBe(3);
  expect(fleet.read("/jobs")).toBe(0);
});

/**
 * `#661`: the sweep found this Job's pull request had a new comment, and the
 * comments a person is already looking at are read again without anybody
 * reopening the Job.
 */
it("re-reads the comments on job.remarks_changed, only where a person asked for them", async () => {
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
  const stream = await fleet.stream(0);
  stream.send(resyncing(1));
  await published.until((state) => state.connection.state === "connected");

  // Nobody has asked for the comments yet — `Decide` opens them, and nothing
  // else in Bridge does. The event must have nobody to wake.
  stream.send(
    JSON.stringify({
      message: "event",
      cursor: 2,
      event: {
        kind: "job.remarks_changed",
        job_id: A_JOB,
        actor: "fleet",
        at: "2026-09-11T10:00:00Z",
      },
    }),
  );
  await published.until(
    (state) => state.connection.state === "connected" && state.connection.cursor === 2,
  );
  expect(fleet.read(REMARKS_ROUTE)).toBe(0);

  // The surface that reviews this Job opened its comments, exactly as
  // `onNeedRemarks` does.
  await connection.readRemarks(A_JOB);
  await published.until((state) => state.remarks.state === "read");
  expect(fleet.read(REMARKS_ROUTE)).toBe(1);

  stream.send(
    JSON.stringify({
      message: "event",
      cursor: 3,
      event: {
        kind: "job.remarks_changed",
        job_id: A_JOB,
        actor: "fleet",
        at: "2026-09-11T10:05:00Z",
      },
    }),
  );
  await published.until(() => fleet.read(REMARKS_ROUTE) === 2);

  // **A different Job's pull request is not this Job's read.** Sent, then
  // proved a no-op by a third read of this Job's own that only a further
  // event of its own should cause.
  stream.send(
    JSON.stringify({
      message: "event",
      cursor: 4,
      event: {
        kind: "job.remarks_changed",
        job_id: "01M1HQZAKN001AJ5MT3PT0OTHR",
        actor: "fleet",
        at: "2026-09-11T10:06:00Z",
      },
    }),
  );
  stream.send(
    JSON.stringify({
      message: "event",
      cursor: 5,
      event: {
        kind: "job.remarks_changed",
        job_id: A_JOB,
        actor: "fleet",
        at: "2026-09-11T10:07:00Z",
      },
    }),
  );
  await published.until(() => fleet.read(REMARKS_ROUTE) === 3);
  expect(fleet.read(REMARKS_ROUTE)).toBe(3);
});
