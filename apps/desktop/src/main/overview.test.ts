// Overview's reads, sent against a Fleet that records what arrived: health once, and drift for each
// repository in the scope, each named by its own Manifest.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import type { RepositorySummary } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { OverviewReads } from "./overview";
import { Picked } from "./picked";

const manifest = (id: string, root: string) => ({ id, repository: id, path: `${root}/armada.yml`, records_root: `/records/${id}`, version: 1, checks: [] });
const ARMADA: RepositorySummary = { root: "/Users/user/armada", records_root: "/records/armada", manifest: manifest("armada", "/Users/user/armada") };
const SHOP: RepositorySummary = { root: "/Users/user/shop", records_root: "/records/shop", manifest: manifest("shop-01", "/Users/user/shop") };
const SCRATCH: RepositorySummary = { root: "/Users/user/scratch", records_root: "/records/scratch" };

const HEALTH = {
  probes: [{ module: "Fleet", outcome: "pass", detail: "answering" }],
  not_probed: [],
  helm_action_authority: "acting",
};
const DRIFT = { path: "armada.yml", checkout: "/Users/user/armada", declarations: [] };

let listening: Server | null = null;
afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise<void>((done) => server.close(() => done()));
});

async function fleet(asked: string[]): Promise<number> {
  const server = createServer((request, response) => {
    const url = request.url ?? "";
    asked.push(url);
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify(url === "/health" ? HEALTH : DRIFT));
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  return (server.address() as AddressInfo).port;
}

function reading(port: number | null, picked: Picked) {
  let held: Partial<BridgeState> = {};
  const overview = new OverviewReads({ publish: (change) => (held = { ...held, ...change }), picked, port: () => port });
  return { overview, held: () => held };
}

function served(): Picked {
  const picked = new Picked();
  picked.hold([ARMADA, SHOP, SCRATCH]);
  return picked;
}

describe("Overview's reads", () => {
  it("ask nothing until a surface wants them", async () => {
    const asked: string[] = [];
    const port = await fleet(asked);
    const { overview, held } = reading(port, served());
    await overview.again(port);
    expect(asked).toEqual([]);
    expect(held()).toEqual({});
  });

  it("read health once, and drift for every repository on All by its own Manifest", async () => {
    const asked: string[] = [];
    const port = await fleet(asked);
    const { overview, held } = reading(port, served());
    await overview.watch(true);
    expect(asked.sort()).toEqual(["/health", "/manifest/drift?manifest_id=armada", "/manifest/drift?manifest_id=shop-01"]);
    expect(held().health).toEqual({ state: "read", health: HEALTH });
    expect(held().drifts).toEqual({
      state: "held",
      repositories: [
        { root: ARMADA.root, drift: { state: "read", drift: DRIFT } },
        { root: SHOP.root, drift: { state: "read", drift: DRIFT } },
        { root: SCRATCH.root, drift: { state: "failed", outcome: { ok: false, why: "not_set_up" } } },
      ],
    });
  });

  it("read only the picked repository's drift on a pick", async () => {
    const asked: string[] = [];
    const port = await fleet(asked);
    const picked = served();
    picked.pick(SHOP.root);
    const { overview, held } = reading(port, picked);
    await overview.watch(true);
    expect(asked.sort()).toEqual(["/health", "/manifest/drift?manifest_id=shop-01"]);
    expect(held().drifts).toEqual({ state: "held", repositories: [{ root: SHOP.root, drift: { state: "read", drift: DRIFT } }] });
  });

  it("drop what they held when the surface closes, and an answer that lands after", async () => {
    const asked: string[] = [];
    const port = await fleet(asked);
    const { overview, held } = reading(port, served());
    const opened = overview.watch(true);
    await overview.watch(false);
    await opened;
    expect(held()).toEqual({ health: { state: "none" }, drifts: { state: "none" } });
  });

  it("fail both without a connection, naming every repository", async () => {
    const { overview, held } = reading(null, served());
    await overview.watch(true);
    const failed = { state: "failed", outcome: { ok: false, why: "not_connected" } };
    expect(held().health).toEqual(failed);
    expect(held().drifts).toEqual({
      state: "held",
      repositories: [ARMADA, SHOP, SCRATCH].map(({ root }) => ({ root, drift: failed })),
    });
  });
});
