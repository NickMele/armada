// `recent.ts`'s arithmetic: which readings mark a row, and how far a mark has gone.

import { describe, expect, it } from "vitest";

import { job } from "./fixtures/build/base";
import { decaying, observe } from "./recent";

const DECAY = 45_000;

describe("observe", () => {
  it("marks nothing on the first reading", () => {
    const seen = observe(null, [job("running", { id: "a" }), job("escalated", { id: "b" })], 0, DECAY);
    expect(seen.changedAt.size).toBe(0);
  });

  it("marks a Job whose status changed, at the moment it was seen", () => {
    const first = observe(null, [job("running", { id: "a" })], 0, DECAY);
    const next = observe(first, [job("awaiting_review", { id: "a" })], 1_000, DECAY);
    expect(next.changedAt.get("a")).toBe(1_000);
  });

  it("does not mark a Job seen for the first time", () => {
    const first = observe(null, [job("running", { id: "a" })], 0, DECAY);
    const next = observe(first, [job("running", { id: "a" }), job("awaiting_review", { id: "b" })], 1_000, DECAY);
    expect(next.changedAt.size).toBe(0);
  });

  it("marks a queued Job whose reason moved, because the badge did", () => {
    const first = observe(null, [job("queued", { id: "a", queued_reason: "waiting_on_resources" })], 0, DECAY);
    const next = observe(first, [job("queued", { id: "a", queued_reason: "over_budget" })], 1_000, DECAY);
    expect(next.changedAt.get("a")).toBe(1_000);
  });

  it("keeps a mark across an unchanged reading, and drops it once decayed", () => {
    const first = observe(null, [job("running", { id: "a" })], 0, DECAY);
    const changed = observe(first, [job("completed_success", { id: "a" })], 1_000, DECAY);
    const held = observe(changed, [job("completed_success", { id: "a" })], 2_000, DECAY);
    expect(held.changedAt.get("a")).toBe(1_000);
    const gone = observe(held, [job("completed_success", { id: "a" })], 1_000 + DECAY, DECAY);
    expect(gone.changedAt.has("a")).toBe(false);
  });
});

describe("decaying", () => {
  it("walks from 1 to 0 across the decay, and is gone at the end", () => {
    const first = observe(null, [job("running", { id: "a" })], 0, DECAY);
    const seen = observe(first, [job("completed_success", { id: "a" })], 0, DECAY);
    expect(decaying(seen, 0, DECAY).get("a")).toEqual({ age: 0, remaining: 1 });
    expect(decaying(seen, 18_000, DECAY).get("a")?.remaining).toBeCloseTo(0.6);
    expect(decaying(seen, DECAY, DECAY).has("a")).toBe(false);
  });

  it("decays nothing where the token did not read", () => {
    const first = observe(null, [job("running", { id: "a" })], 0, 0);
    const seen = observe(first, [job("completed_success", { id: "a" })], 0, 0);
    expect(decaying(seen, 0, 0).size).toBe(0);
  });
});
