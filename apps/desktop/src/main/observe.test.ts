// What one Job's Observe socket publishes, against a real socket.
//
// **A `ws` server on a loopback port rather than a mock.** What is under test is
// message handling, and every message arrives through the same three listeners
// the real one is wired with — a stub for `ws` would test the wiring this file
// exists to check. The server here sends what `crates/ipc/src/turn.rs` declares;
// `crates/api/src/tests/observing.rs` is the other half, and holds Fleet's.

import { once } from "node:events";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";
import { WebSocketServer, type WebSocket as Socket } from "ws";

import type { Observed } from "@armada/protocol";
import { ObserveSocket } from "./observe";
import { HOST } from "./runtime-file";

const A_JOB = "01M1HQZAKN001AJ5MT3PT09KKY";

/** One row, as Fleet spells it: the step and the voice sit beside the kind. */
const A_ROW = {
  message: "row",
  ts: "2026-09-02T19:10:04.000Z",
  step: "implement",
  by: "drone",
  event: "said",
  text: "reading the parser",
};

const OPENED = {
  message: "opened",
  protocol_version: { major: 4, minor: 12 },
  job_id: A_JOB,
  live: true,
  skipped: 0,
};

/** Everything one case opens, closed in the order it was opened. */
const opened: (() => void)[] = [];

afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

/**
 * A Fleet serving this Job's turns, and the socket it serves them down.
 *
 * The connection is awaited by the caller, so a frame is never sent before
 * there is somebody to send it to.
 */
async function serving(): Promise<{ port: number; talking: Promise<Socket> }> {
  const server = new WebSocketServer({ host: HOST, port: 0 });
  await once(server, "listening");
  const talking = once(server, "connection").then(([socket]) => socket as Socket);
  opened.push(() => server.close());
  return { port: (server.address() as AddressInfo).port, talking };
}

/** Every state the socket published, and a wait for the one a case is about. */
function watching() {
  const seen: Observed[] = [];
  const wanted: { holds: (state: Observed) => boolean; keep: (state: Observed) => void }[] = [];
  return {
    seen,
    publish(state: Observed): void {
      seen.push(state);
      for (const [at, want] of [...wanted.entries()].reverse()) {
        if (!want.holds(state)) continue;
        wanted.splice(at, 1);
        want.keep(state);
      }
    },
    /** The first published state that answers this, past or future. */
    until(holds: (state: Observed) => boolean): Promise<Observed> {
      const already = seen.find(holds);
      if (already !== undefined) return Promise.resolve(already);
      return new Promise((keep) => wanted.push({ holds, keep }));
    },
  };
}

describe("a transcript socket that stops", () => {
  it("keeps the rows it already had when it breaks", async () => {
    const fleet = await serving();
    const published = watching();
    const turns = new ObserveSocket((state) => published.publish(state));
    opened.push(() => turns.close());

    turns.open(fleet.port, A_JOB);
    const fleetSide = await fleet.talking;
    fleetSide.send(JSON.stringify(OPENED));
    fleetSide.send(JSON.stringify(A_ROW));
    await published.until((state) => "turns" in state && state.turns.rows.length === 1);

    // A message this Bridge cannot read. **The defect**: one of these emptied a
    // log that was full a moment before, so a reader lost the step's whole
    // history to a frame that was never about the rows.
    fleetSide.send("{ not json");
    const broke = await published.until((state) => state.state === "failed");

    expect(broke).toMatchObject({ state: "failed", jobId: A_JOB });
    expect("turns" in broke && broke.turns.rows.length).toBe(1);
  });

  it("says why it closed and keeps the rows", async () => {
    const fleet = await serving();
    const published = watching();
    const turns = new ObserveSocket((state) => published.publish(state));
    opened.push(() => turns.close());

    turns.open(fleet.port, A_JOB);
    const fleetSide = await fleet.talking;
    fleetSide.send(JSON.stringify(OPENED));
    fleetSide.send(JSON.stringify(A_ROW));
    fleetSide.send(JSON.stringify({ message: "closed", because: "drone_ended" }));

    const ended = await published.until((state) => state.state === "ended");
    expect(ended).toMatchObject({ state: "ended", because: "drone_ended" });
    expect("turns" in ended && ended.turns.rows.length).toBe(1);
    // The socket is let go on `closed`, which is what makes the reopen on the
    // next event about this Job the thing that resumes it — `connection.ts`.
    expect(turns.attached()).toBe(false);
  });
});
