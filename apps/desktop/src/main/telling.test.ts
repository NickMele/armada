// What main does with a reading, told through the two effects and nothing else.
//
// The words a notification carries are `waiting.test.ts`'s in
// `packages/screens`. What is pinned here is the timing — the quiet window, the
// seeding, and the dock count — which is the half that lives in this process.

import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";

import { Attention, QUIET_MS } from "./telling";
import type { Effects } from "./telling";
import type { Telling } from "@armada/screens/src/waiting";
import type { JobSummary } from "@armada/protocol";

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "a",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    ...over,
  };
}

const READ = Date.parse("2026-08-31T12:00:00Z");

function watching(): { attention: Attention; shown: Telling[]; counts: number[] } {
  const shown: Telling[] = [];
  const counts: number[] = [];
  const effects: Effects = {
    show: (told) => shown.push(told),
    count: (waiting) => counts.push(waiting),
    now: () => READ,
  };
  return { attention: new Attention(effects), shown, counts };
}

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe("the first reading", () => {
  it("tells nobody about a job that was already waiting", () => {
    const { attention, shown } = watching();
    attention.read([job({ status: "awaiting_review" })], READ);
    vi.advanceTimersByTime(QUIET_MS * 2);
    expect(shown).toEqual([]);
  });

  it("still counts what is waiting, because the dock is a standing signal", () => {
    const { attention, counts } = watching();
    attention.read([job({ status: "awaiting_review" })], READ);
    expect(counts).toEqual([1]);
  });

  it("does not seed off a publish that no reading produced", () => {
    // Bridge publishes state before it is connected to anything. Seeding on one
    // of those would make the first real reading look like every waiting job
    // arriving at once.
    const { attention, shown, counts } = watching();
    attention.read([], null);
    expect(counts).toEqual([]);
    attention.read([job({ status: "awaiting_review" })], READ);
    vi.advanceTimersByTime(QUIET_MS * 2);
    expect(shown).toEqual([]);
  });
});

describe("a job entering the set", () => {
  it("is told once, after the quiet window and not before", () => {
    const { attention, shown } = watching();
    attention.read([job({ status: "running" })], READ);
    attention.read([job({ status: "awaiting_review" })], READ);
    expect(shown).toEqual([]);
    vi.advanceTimersByTime(QUIET_MS);
    expect(shown).toHaveLength(1);
    expect(shown[0]?.jobId).toBe("a");
  });

  it("is five jobs' worth of one notification, not five", () => {
    const { attention, shown } = watching();
    const running = ["a", "b", "c", "d", "e"].map((id) => job({ id, status: "running" }));
    attention.read(running, READ);
    // They arrive on five separate publishes, which is what an event stream
    // does — one per status move.
    for (let at = 0; at < running.length; at += 1) {
      attention.read(
        running.map((one, index) =>
          index <= at ? { ...one, status: "awaiting_review" } : one,
        ),
        READ,
      );
    }
    vi.advanceTimersByTime(QUIET_MS);
    expect(shown).toHaveLength(1);
    expect(shown[0]?.title).toBe("5 jobs need you.");
    expect(shown[0]?.jobId).toBeNull();
  });

  it("does not hold the window open while jobs keep arriving", () => {
    const { attention, shown } = watching();
    attention.read([job({ id: "a", status: "running" }), job({ id: "b", status: "running" })], READ);
    attention.read([job({ id: "a", status: "awaiting_review" }), job({ id: "b", status: "running" })], READ);
    vi.advanceTimersByTime(QUIET_MS - 1);
    attention.read(
      [job({ id: "a", status: "awaiting_review" }), job({ id: "b", status: "awaiting_review" })],
      READ,
    );
    vi.advanceTimersByTime(1);
    // Both are in the one telling, because the second landed inside the window
    // the first opened — and the window did not restart.
    expect(shown).toHaveLength(1);
    expect(shown[0]?.title).toBe("2 jobs need you.");
  });

  it("tells again about a job that left the set and came back", () => {
    const { attention, shown } = watching();
    attention.read([job({ status: "running" })], READ);
    attention.read([job({ status: "awaiting_review" })], READ);
    vi.advanceTimersByTime(QUIET_MS);
    attention.read([job({ status: "running" })], READ);
    attention.read([job({ status: "awaiting_review" })], READ);
    vi.advanceTimersByTime(QUIET_MS);
    expect(shown).toHaveLength(2);
  });

  it("says nothing when a reading changes nothing about who is waiting", () => {
    const { attention, shown, counts } = watching();
    attention.read([job({ status: "awaiting_review" })], READ);
    attention.read([job({ status: "awaiting_review", current_step_id: "verify" })], READ);
    vi.advanceTimersByTime(QUIET_MS);
    expect(shown).toEqual([]);
    // And the dock is told once, not on every publish.
    expect(counts).toEqual([1]);
  });
});

describe("the dock count", () => {
  it("follows the set down as well as up", () => {
    const { attention, counts } = watching();
    attention.read([job({ id: "a", status: "awaiting_review" }), job({ id: "b", status: "awaiting_review" })], READ);
    attention.read([job({ id: "a", status: "awaiting_review" }), job({ id: "b", status: "running" })], READ);
    attention.read([job({ id: "a", status: "completed_success" }), job({ id: "b", status: "running" })], READ);
    expect(counts).toEqual([2, 1, 0]);
  });
});

describe("closing", () => {
  it("drops a collected batch rather than posting it later", () => {
    const { attention, shown } = watching();
    attention.read([job({ status: "running" })], READ);
    attention.read([job({ status: "awaiting_review" })], READ);
    attention.close();
    vi.advanceTimersByTime(QUIET_MS * 2);
    expect(shown).toEqual([]);
  });
});
