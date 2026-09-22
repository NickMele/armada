// A Record row, from the three shapes the log admits.

import type { JobDetail, Recorded } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { executingSequential } from "../fixtures/build/arc";
import {
  countsOf,
  familyOf,
  LEDGER_FAMILIES,
  ledgerOf,
  ledgerRowOf,
  ledgerRowsOf,
  unfiledIn,
} from "./ledger";

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

describe("the eight filters", () => {
  it("puts a Check's run and a Judge's answer in different families", () => {
    expect(familyOf("checked")).toBe("checks");
    expect(familyOf("judged")).toBe("judges");
  });

  it("files a task's own done under Tasks and never under Evidence", () => {
    expect(familyOf("task_done")).toBe("tasks");
  });

  it("files a case run under Tests, which is never an Evidence row", () => {
    expect(familyOf("case_run")).toBe("tests");
    expect(familyOf("cases_rerun")).toBe("tests");
  });

  it("files the Job's own machine moving under no family, because it is not a Task", () => {
    expect(familyOf("created")).toBeNull();
    expect(familyOf("status_completed_success")).toBeNull();
  });

  it("gives a kind it has never heard of no family rather than guessing one", () => {
    expect(familyOf("something_the_backend_invented")).toBeNull();
  });

  it("counts no row twice, which is the invariant — never that the seven sum to All", () => {
    const rows = ledgerOf({ detail: arcDetail() });
    const counts = countsOf(rows);
    const filed = LEDGER_FAMILIES.reduce((total, one) => total + counts[one], 0);

    expect(filed).toBe(rows.length - unfiledIn(rows).length);
    expect(filed).toBeLessThan(rows.length);
  });

  it("names the rows All holds and no filter does, so nobody subtracts", () => {
    const rows = ledgerOf({ detail: arcDetail() });

    expect(unfiledIn(rows).map((row) => row.kind)).toContain("created");
  });
});

describe("the Record, composed from today's reads", () => {
  it("reads newest first", () => {
    const rows = ledgerOf({ detail: arcDetail() });

    expect(rows.map((row) => row.at)).toEqual([...rows.map((row) => row.at)].sort().reverse());
  });

  it("gives a Check's run its own row, with Check in the who column", () => {
    const rows = ledgerOf({ detail: arcDetail() }).filter((row) => row.kind === "checked");

    expect(rows.map((row) => row.what)).toContain("typecheck");
    expect(rows.every((row) => row.actor === "check")).toBe(true);
  });

  it("says Judge on a criterion answered and Fleet on the plan being recorded", () => {
    const rows = ledgerOf({ detail: arcDetail() });

    expect(rows.find((row) => row.kind === "judged")?.actor).toBe("judge");
    expect(rows.find((row) => row.kind === "plan_recorded")?.actor).toBe("fleet");
  });

  it("places a task row down to its group and its task", () => {
    const rows = ledgerOf({ detail: arcDetail() }).filter((one) => one.kind === "task_done");

    expect(rows.every((row) => row.coord?.group !== undefined)).toBe(true);
    expect(rows.map((row) => row.coord?.task)).toContain("T1");
  });

  it("names no step at all for the Job's own machine moving", () => {
    const row = ledgerOf({ detail: arcDetail() }).find((one) => one.kind === "created");

    expect(row?.coord).toBeNull();
  });

  it("says when a finished task's files fell outside the plan", () => {
    const rows = ledgerOf({ detail: arcDetail() }).filter((row) => row.kind === "task_files");

    expect(rows.some((row) => row.outcome.startsWith("outside the plan"))).toBe(true);
  });

  it("lets the history own the moves it carries, rather than deriving them twice", () => {
    const detail = arcDetail();
    const history = [
      moved({ seq: 1, moved: { kind: "status", to: "queued" }, actor: "human", at: detail.created_at }),
    ];
    const rows = ledgerOf({ detail, history });

    expect(rows.filter((row) => row.coord === null).map((row) => row.kind)).toEqual([
      "status_queued",
    ]);
  });

  it("draws a Job with no plan and nothing run, rather than nothing at all", () => {
    const rows = ledgerOf({ detail: { ...arcDetail(), steps: [], work_plan: undefined } });

    expect(rows.map((row) => row.kind)).toContain("created");
  });
});

/** The arc's own Job mid-`implement`, which is the richest Record there is. */
function arcDetail(): JobDetail {
  const moment = executingSequential();
  const read = moment.fixtures[0]!.watched;
  if (read.state !== "read") throw new Error("the arc's implement moment carries no detail");
  return read.detail;
}
