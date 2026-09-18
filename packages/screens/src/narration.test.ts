// Where a step's rows land: under the sentence before them, and under the task
// the Drone had marked working when they happened. #1185.
import { describe, expect, it } from "vitest";

import type { PlanTask, Turn, WorkPlan } from "@armada/protocol";

import { answered, called, said } from "./fixtures/build/base";
import { callsSaid, narrationOf, planBarOf, taskAt, workSaid } from "./narration";
import { entriesOf, hideUnread } from "./story";

const STEP = "implement";

function at(seconds: number): string {
  return new Date(Date.parse("2026-09-15T08:20:00Z") + seconds * 1000).toISOString();
}

function planOf(tasks: PlanTask[]): WorkPlan {
  return {
    approach: "Keep arriving evidence durable",
    recorded_by: { by: "step", step_id: "plan", attempt: 1 },
    recorded_at: at(0),
    tasks,
  };
}

function read(turns: Turn[], plan?: WorkPlan, mostEntries?: number) {
  const { rows } = hideUnread(entriesOf(turns, STEP));
  return narrationOf(rows, turns, STEP, plan, mostEntries);
}

/** Every row of every section, which is what the preview would draw. */
function drawn(narration: ReturnType<typeof read>): number {
  return narration.sections.reduce(
    (count, work) => count + work.beats.reduce((rows, beat) => rows + beat.rows.length, 0),
    0,
  );
}

/** Said, then two calls, twice over — ten seconds apart. */
function work(from: number, first: string, second: string): Turn[] {
  const id = `c${from}`;
  return [
    said(STEP, at(from), first),
    called(STEP, at(from + 1), `${id}a`, "Edit", "crates/fleet/src/evidence.rs"),
    answered(STEP, at(from + 2), `${id}a`),
    called(STEP, at(from + 3), `${id}b`, "Read", "crates/fleet/src/settling.rs"),
    answered(STEP, at(from + 4), `${id}b`),
    said(STEP, at(from + 5), second),
    called(STEP, at(from + 6), `${id}c`, "Edit", "crates/fleet/src/daemon/fittings.rs"),
    answered(STEP, at(from + 7), `${id}c`),
  ];
}

describe("a sentence folds the calls after it", () => {
  it("keeps the sentence verbatim, colon and all, and counts the tools behind it", () => {
    const narration = read(work(0, "Now wire this into `Fleet::assembled` in fittings.rs:", "Next."));
    const [only] = narration.sections;
    expect(narration.plan).toBeUndefined();
    expect(only?.task).toBeUndefined();
    const [first, second] = only?.beats ?? [];
    expect(first?.said).toBe("Now wire this into `Fleet::assembled` in fittings.rs:");
    expect(first?.calls).toBe(2);
    expect(callsSaid(first!, false)).toBe("2 calls · Edit, Read");
    expect(callsSaid(second!, true)).toBe("1 call so far · Edit");
  });

  it("puts calls made before any sentence in a beat of their own", () => {
    const turns = [
      called(STEP, at(0), "early", "Grep", "empty_the_inbox"),
      answered(STEP, at(1), "early"),
      said(STEP, at(2), "Found them."),
    ];
    const [early, found] = read(turns).sections[0]?.beats ?? [];
    expect(early?.said).toBeUndefined();
    expect(early?.calls).toBe(1);
    expect(found?.said).toBe("Found them.");
  });

  it("marks a beat holding a failed call", () => {
    const turns = [
      said(STEP, at(0), "Run the tests."),
      called(STEP, at(1), "t", "Bash", "cargo test"),
      answered(STEP, at(2), "t", true),
    ];
    expect(read(turns).sections[0]?.beats[0]?.wrong).toBe(true);
  });
});

describe("work groups under the task marked working when it happened", () => {
  const tasks = (): PlanTask[] => [
    {
      id: "T1",
      title: "Give arriving evidence its own row",
      state: "done",
      working_windows: [{ entered: at(0), left: at(10) }],
    },
    {
      id: "T2",
      title: "Reload saved evidence when Fleet starts",
      state: "working",
      working_windows: [{ entered: at(10) }],
    },
    { id: "T3", title: "Test a restart between submit and settle", state: "open" },
  ];

  it("places each row by its own instant, and a later task waits with nothing", () => {
    const narration = read([...work(1, "First.", "Second."), ...work(11, "Third.", "Fourth.")], planOf(tasks()));
    const [t1, t2, t3] = narration.sections;
    expect(t1?.task?.id).toBe("T1");
    expect(t1?.beats.map((beat) => beat.said)).toEqual(["First.", "Second."]);
    expect(workSaid(t1!)).toBe("3 calls · 6s");
    expect(t1?.newest).toBe(false);
    expect(t2?.beats.map((beat) => beat.said)).toEqual(["Third.", "Fourth."]);
    expect(t2?.newest).toBe(true);
    expect(t3?.beats).toEqual([]);
    expect(workSaid(t3!)).toBeUndefined();
    expect(narration.plan).toEqual({ done: 1, total: 3, states: ["done", "working", "open"] });
  });

  it("draws work before any task was marked under the outside heading, first", () => {
    const later = tasks().map((task) =>
      task.id === "T1" ? { ...task, working_windows: [{ entered: at(20), left: at(30) }] } : task,
    );
    const narration = read(work(0, "Reading first.", "Still reading."), planOf(later));
    const [outside] = narration.sections;
    expect(outside?.task).toBeUndefined();
    expect(outside?.beats).toHaveLength(2);
    expect(outside?.newest).toBe(true);
  });

  it("reads a Drone that never marks a task as work outside any, with the plan still drawn", () => {
    const unmarked = tasks().map(({ working_windows: _, ...task }) => ({ ...task, state: "open" }));
    const narration = read(work(0, "One.", "Two."), planOf(unmarked));
    expect(narration.sections).toHaveLength(4);
    expect(narration.sections[0]?.task).toBeUndefined();
    expect(narration.sections[0]?.beats).toHaveLength(2);
    expect(narration.sections.slice(1).every((one) => one.beats.length === 0)).toBe(true);
    expect(narration.plan).toEqual({ done: 0, total: 3, states: ["open", "open", "open"] });
  });

  it("never matches a call to a task by the task's words", () => {
    const turns = [
      said(STEP, at(50), "Reload saved evidence when Fleet starts"),
      called(STEP, at(51), "r", "Edit", "crates/fleet/src/evidence.rs"),
    ];
    const closed = tasks().map((task) => ({ ...task, working_windows: [] }));
    expect(read(turns, planOf(closed)).sections[0]?.task).toBeUndefined();
  });
});

