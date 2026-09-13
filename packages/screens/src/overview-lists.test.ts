// Overview's lists, case by case. `board.test.ts` already pins `sectionsOf`'s own rules; these
// tests pin only what `overview-lists.ts` adds on top of it — the scope, the fold, and Done left off.

import { describe, expect, it } from "vitest";

import type { RepositorySummary } from "@armada/protocol";
import { job } from "./fixtures/build/base";
import { overviewListsOf } from "./overview-lists";

const manifest = (id: string) => ({ id, repository: id, path: `${id}/armada.yml`, records_root: `/records/${id}`, version: 1, checks: [] });
const SHOP: RepositorySummary = { root: "/Users/user/shop", records_root: "/records/shop", manifest: manifest("shop") };

describe("overviewListsOf", () => {
  it("draws needs-you, running and queued, in that order, and leaves Done off", () => {
    const jobs = [
      job("completed_success", { id: "done" }),
      job("queued", { id: "q" }),
      job("running", { id: "r" }),
      job("awaiting_approval", { id: "ny" }),
    ];
    expect(overviewListsOf(jobs, null).sections.map((section) => section.id)).toEqual([
      "needs-you",
      "running",
      "queued",
    ]);
  });

  it("scopes to the pick, and to every repository on All", () => {
    const jobs = [
      job("queued", { id: "a", owner_manifest_id: "armada" }),
      job("queued", { id: "b", owner_manifest_id: "shop" }),
    ];
    expect(overviewListsOf(jobs, null).sections[0]?.jobs.map((one) => one.id)).toEqual(["a", "b"]);
    expect(overviewListsOf(jobs, SHOP).sections[0]?.jobs.map((one) => one.id)).toEqual(["b"]);
  });

  it("draws no section with nothing in it", () => {
    expect(overviewListsOf([job("running")], null).sections.map((section) => section.id)).toEqual(["running"]);
  });

  it("folds a redispatch chain to its live member", () => {
    const jobs = [
      job("killed", { id: "first", created_at: "2026-09-10T00:00:00Z" }),
      job("running", { id: "second", created_at: "2026-09-11T00:00:00Z", redispatched_from: "first" }),
    ];
    const { sections, dispatch } = overviewListsOf(jobs, null);
    expect(sections.find((section) => section.id === "running")?.jobs.map((one) => one.id)).toEqual(["second"]);
    expect(dispatch.get("second")).toMatchObject({ nth: 2, of: 2 });
  });

  it("names a Job the row shape cannot draw, rather than placing it in Other", () => {
    const jobs = [job("not_a_status_the_registry_has", { id: "x" })];
    const { sections, undrawable } = overviewListsOf(jobs, null);
    expect(sections.find((section) => section.id === "other")).toBeUndefined();
    expect(undrawable.map((one) => one.id)).toEqual(["x"]);
  });

  it("sorts oldest first within a section, the Board's own default", () => {
    const jobs = [
      job("running", { id: "newer", created_at: "2026-09-12T00:00:00Z" }),
      job("running", { id: "older", created_at: "2026-09-01T00:00:00Z" }),
    ];
    expect(overviewListsOf(jobs, null).sections[0]?.jobs.map((one) => one.id)).toEqual(["older", "newer"]);
  });
});
