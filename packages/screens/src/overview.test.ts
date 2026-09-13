// Overview's readings, case by case: the machine's three tiles ignore a pick, the scope's two follow it.

import { describe, expect, it } from "vitest";

import type { Connection, ManifestDriftRead, RepositorySummary } from "@armada/protocol";
import { connectedTo, PROTOCOL_VERSION } from "@armada/protocol";
import { job } from "./fixtures/build/base";
import { doctorReading, driftReading, dronesReading, fleetReading, queuedIn, queuedReading } from "./overview";
import type { FleetHealth, RepositoryDrift } from "./overview-reads";

const NOW = Date.parse("2026-09-13T12:00:00Z");
const FLEET = { protocolVersion: PROTOCOL_VERSION, pid: 4242, port: 7878, startedAt: "2026-09-13T11:00:00Z" };
const CONNECTED: Connection = connectedTo(FLEET, 1);

const manifest = (id: string, root: string) => ({ id, repository: id, path: `${root}/armada.yml`, records_root: `/records/${id}`, version: 1, checks: [] });
const ARMADA: RepositorySummary = { root: "/Users/user/armada", records_root: "/records/armada", manifest: manifest("armada", "/Users/user/armada") };
const SHOP: RepositorySummary = { root: "/Users/user/shop", records_root: "/records/shop", manifest: manifest("shop", "/Users/user/shop") };
const SCRATCH: RepositorySummary = { root: "/Users/user/scratch", records_root: "/records/scratch" };

const gone = { section: "checks", name: "lint", key: "run", run: "scripts/lint.sh", drift: { verdict: "gone" as const, missing: ["scripts/lint.sh"] }, unfollowed: [] };
const current = { section: "checks", name: "test", key: "run", run: "cargo test", drift: { verdict: "current" as const, checked: 1 }, unfollowed: [] };
const read = (...declarations: (typeof gone | typeof current)[]): ManifestDriftRead => ({
  state: "read",
  drift: { path: "armada.yml", checkout: "/Users/user/armada", declarations },
});
const unset: ManifestDriftRead = { state: "failed", outcome: { ok: false, why: "not_set_up" } };

describe("the Fleet tile", () => {
  it("reads the connection's own statement, in the status bar's three hues", () => {
    expect(fleetReading(CONNECTED, NOW, null)).toMatchObject({ value: "Fleet running", tone: "completed-success" });
    const unreachable: Connection = { state: "unreachable", fleet: FLEET, detail: "", sinceMs: NOW - 20_000 };
    expect(fleetReading(unreachable, NOW, null)).toMatchObject({ value: "Fleet unreachable", tone: "awaiting-review" });
    const down: Connection = { state: "not_running", absence: { why: "no_runtime_file", path: "fleet.json" } };
    expect(fleetReading(down, NOW, null)).toMatchObject({ value: "Fleet is not running", tone: "completed-failed" });
    expect(fleetReading({ state: "reading" }, NOW, null).tone).toBeUndefined();
  });
});

describe("the Doctor tile", () => {
  const health = (...outcomes: [string, string][]): FleetHealth => ({
    probes: outcomes.map(([module, outcome]) => ({ module, outcome, detail: "read" })),
    not_probed: [{ owner: "adapters", because: "Doctor is not built" }],
  });

  it("says the worst of Doctor's words and names every module that did not pass", () => {
    const reading = doctorReading({ state: "read", health: health(["Fleet", "pass"], ["SQLite", "fail"], ["Manifest", "warn"]) });
    expect(reading).toMatchObject({ value: "fail", tone: "completed-failed", detail: "SQLite: fail · Manifest: warn" });
  });

  it("names what was probed when all of it passes", () => {
    const reading = doctorReading({ state: "read", health: health(["Fleet", "pass"], ["SQLite", "pass"]) });
    expect(reading).toMatchObject({ value: "pass", tone: "completed-success", detail: "Fleet, SQLite" });
  });

  it("draws a word it does not know as itself, with no hue", () => {
    const reading = doctorReading({ state: "read", health: health(["Docker", "skipped"]) });
    expect(reading).toMatchObject({ value: "skipped", detail: "Docker: skipped" });
    expect(reading.tone).toBeUndefined();
  });

  it("stands in until it is read", () => {
    expect(doctorReading({ state: "reading" }).value).toBeUndefined();
  });
});

