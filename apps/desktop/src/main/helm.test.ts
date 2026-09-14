// One repository's Helm conversation, against a real socket and a real HTTP
// server on the same port — `observe.ts`'s own reason: what is under test is
// message handling and the two requests, through the same listeners the real
// connection is wired with.

import { once } from "node:events";
import { createServer, type IncomingMessage, type Server } from "node:http";
import type { Socket } from "node:net";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";
import { WebSocketServer, type WebSocket as Client } from "ws";

import type { HelmContext, HelmThread, RepositorySummary } from "@armada/protocol";
import { HelmConnection, HelmSocket } from "./helm";

const MANIFEST = "01M2ARMADA0000000000000000";
const OPENED = { message: "opened", protocol_version: { major: 13, minor: 38 }, manifest_id: MANIFEST, replying: false, skipped: 0 };
const A_REPLY = { message: "row", ts: "2026-09-13T10:00:00.000Z", event: "said", text: "Job 12 is waiting on a command." };
const ENDED = { message: "row", ts: "2026-09-13T10:00:01.000Z", event: "ended", turns: 3, cost_micros: 12_000, refusals: 0 };

const opened: (() => void)[] = [];
afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

/**
 * A Fleet serving `/helm/observe`, `/helm/ask` and `/helm/start_fresh` on one
 * port. `bodies` collects each request's raw text, parallel to `requests`.
 */
