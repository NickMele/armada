// A Record row, from the three shapes the log admits.

import type { Recorded } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { ledgerRowOf, ledgerRowsOf } from "./ledger";

function moved(over: Partial<Recorded> = {}): Recorded {
  return {
    seq: 7,
    status: "running",
    moved: { kind: "status", to: "awaiting_review" },
    actor: "fleet",
    at: "2026-09-22T10:00:00Z",
    ...over,
  };
}

describe("where a row happened", () => {
  it("names no step for the Job's own machine moving, which is a fact not a gap", () => {
    expect(ledgerRowOf(moved()).coord).toBeNull();
  });

  it("names the step for a step move", () => {
    const row = ledgerRowOf(
      moved({ moved: { kind: "step", step_id: "implement", from: "running", to: "advanced" } }),
    );

    expect(row.coord).toEqual({ step: "implement", step_attempt: 1 });
  });

  it("names no group and no task, because the wire has neither", () => {
    const row = ledgerRowOf(
      moved({ moved: { kind: "drone", step_id: "implement", drone_id: "01D", presence: "drone_spawned" } }),
    );

    expect(row.coord?.group).toBeUndefined();
    expect(row.coord?.task).toBeUndefined();
  });
});

describe("what a row is", () => {
  it("qualifies a status move by where it went, so kinds stay distinguishable", () => {
    expect(ledgerRowOf(moved()).kind).toBe("status_awaiting_review");
  });

  it("takes a Drone row's kind from its presence", () => {
    const row = ledgerRowOf(
      moved({ moved: { kind: "drone", step_id: "plan", drone_id: "01D", presence: "drone_exited" } }),
    );

    expect(row.kind).toBe("drone_exited");
  });

  it("is a plain string, so a new kind is not a major version", () => {
    expect(typeof ledgerRowOf(moved()).kind).toBe("string");
  });
});

describe("who a row is about", () => {
  it("calls the wire's human a person", () => {
    expect(ledgerRowOf(moved({ actor: "human" })).actor).toBe("person");
  });

  it("carries a Drone through", () => {
    expect(ledgerRowOf(moved({ actor: "drone" })).actor).toBe("drone");
  });

  it("reads anything else as Fleet, since judge and check are the draft's own", () => {
    expect(ledgerRowOf(moved({ actor: "fleet" })).actor).toBe("fleet");
    expect(ledgerRowOf(moved({ actor: "something" })).actor).toBe("fleet");
  });
});

describe("what a row came to", () => {
  it("carries a status move's reason where it stored one", () => {
    const row = ledgerRowOf(
      moved({ moved: { kind: "status", to: "escalated", reason: { named: "gate_failure" } } }),
    );

    expect(row.outcome).toBe("gate_failure");
  });

  it("is empty on the destinations that store none, not a placeholder", () => {
    expect(ledgerRowOf(moved()).outcome).toBe("");
  });

  it("carries the why on the one step move that stops a step", () => {
    const row = ledgerRowOf(
      moved({
        moved: { kind: "step", step_id: "tests", from: "running", to: "stopped", why: "gate_failure" },
      }),
    );

    expect(row.outcome).toBe("gate_failure");
  });
});

describe("what a reader pages with", () => {
  it("is the log's own seq and never the instant", () => {
    const row = ledgerRowOf(moved({ seq: 42, at: "2026-09-22T10:00:00Z" }));

    expect(row.cursor).toBe(42);
  });

  it("keeps rows in seq order, including two inside one millisecond", () => {
    const rows = ledgerRowsOf({
      job_id: "01J",
      moves: [
        moved({ seq: 1, at: "2026-09-22T10:00:00Z" }),
        moved({ seq: 2, at: "2026-09-22T10:00:00Z" }),
      ],
    });

    expect(rows.map((row) => row.cursor)).toEqual([1, 2]);
  });

  it("is empty on a Job created and not yet moved, which is a real answer", () => {
    expect(ledgerRowsOf({ job_id: "01J", moves: [] })).toEqual([]);
  });
});