describe("the Drones tile", () => {
  it("names the hold only while something is queued", () => {
    const full = { bound: 2, occupied: 2, held_by: "memory" };
    expect(dronesReading(CONNECTED, full, 1)).toMatchObject({ value: "2 of 2", detail: "waiting on memory" });
    expect(dronesReading(CONNECTED, full, 0)).toMatchObject({ detail: "None free" });
    expect(dronesReading(CONNECTED, { bound: 4, occupied: 1 }, 0)).toMatchObject({ detail: "3 free" });
  });

  it("stands in while connected and unread, and says not read otherwise", () => {
    expect(dronesReading(CONNECTED, null, 0).value).toBeUndefined();
    expect(dronesReading({ state: "reading" }, null, 0).value).toBe("Not read");
  });
});

describe("the Queued tile", () => {
  const jobs = [
    job("queued", { id: "a", owner_manifest_id: "armada" }),
    job("queued", { id: "b", owner_manifest_id: "shop" }),
    job("running", { id: "c", owner_manifest_id: "shop" }),
  ];

  it("counts every repository's on All, and only the pick's on a pick", () => {
    expect(queuedIn(jobs, null).map((one) => one.id)).toEqual(["a", "b"]);
    expect(queuedIn(jobs, SHOP).map((one) => one.id)).toEqual(["b"]);
  });

  it("says where they are", () => {
    const served = [ARMADA, SHOP, SCRATCH];
    expect(queuedReading(queuedIn(jobs, null), served, null)).toMatchObject({ value: "2", detail: "In 2 of 3 repositories" });
    expect(queuedReading(queuedIn(jobs, SHOP), served, SHOP)).toMatchObject({ value: "1", detail: "shop" });
    expect(queuedReading([], served, null)).toMatchObject({ value: "0", detail: "Across 3 repositories" });
    expect(queuedReading(queuedIn(jobs, null), [ARMADA], null).detail).toBeUndefined();
  });
});

describe("the Manifest drift tile", () => {
  const held = (...repositories: RepositoryDrift[]) => ({ state: "held" as const, repositories });

  it("says how many repositories are behind on All, and which could not be asked", () => {
    const reading = driftReading(
      held({ root: ARMADA.root, drift: read(gone, current) }, { root: SHOP.root, drift: read(current) }, { root: SCRATCH.root, drift: unset }),
      null,
    );
    expect(reading).toMatchObject({ value: "1 behind", tone: "notice-caution", detail: "1 of 2 repositories · 1 not set up" });
  });

  it("reads current, without a hue, when nothing on All is behind", () => {
    const reading = driftReading(held({ root: SHOP.root, drift: read(current) }), null);
    expect(reading).toMatchObject({ value: "Current", detail: "1 repository, none behind" });
    expect(reading.tone).toBeUndefined();
  });

  it("counts the picked repository's lines on a pick", () => {
    const drifts = held({ root: ARMADA.root, drift: read(gone, gone, current) }, { root: SHOP.root, drift: read(current) });
    expect(driftReading(drifts, ARMADA)).toMatchObject({ value: "2 behind", detail: "Of 3 lines armada.yml names" });
    expect(driftReading(drifts, SHOP)).toMatchObject({ value: "Current", detail: "1 line, nothing they name is gone" });
    expect(driftReading(held({ root: SCRATCH.root, drift: unset }), SCRATCH)).toMatchObject({ value: "Not set up" });
  });

  it("stands in until anything is read", () => {
    expect(driftReading({ state: "none" }, null).value).toBeUndefined();
    expect(driftReading(held({ root: ARMADA.root, drift: { state: "reading" } }), null).value).toBeUndefined();
  });
});