async function serving(
  requests: { url: string; method: string }[] = [],
  bodies: string[] = [],
): Promise<{ port: number; helmSide: () => Promise<Client>; server: Server }> {
  const socket = new WebSocketServer({ noServer: true });
  const server = createServer((request, response) => {
    requests.push({ url: request.url ?? "", method: request.method ?? "" });
    const chunks: Buffer[] = [];
    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("end", () => {
      bodies.push(Buffer.concat(chunks).toString("utf8"));
      response.writeHead(202, { "content-type": "application/json" });
      response.end(JSON.stringify({ manifest_id: MANIFEST, replying: true, resumes: true }));
    });
  });
  server.on("upgrade", (request: IncomingMessage, raw: Socket, head: Buffer) => {
    socket.handleUpgrade(request, raw, head, (client) => socket.emit("connection", client, request));
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  opened.push(() => server.close());
  return {
    port: (server.address() as AddressInfo).port,
    helmSide: () => once(socket, "connection").then(([client]) => client as Client),
    server,
  };
}

function watching() {
  const seen: HelmThread[] = [];
  const wanted: { holds: (state: HelmThread) => boolean; keep: (state: HelmThread) => void }[] = [];
  return {
    seen,
    publish(state: HelmThread): void {
      seen.push(state);
      for (const [at, want] of [...wanted.entries()].reverse()) {
        if (!want.holds(state)) continue;
        wanted.splice(at, 1);
        want.keep(state);
      }
    },
    until(holds: (state: HelmThread) => boolean): Promise<HelmThread> {
      const already = seen.find(holds);
      if (already !== undefined) return Promise.resolve(already);
      return new Promise((keep) => wanted.push({ holds, keep }));
    },
  };
}

describe("Helm's socket", () => {
  it("folds a reply into an open thread, with its cost", async () => {
    const fleet = await serving();
    const published = watching();
    const helm = new HelmSocket((state) => published.publish(state));
    opened.push(() => helm.close());

    helm.open(fleet.port, MANIFEST);
    const fleetSide = await fleet.helmSide();
    fleetSide.send(JSON.stringify(OPENED));
    fleetSide.send(JSON.stringify(A_REPLY));
    fleetSide.send(JSON.stringify(ENDED));

    const settled = await published.until(
      (state) => state.state === "open" && !state.replying && state.items.length === 2,
    );
    expect(settled).toMatchObject({ state: "open", manifestId: MANIFEST, replying: false });
  });

  it("reopens onto the fresh, empty conversation a start-fresh leaves behind", async () => {
    const fleet = await serving();
    const published = watching();
    const helm = new HelmSocket((state) => published.publish(state));
    opened.push(() => helm.close());

    helm.open(fleet.port, MANIFEST);
    const first = await fleet.helmSide();
    first.send(JSON.stringify(OPENED));
    first.send(JSON.stringify(A_REPLY));
    await published.until((state) => state.state === "open" && state.items.length === 1);

    const reopened = fleet.helmSide();
    first.send(JSON.stringify({ message: "closed", because: "started_fresh" }));
    await published.until((state) => state.state === "cleared");

    const second = await reopened;
    second.send(JSON.stringify({ ...OPENED, skipped: 0 }));
    const empty = await published.until((state) => state.state === "open");
    expect(empty).toMatchObject({ state: "open", manifestId: MANIFEST, items: [] });
  });

  it("keeps what arrived when the connection breaks", async () => {
    const fleet = await serving();
    const published = watching();
    const helm = new HelmSocket((state) => published.publish(state));
    opened.push(() => helm.close());

    helm.open(fleet.port, MANIFEST);
    const fleetSide = await fleet.helmSide();
    fleetSide.send(JSON.stringify(OPENED));
    fleetSide.send(JSON.stringify(A_REPLY));
    await published.until((state) => state.state === "open" && state.items.length === 1);

    fleetSide.send("{ not json");
    const broke = await published.until((state) => state.state === "failed");
    expect(broke).toMatchObject({ state: "failed", manifestId: MANIFEST });
    expect("items" in broke && broke.items.length).toBe(1);
  });

  it("says Fleet is not connected without a port, rather than trying to reach one", () => {
    const published = watching();
    const helm = new HelmSocket((state) => published.publish(state));
    opened.push(() => helm.close());

    helm.open(null, MANIFEST);
    expect(published.seen.at(-1)).toMatchObject({ state: "failed", manifestId: MANIFEST });
  });

  it("asks and starts fresh against the manifest it is watching", async () => {
    const requests: { url: string; method: string }[] = [];
    const fleet = await serving(requests);
    const helm = new HelmSocket(() => {});
    opened.push(() => helm.close());
    helm.open(fleet.port, MANIFEST);
    await fleet.helmSide();

    expect(await helm.askHelm(MANIFEST, "Why did job 12 stall?")).toEqual({ ok: true });
    expect(await helm.startFresh(MANIFEST)).toEqual({ ok: true });
    expect(requests).toEqual([
      { url: `/helm/ask?manifest_id=${MANIFEST}`, method: "POST" },
      { url: `/helm/start_fresh?manifest_id=${MANIFEST}`, method: "POST" },
    ]);
  });

  // #1075: Bridge sends where the person is with every ask.
  it("sends the context with an ask, and sends none where none was given", async () => {
    const bodies: string[] = [];
    const fleet = await serving([], bodies);
    const helm = new HelmSocket(() => {});
    opened.push(() => helm.close());
    helm.open(fleet.port, MANIFEST);
    await fleet.helmSide();

    const context: HelmContext = { screen: "job_detail", chip: "01JOB0000000000000000000A" };
    expect(await helm.askHelm(MANIFEST, "what is this stuck on?", context)).toEqual({ ok: true });
    expect(await helm.askHelm(MANIFEST, "and this one?")).toEqual({ ok: true });

    expect(JSON.parse(bodies[0]!)).toEqual({ text: "what is this stuck on?", context });
    expect(JSON.parse(bodies[1]!)).toEqual({ text: "and this one?" });
  });
});

const OTHER = "01M2SHOP00000000000000000A";
const REPOSITORIES: RepositorySummary[] = [
  { root: "/repos/armada", records_root: "/records/armada", manifest: { id: MANIFEST } as never },
  { root: "/repos/shop", records_root: "/records/shop", manifest: { id: OTHER } as never },
];

describe("which repository Helm answers for", () => {
  it("follows a pick made after a point", () => {
    const helm = new HelmConnection({ publish: () => {}, port: () => null });
    helm.onRepositoriesChanged(REPOSITORIES);
    helm.point(OTHER);
    helm.onPicked(REPOSITORIES[0]!.root);
    expect(helm.target()).toBe(MANIFEST);
  });

  // #948's bug: Discuss on another card was ignored whenever a specific
  // repository was already picked, because the pick was given a fixed
  // priority over a point rather than the two being compared by which was
  // most recent.
  it("lets Discuss point Helm at another repository while one stays picked", () => {
    const helm = new HelmConnection({ publish: () => {}, port: () => null });
    helm.onRepositoriesChanged(REPOSITORIES);
    helm.onPicked(REPOSITORIES[0]!.root);
    expect(helm.target()).toBe(MANIFEST);

    helm.point(OTHER);
    expect(helm.target()).toBe(OTHER);
  });

  it("a pick made after Discuss is the most recent act again", () => {
    const helm = new HelmConnection({ publish: () => {}, port: () => null });
    helm.onRepositoriesChanged(REPOSITORIES);
    helm.point(OTHER);
    expect(helm.target()).toBe(OTHER);

    helm.onPicked(REPOSITORIES[0]!.root);
    expect(helm.target()).toBe(MANIFEST);
  });

  it("moving to All is not itself an act — the most recent pick or point still stands", () => {
    const helm = new HelmConnection({ publish: () => {}, port: () => null });
    helm.onRepositoriesChanged(REPOSITORIES);
    helm.onPicked(REPOSITORIES[0]!.root);
    helm.onPicked(null);
    expect(helm.target()).toBe(MANIFEST);
  });

  it("falls back to the first repository with a Manifest where nothing has talked to it yet", () => {
    const helm = new HelmConnection({ publish: () => {}, port: () => null });
    helm.onRepositoriesChanged(REPOSITORIES);
    helm.onPicked(null);
    expect(helm.target()).toBe(MANIFEST);
  });
});
