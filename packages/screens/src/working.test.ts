// The Working body's arithmetic, case by case.
//
// **Every case is a claim `working.ts` makes**, and the two drafts drawn from it
// are drawn from these numbers. The turns are built with the fixtures' own
// constructors, so a change to the wire's shape breaks this rather than drifting
// past it.
import { describe, expect, it } from "vitest";

import type { Turn } from "@armada/protocol";

import { answered, called, checked, producedTurn, said } from "./fixtures/build/base";
import { byToolOf, producedOf, runsOf, workingOf } from "./working";

const STEP = "implement";

/** An instant this many seconds into the attempt. */
function at(seconds: number): string {
  return new Date(Date.parse("2026-09-10T14:00:00Z") + seconds * 1000).toISOString();
}

/** A row this Bridge has no case for, which no builder makes. */
function unrecognised(ts: string, kind: string): Turn {
  return { ts, seq: 9000 + kind.length, step: STEP, by: "drone", saw: { event: "unrecognised", kind } };
}

describe("a call and its answer are one act", () => {
  const turns = [
    called(STEP, at(0), "c1", "Read", "duration.ts"),
    answered(STEP, at(0.4), "c1"),
  ];

  it("draws one act, not two", () => {
    const { acts } = workingOf(turns, STEP);
    expect(acts).toHaveLength(1);
    expect(acts[0]?.kind).toBe("call");
  });

  it("carries how long the call was open", () => {
    const [call] = workingOf(turns, STEP).acts;
    expect(call?.ms).toBe(400);
  });

  it("names the answer's own row, so a surface drawing rows can still draw it", () => {
    const [call] = workingOf(turns, STEP).acts;
    expect(call?.answeredId).toBe(String(turns[1]?.seq));
  });

  it("names none where the call is still open", () => {
    const [call] = workingOf([called(STEP, at(0), "c9", "Read", "x.ts")], STEP).acts;
    expect(call?.answeredId).toBeUndefined();
  });

  it("marks a call whose answer failed", () => {
    const failed = [called(STEP, at(0), "c1", "Bash", "npm publish"), answered(STEP, at(1), "c1", true)];
    expect(workingOf(failed, STEP).acts[0]?.wrong).toBe(true);
  });

  it("says nothing about the duration of a call still open", () => {
    const [call] = workingOf([called(STEP, at(0), "c1", "Read", "x.ts")], STEP).acts;
    expect(call?.ms).toBeUndefined();
    expect(call?.wrong).toBeUndefined();
  });
});

describe("what the body does not draw", () => {
  it("counts unread rows by kind, most first, and draws none of them", () => {
    const turns = [
      unrecognised(at(1), "system/thinking_tokens"),
      unrecognised(at(2), "reasoning"),
      unrecognised(at(3), "system/thinking_tokens"),
      said(STEP, at(4), "Reading the failure."),
    ];
    const { acts, unread } = workingOf(turns, STEP);
    expect(acts).toHaveLength(1);
    expect(unread).toEqual([
      { kind: "system/thinking_tokens", count: 2 },
      { kind: "reasoning", count: 1 },
    ]);
  });

  it("drops Armada's echo of its own instruction, the way the log does", () => {
    const echo: Turn = {
      ts: at(1),
      seq: 900,
      step: STEP,
      by: "armada",
      saw: { event: "said", text: "Armada opened the step." },
    };
    expect(workingOf([echo, said(STEP, at(2), "Mine.")], STEP).acts).toHaveLength(1);
  });
});

describe("the wall clock and the call time are two figures", () => {
  // The claim the recorded step made unavoidable: 8m of calls inside 43m of
  // attempt. A body that reported one as the other would be wrong five-fold.
  const turns = [
    called(STEP, at(0), "c1", "Read", "a.ts"),
    answered(STEP, at(1), "c1"),
    said(STEP, at(300), "Thinking took the rest of it."),
    called(STEP, at(600), "c2", "Edit", "a.ts"),
    answered(STEP, at(602), "c2"),
  ];

  it("measures the attempt from the first act to the last", () => {
    expect(workingOf(turns, STEP).wall).toBe(602_000);
  });

  it("adds up only the time a call was open", () => {
    expect(workingOf(turns, STEP).inCalls).toBe(3_000);
  });
});

