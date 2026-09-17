// `fleetPanelOf`'s rows, on their own — no window, no Fleet socket.
// Bridge/1114: #1088 shipped the panel with `pid · port` and dropped the
// second line the mock draws under it. Settled 2026-09-17: the two lines are
// now four rows, and a state draws only the rows it has a value for.

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

function pairs(panel: ReturnType<typeof fleetPanelOf>): [string, string][] | undefined {
  return panel.rows?.map((row) => [row.label, row.value]);
}

test("a connected Fleet reads four rows: pid, port, protocol and a live uptime", () => {
  // Two hours and fourteen minutes after `startedAt`.
  const now = Date.parse("Mon Sep 14 14:22:06 2026") + (2 * 60 + 14) * 60_000;
  const panel = fleetPanelOf(CONNECTED, STATEMENT, NO_HEALTH, now, null);
  expect(pairs(panel)).toEqual([
    ["pid", "61372"],
    ["port", "40000"],
    ["protocol", "13.49"],
    ["up", "2h 14m"],
  ]);
  expect(panel.detail).toBeUndefined();
});

test("the up row ticks: two reads a minute apart move it a minute", () => {
  const first = Date.parse("Mon Sep 14 14:22:06 2026") + 60_000;
  const later = first + 60_000;
  const before = pairs(fleetPanelOf(CONNECTED, STATEMENT, NO_HEALTH, first, null));
  const after = pairs(fleetPanelOf(CONNECTED, STATEMENT, NO_HEALTH, later, null));
  expect(before).not.toEqual(after);
});

test("a startedAt that will not parse drops the up row rather than drawing it blank", () => {
  const unparsable: Connection = {
    ...CONNECTED,
    fleet: { ...CONNECTED.fleet, startedAt: "not a date" },
  };
  const panel = fleetPanelOf(unparsable, STATEMENT, NO_HEALTH, Date.now(), null);
  expect(panel.rows?.map((row) => row.label)).toEqual(["pid", "port", "protocol"]);
});

test("a Fleet ahead of Bridge keeps its rows and names both versions under them", () => {
  const ahead: Connection = { ...CONNECTED, skew: "fleet_ahead" };
  const panel = fleetPanelOf(ahead, STATEMENT, NO_HEALTH, Date.now(), null);
  expect(panel.detail).toMatch(/^Fleet 13\.49, Bridge /);
});

test("an unreachable Fleet reads pid and port, and how long it has been silent", () => {
  const now = 100_000;
  const unreachable: Connection = { state: "unreachable", fleet: CONNECTED.fleet, detail: "", sinceMs: now - 20_000 };
  const panel = fleetPanelOf(unreachable, STATEMENT, NO_HEALTH, now, now - 4_000);
  expect(pairs(panel)).toEqual([
    ["pid", "61372"],
    ["port", "40000"],
  ]);
  expect(panel.detail).toBe("alive, no answer for 20s · last read 4s ago");
});

test("a Fleet that is not running has no rows, only what the runtime file says", () => {
  const notRunning: Connection = { state: "not_running", absence: { why: "no_runtime_file", path: "~/x" } };
  const said: Statement = { headline: "Fleet is not running", detail: "no runtime file at ~/x", next: null };
  const panel = fleetPanelOf(notRunning, said, NO_HEALTH, Date.now(), null);
  expect(panel.rows).toBeUndefined();
  expect(panel.detail).toBe("no runtime file at ~/x");
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
