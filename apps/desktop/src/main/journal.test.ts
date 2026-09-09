// What one Job's Journal socket publishes, against a real socket.
//
// **A `ws` server on a loopback port rather than a mock**, for `observe.test.ts`'s
// reason: what is under test is the three listeners the real socket is wired
// with, and a stub for `ws` would test the wiring instead of the handling. The
// server here sends what `crates/ipc/src/journal.rs` declares.

import { once } from "node:events";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";
import { WebSocketServer, type WebSocket as Socket } from "ws";

import type { Journalled } from "@armada/protocol";
import { JournalSocket } from "./journal";
import { HOST } from "./runtime-file";

const A_JOB = "01M1HQZAKN001AJ5MT3PT09KKY";

/** Everything one case opens, closed in the order it was opened. */
const opened: (() => void)[] = [];

afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

/** A Fleet serving this Job's log, and the socket it serves it down. */
async function serving(): Promise<{ port: number; talking: Promise<Socket> }> {
  const server = new WebSocketServer({ host: HOST, port: 0 });
  await once(server, "listening");
  const talking = once(server, "connection").then(([socket]) => socket as Socket);
  opened.push(() => server.close());
  return { port: (server.address() as AddressInfo).port, talking };
}

describe("a log socket that stops", () => {
  // **The handshake is the case that crashed.** A Job opened while Fleet is
  // still connecting leaves ws no connection to close, so it aborts and reports
  // the abort as an `error` a tick later. With every listener already gone,
  // Node threw it, and the main process went down with it.
  it("closes while the handshake is still in flight, and nothing is thrown", async () => {
    const fleet = await serving();
    const log = new JournalSocket(() => {});
    opened.push(() => log.close());

    log.open(fleet.port, A_JOB);
    log.close();

    await new Promise((settle) => setTimeout(settle, 20));
    expect(log.attached()).toBe(false);
  });

  // **The socket is Fleet's too.** A frame this Bridge cannot read used to
  // leave the connection open with every listener removed — nothing to notice
  // it, and Fleet holding a socket for a pane that had stopped reading. The
  // assertion is on the server's side, because this Bridge reporting itself
  // closed is exactly what it did before.
  it("lets the connection go when it cannot read what arrived", async () => {
    const fleet = await serving();
    const log = new JournalSocket(() => {});
    opened.push(() => log.close());

    log.open(fleet.port, A_JOB);
    const fleetSide = await fleet.talking;
    const closed = once(fleetSide, "close");
    fleetSide.send("{ not json");

    await closed;
    expect(log.attached()).toBe(false);
  });

  // The same abort, reached the way the crash actually was: a second `open`
  // closes the first, and the first had not finished connecting.
  it("reopens on another Job while the first is still connecting", async () => {
    const fleet = await serving();
    const seen: Journalled[] = [];
    const log = new JournalSocket((state) => seen.push(state));
    opened.push(() => log.close());

    log.open(fleet.port, A_JOB);
    log.open(fleet.port, "01M1HQZAKN001AJ5MT3PT09KKZ");

    await new Promise((settle) => setTimeout(settle, 20));
    expect(seen.every((state) => state.state !== "failed")).toBe(true);
  });
});