describe("a run of one tool folds to one line", () => {
  const three = [
    called(STEP, at(0), "c1", "Edit", "a.ts"),
    answered(STEP, at(1), "c1"),
    called(STEP, at(2), "c2", "Edit", "b.ts"),
    answered(STEP, at(3), "c2"),
    called(STEP, at(4), "c3", "Edit", "c.ts"),
    answered(STEP, at(5), "c3"),
  ];

  it("folds consecutive calls of the same tool, and adds their time", () => {
    const runs = runsOf(workingOf(three, STEP).acts);
    expect(runs).toHaveLength(1);
    expect(runs[0]?.acts).toHaveLength(3);
    expect(runs[0]?.folded).toBe(true);
    expect(runs[0]?.ms).toBe(3_000);
  });

  it("never folds a run holding a failure", () => {
    const withFailure = [...three, called(STEP, at(6), "c4", "Edit", "d.ts"), answered(STEP, at(7), "c4", true)];
    const runs = runsOf(workingOf(withFailure, STEP).acts);
    expect(runs[0]?.acts).toHaveLength(4);
    expect(runs[0]?.folded).toBe(false);
    expect(runs[0]?.wrong).toBe(1);
  });

  it("breaks a run where the Drone said something in the middle of it", () => {
    const interrupted = [
      ...three.slice(0, 4),
      said(STEP, at(3.5), "Halfway."),
      ...three.slice(4),
    ];
    expect(runsOf(workingOf(interrupted, STEP).acts).map((run) => run.acts.length)).toEqual([2, 1, 1]);
  });

  it("leaves a lone act as a run of one, drawn open", () => {
    const runs = runsOf(workingOf([producedTurn(STEP, at(1), [{ path: "a.ts", change: "modified" }])], STEP).acts);
    expect(runs[0]?.folded).toBe(false);
    expect(runs[0]?.acts[0]?.said).toBe("1 file");
  });
});

describe("a Check is read through the registry", () => {
  it("marks a run the registry says does not advance", () => {
    const failed = checked(STEP, at(1), {
      attempt: 1,
      name: "cargo_nextest",
      outcome: "failed",
      produced: "exit 101",
    });
    expect(workingOf([failed], STEP).acts[0]?.wrong).toBe(true);
  });

  it("leaves a passing run unmarked", () => {
    const passed = checked(STEP, at(1), { attempt: 1, name: "fmt", outcome: "passed" });
    expect(workingOf([passed], STEP).acts[0]?.wrong).toBeUndefined();
  });
});

describe("the calls added up by tool", () => {
  it("counts them and sorts slowest first", () => {
    const turns = [
      called(STEP, at(0), "c1", "Read", "a.ts"),
      answered(STEP, at(1), "c1"),
      called(STEP, at(2), "c2", "TaskOutput", ""),
      answered(STEP, at(62), "c2"),
      called(STEP, at(63), "c3", "Read", "b.ts"),
      answered(STEP, at(64), "c3", true),
    ];
    expect(byToolOf(workingOf(turns, STEP).acts)).toEqual([
      { tool: "TaskOutput", calls: 1, ms: 60_000, wrong: 0 },
      { tool: "Read", calls: 2, ms: 2_000, wrong: 1 },
    ]);
  });
});

describe("what the attempt wrote", () => {
  const first = producedTurn(STEP, at(10), [{ path: "a.ts", change: "modified" }]);
  const again = producedTurn(STEP, at(20), [
    { path: "a.ts", change: "modified" },
    { path: "b.ts", change: "added" },
    { path: "c.ts", change: "added" },
    { path: "d.ts", change: "deleted", outside_plan: true },
  ]);

  it("takes the last reading, because only the newest describes the work", () => {
    expect(producedOf(workingOf([first, again], STEP).acts).files).toHaveLength(4);
  });

  it("counts by the wire's own spelling of the change, most first", () => {
    expect(producedOf(workingOf([again], STEP).acts).changes).toEqual([
      { change: "added", files: 2 },
      { change: "deleted", files: 1 },
      { change: "modified", files: 1 },
    ]);
  });

  it("says how many fall outside the plan the step declared", () => {
    expect(producedOf(workingOf([again], STEP).acts).outsidePlan).toBe(1);
  });

  it("reads empty on an attempt that wrote nothing, rather than nothing at all", () => {
    const empty = producedOf(workingOf([said(STEP, at(1), "Read it.")], STEP).acts);
    expect(empty.files).toEqual([]);
    expect(empty.changes).toEqual([]);
    expect(empty.at).toBeUndefined();
  });
});
