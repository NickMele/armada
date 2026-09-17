// `fleetPanelOf`'s `meta` line, on its own — no window, no Fleet socket.
// Bridge/1114: #1088 shipped the panel with `pid · port` and dropped the
// second line the mock draws under it.

import { expect, test } from "vitest";
import type { StatRow } from "@armada/components";
import type { Connection, Declaration, FleetCapacity, JobSummary } from "@armada/protocol";
import type { DriftsRead, HealthRead } from "@armada/screens/src/overview-reads";
import type { Statement } from "@armada/shell";
import { fleetPanelOf, statsOf } from "./left-column";

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

// `statsOf`'s dots, Bridge/1263: every row a hue of its own, dim until
// something is counted, running or behind. design-system.md → Stats panel.

const NO_DRIFTS: DriftsRead = { state: "none" };

function dotsOf(rows: StatRow[]): Record<string, [string, boolean]> {
  return Object.fromEntries(rows.map((row) => [row.id, [row.hue, row.idle]]));
}

function job(status: string): JobSummary {
  return { status } as JobSummary;
}

function driftOf(gone: number): DriftsRead {
  const line = (verdict: string) => ({ drift: { verdict } }) as unknown as Declaration;
  const declarations = [line("current"), ...Array.from({ length: gone }, () => line("gone"))];
  return {
    state: "held",
    repositories: [{ root: "/r", drift: { state: "read", drift: { path: "armada.yml", checkout: "/r", declarations } } }],
  };
}

test("on a quiet day every row has a dim dot in its own hue", () => {
  const capacity: FleetCapacity = { bound: 2, occupied: 0 };
  expect(dotsOf(statsOf(CONNECTED, [], capacity, [], null, driftOf(0)))).toEqual({
    approval: ["status-awaiting-review", true],
    review: ["status-awaiting-review", true],
    escalated: ["status-escalated", true],
    jobs: ["status-not-started", true],
    drones: ["stat-drones", true],
    manifest: ["stat-manifest-current", true],
  });
});

test("a row with something waiting, a Drone running or a Manifest behind draws full", () => {
  const jobs = [job("awaiting_approval"), job("awaiting_review"), job("escalated")];
  const capacity: FleetCapacity = { bound: 4, occupied: 2 };
  expect(dotsOf(statsOf(CONNECTED, jobs, capacity, [], null, driftOf(1)))).toEqual({
    approval: ["status-awaiting-review", false],
    review: ["status-awaiting-review", false],
    escalated: ["status-escalated", false],
    jobs: ["status-not-started", false],
    drones: ["stat-drones", false],
    manifest: ["notice-caution", false],
  });
});

test("Drones and Manifest stay dim while neither has been read", () => {
  const dots = dotsOf(statsOf(CONNECTED, [], null, [], null, NO_DRIFTS));
  expect(dots.drones).toEqual(["stat-drones", true]);
  expect(dots.manifest).toEqual(["stat-manifest-current", true]);
});