describe("the bound is in entries, across the whole reading", () => {
  /** A sentence, then that many `Read` calls — each one entry, its answer folded in. */
  function reading(calls: number): Turn[] {
    const turns = [said(STEP, at(0), "Reading the reducer to find the selector.")];
    for (let one = 0; one < calls; one += 1) {
      turns.push(called(STEP, at(one * 2 + 1), `r${one}`, "Read", `src/file${one}.ts`));
      turns.push(answered(STEP, at(one * 2 + 2), `r${one}`));
    }
    return turns;
  }

  it("draws ten of a dozen consecutive calls of one tool", () => {
    // The case the group counting was introduced to avoid, taken deliberately:
    // twelve `Read` calls fill the preview and the last ten are what is drawn.
    const narration = read(reading(12), undefined, 10);
    expect(drawn(narration)).toBe(10);
    expect(narration.earlier).toBe(3);
  });

  it("cuts inside a run, and the sentence over it goes with the rows it lost", () => {
    const [only] = read(reading(12), undefined, 10).sections;
    const [beat] = only?.beats ?? [];
    expect(only?.beats).toHaveLength(1);
    expect(beat?.said).toBeUndefined();
    // Counted over what is in hand, so the fold line cannot say twelve over ten.
    expect(beat?.calls).toBe(10);
    expect(callsSaid(beat!, false)).toBe("10 calls · Read");
  });

  it("counts a sentence as one of them", () => {
    // Six sentences with one call each is twelve entries, so the bound keeps
    // five sentences and the five calls under them — never ten sentences.
    const turns = Array.from({ length: 6 }, (_, one) => [
      said(STEP, at(one * 3), `Step ${one}.`),
      called(STEP, at(one * 3 + 1), `c${one}`, "Edit", "crates/fleet/src/evidence.rs"),
      answered(STEP, at(one * 3 + 2), `c${one}`),
    ]).flat();
    const narration = read(turns, undefined, 10);
    const beats = narration.sections[0]?.beats ?? [];
    expect(beats.map((beat) => beat.said)).toEqual(["Step 1.", "Step 2.", "Step 3.", "Step 4.", "Step 5."]);
    expect(drawn(narration)).toBe(5);
    expect(narration.earlier).toBe(2);
  });

  it("leaves every task's own figure over the work it holds, drawn or not", () => {
    const plan = planOf([
      {
        id: "T1",
        title: "Read it first",
        state: "done",
        working_windows: [{ entered: at(0), left: at(25) }],
      },
      { id: "T2", title: "Then edit it", state: "working", working_windows: [{ entered: at(25) }] },
    ]);
    const edits = Array.from({ length: 9 }, (_, one) => [
      called(STEP, at(31 + one * 2), `e${one}`, "Edit", "crates/fleet/src/evidence.rs"),
      answered(STEP, at(32 + one * 2), `e${one}`),
    ]).flat();
    const narration = read([...reading(12), said(STEP, at(30), "Now the edits."), ...edits], plan, 10);
    const [t1, t2] = narration.sections;
    // T1's sentence and its twelve reads are all outside the bound, so it draws
    // none of them — and still says what it did, because that figure is the
    // task's rather than the window's.
    expect(t1?.beats).toHaveLength(0);
    expect(workSaid(t1!)).toBe("12 calls · 23s");
    expect(t2?.beats.map((beat) => beat.said)).toEqual(["Now the edits."]);
    expect(drawn(narration)).toBe(9);
    expect(narration.earlier).toBe(13);
  });

  it("draws everything with no bound at all, which is the log sheet", () => {
    const narration = read(reading(12));
    expect(drawn(narration)).toBe(12);
    expect(narration.earlier).toBe(0);
  });
});

describe("which task a moment belongs to", () => {
  it("is the one entered last where two windows overlap", () => {
    const overlapping: PlanTask[] = [
      { id: "T1", title: "a", state: "working", working_windows: [{ entered: at(0) }] },
      { id: "T2", title: "b", state: "working", working_windows: [{ entered: at(5) }] },
    ];
    expect(taskAt(at(3), overlapping)).toBe("T1");
    expect(taskAt(at(6), overlapping)).toBe("T2");
  });

  it("is none at the instant a window closes", () => {
    const closed: PlanTask[] = [
      { id: "T1", title: "a", state: "done", working_windows: [{ entered: at(0), left: at(5) }] },
    ];
    expect(taskAt(at(5), closed)).toBeUndefined();
  });

  it("does not count a dropped task in the bar", () => {
    const bar = planBarOf(
      planOf([
        { id: "T1", title: "a", state: "done" },
        { id: "T2", title: "b", state: "dropped", reason: "covered" },
      ]),
    );
    expect(bar).toEqual({ done: 1, total: 1, states: ["done"] });
  });
});
