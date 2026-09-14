// `fleetPanelOf`'s `meta` line, on its own — no window, no Fleet socket.
// Bridge/1114: #1088 shipped the panel with `pid · port` and dropped the
// second line the mock draws under it.

import { expect, test } from "vitest";
import type { Connection } from "@armada/protocol";
import type { HealthRead } from "@armada/screens/src/overview-reads";
import type { Statement } from "@armada/shell";
import { fleetPanelOf } from "./left-column";

const STATEMENT: Statement = { headline: "Fleet running", detail: "pid 1 · port 2", next: null };
const NO_HEALTH: HealthRead = { state: "none" };

const CONNECTED: Connection = {
  state: "connected",
  fleet: {
    protocolVersion: { major: 13, minor: 49 },
    pid: 61372,
    port: 40000,
    startedAt: "Mon Sep 14 14:22:06 2026",
  },
  cursor: 0,
  skew: "same",
};

test("a connected Fleet's meta line carries protocol and a live uptime", () => {
  // Two hours and fourteen minutes after `startedAt`.
  const now = Date.parse("Mon Sep 14 14:22:06 2026") + (2 * 60 + 14) * 60_000;
  const panel = fleetPanelOf(CONNECTED, STATEMENT, NO_HEALTH, now);
  expect(panel.meta).toBe("protocol 13.49 · up 2h 14m");
});

test("the meta line ticks: two reads a minute apart move it a minute", () => {
  const first = Date.parse("Mon Sep 14 14:22:06 2026") + 60_000;
  const later = first + 60_000;
  const before = fleetPanelOf(CONNECTED, STATEMENT, NO_HEALTH, first).meta;
  const after = fleetPanelOf(CONNECTED, STATEMENT, NO_HEALTH, later).meta;
  expect(before).not.toBe(after);
});

test("a startedAt that will not parse falls back to protocol alone", () => {
  const unparsable: Connection = {
    ...CONNECTED,
    fleet: { ...CONNECTED.fleet, startedAt: "not a date" },
  };
  const panel = fleetPanelOf(unparsable, STATEMENT, NO_HEALTH, Date.now());
  expect(panel.meta).toBe("protocol 13.49");
});

test("no other connection state carries a meta line", () => {
  const notRunning: Connection = { state: "not_running", absence: { why: "no_runtime_file", path: "~/x" } };
  expect(fleetPanelOf(notRunning, STATEMENT, NO_HEALTH, Date.now()).meta).toBeUndefined();
});
