// When a job starts waiting on somebody, and what one notification says.
//
// **The cases here are the ones that decide whether notifications survive
// contact with a person.** Two of them are the whole feature: a job that was
// already waiting when Bridge opened must tell nobody, and five arriving
// together must be one telling rather than five.

import { describe, expect, it } from "vitest";

import { entering, telling, waitingIn } from "./waiting";
import type { JobSummary } from "@armada/protocol";

/** A row with everything this reads, and nothing it does not. */
function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
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

const NOON = Date.parse("2026-08-31T12:00:00Z");

describe("who is waiting", () => {
  it("reads the set off the same rule the Needs-you tab draws", () => {
    const held = waitingIn([
      job({ id: "a", status: "awaiting_review" }),
      job({ id: "b", status: "running" }),
      job({ id: "c", status: "running", asking: true }),
      job({ id: "d", status: "piloted" }),
    ]);
    expect([...held].sort()).toEqual(["a", "c"]);
  });
});

describe("entering the set", () => {
  it("says nothing about a job that was already in it", () => {
    const jobs = [job({ id: "a", status: "awaiting_review" })];
    // The first reading fills the set. This is the whole of "a job already
    // waiting when Bridge started is not news".
    expect(entering(waitingIn(jobs), jobs)).toEqual([]);
  });

  it("names a job that has just arrived in it", () => {
    const before = waitingIn([job({ id: "a", status: "running" })]);
    const entered = entering(before, [job({ id: "a", status: "awaiting_review" })]);
    expect(entered.map((one) => one.id)).toEqual(["a"]);
  });

  it("names a job again when it leaves and comes back", () => {
    // Changes requested sends it back to the drone; reaching the gate a second
    // time is a second thing to look at, not a repeat of the first.
    const waiting = waitingIn([job({ id: "a", status: "awaiting_review" })]);
    const working = waitingIn([job({ id: "a", status: "running" })]);
    expect(entering(waiting, [job({ id: "a", status: "awaiting_review" })])).toEqual([]);
    expect(entering(working, [job({ id: "a", status: "awaiting_review" })])).toHaveLength(1);
  });

  it("says nothing about a job that moved between two statuses both waiting", () => {
    const before = waitingIn([job({ id: "a", status: "awaiting_approval" })]);
    expect(entering(before, [job({ id: "a", status: "awaiting_review" })])).toEqual([]);
  });

  it("says nothing about a job that stopped needing anybody", () => {
    const before = waitingIn([job({ id: "a", status: "awaiting_review" })]);
    expect(entering(before, [job({ id: "a", status: "completed_success" })])).toEqual([]);
  });
});

describe("what one telling says", () => {
  it("has nothing to say about a batch of none", () => {
    expect(telling([], NOON)).toBeNull();
  });

  it("names the job, where it is and how long, for a batch of one", () => {
    const one = telling(
      [job({ id: "01M13", title: "Fix session expiry", status: "awaiting_review", current_step_id: "implement" })],
      NOON,
    );
    expect(one?.title).toBe("“Fix session expiry” needs you.");
    expect(one?.body).toBe("01M13 · step implement · 180 min");
    expect(one?.jobId).toBe("01M13");
  });

  it("drops the parts nothing served rather than filling them in", () => {
    const one = telling([job({ id: "01M13", created_at: "not a date" })], NOON);
    expect(one?.body).toBe("01M13");
  });

  it("is one telling for five, not five", () => {
    const five = ["a", "b", "c", "d", "e"].map((id, at) =>
      job({ id, title: `Job ${id}`, created_at: `2026-08-31T0${at}:00:00Z`, status: "awaiting_review" }),
    );
    const one = telling(five, NOON);
    expect(one?.title).toBe("5 jobs need you.");
    expect(one?.body).toBe("Job a\nJob b\nJob c\nand 2 more");
  });

  it("opens the set rather than picking one of several", () => {
    const two = [job({ id: "a" }), job({ id: "b" })];
    expect(telling(two, NOON)?.jobId).toBeNull();
  });

  it("names the oldest first, and a job with no readable date last", () => {
    const three = [
      job({ id: "broken", title: "Broken date", created_at: "not a date" }),
      job({ id: "new", title: "Newer", created_at: "2026-08-31T11:00:00Z" }),
      job({ id: "old", title: "Older", created_at: "2026-08-31T09:00:00Z" }),
    ];
    expect(telling(three, NOON)?.body).toBe("Older\nNewer\nBroken date");
  });

  it("counts the ones it did not name", () => {
    const four = ["a", "b", "c", "d"].map((id, at) =>
      job({ id, title: id, created_at: `2026-08-31T0${at}:00:00Z` }),
    );
    expect(telling(four, NOON)?.body).toBe("a\nb\nc\nand 1 more");
  });
});
