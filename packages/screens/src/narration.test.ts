// Where a step's rows land: under the sentence before them, and under the task
// the Drone had marked working when they happened. #1185.
import { describe, expect, it } from "vitest";

import type { PlanTask, Turn, WorkPlan } from "@armada/protocol";

import { answered, called, said } from "./fixtures/build/base";
import { callsSaid, declaredBy, narrationOf, planBarOf, taskAt, workSaid } from "./narration";
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

// `#1498`. The owner read Job `3-show-what-s-running-in-the-drones-stat` while
// it ran: T5 declared two files, edited both, moved `open` → `done` without
// ever being marked `working`, and its edits drew outside every task.
describe("an edit no window covers draws under the task that declared its path", () => {
  const TREE = "~/Development/armada/.armada/worktrees/01K5/";
  const OVERVIEW = "packages/screens/src/overview.ts";
  const SPEC = "packages/screens/src/overview.test.ts";
  const LEFT = "packages/screens/src/left-column.ts";

  /** T5 is never marked working, so Fleet rightly sends it no window. T1 is. */
  const declaring = (): PlanTask[] => [
    {
      id: "T1",
      title: "Give the left column its own reading",
      state: "working",
      scope: [LEFT],
      working_windows: [{ entered: at(60) }],
    },
    { id: "T5", title: "Show what is running", state: "done", scope: [OVERVIEW, SPEC] },
  ];

  /** Every section in order, by task id — `undefined` is the outside row. */
  const headings = (narration: ReturnType<typeof read>) =>
    narration.sections.map((work) => work.task?.id);

  /** Every call row of a section, as `[tool, what it named]`. */
  function calls(work: { beats: { rows: { called?: { tool: string; detail: string } }[] }[] }) {
    return work.beats
      .flatMap((beat) => beat.rows)
      .filter((row) => row.called !== undefined)
      .map((row) => [row.called?.tool, row.called?.detail]);
  }

  it("draws all three edits under it and takes the sentence over them with it", () => {
    const turns = [
      said(STEP, at(0), "Now the overview reading."),
      called(STEP, at(1), "a", "Edit", `${TREE}${OVERVIEW} +8 -3`),
      answered(STEP, at(2), "a"),
      called(STEP, at(3), "b", "Edit", `${TREE}${SPEC} +30`),
      answered(STEP, at(4), "b"),
      called(STEP, at(5), "c", "Edit", `${TREE}${SPEC} +10`),
      answered(STEP, at(6), "c"),
    ];
    const narration = read(turns, planOf(declaring()));
    // No outside row at all: the sentence went with its run, so nothing is left
    // for a heading to stand over.
    expect(headings(narration)).toEqual(["T1", "T5"]);
    const [t1, t5] = narration.sections;
    expect(t5?.beats.map((beat) => beat.said)).toEqual(["Now the overview reading."]);
    expect(calls(t5!)).toEqual([
      ["Edit", OVERVIEW],
      ["Edit", SPEC],
      ["Edit", SPEC],
    ]);
    // Only `called` turns are counted, so the sentence adds nothing to the
    // three — and the stretch reaches back over it, at(0) to at(5).
    expect(t5?.calls).toBe(3);
    expect(workSaid(t5!)).toBe("3 calls · 5s");
    expect(t1?.beats).toEqual([]);
  });

  it("keeps the sentence outside where one Read in its run stayed there", () => {
    const turns = [
      said(STEP, at(0), "Read it, then reword it."),
      called(STEP, at(1), "a", "Read", `${TREE}${OVERVIEW}`),
      answered(STEP, at(2), "a"),
      called(STEP, at(3), "b", "Edit", `${TREE}${OVERVIEW} +8 -3`),
      answered(STEP, at(4), "b"),
    ];
    const narration = read(turns, planOf(declaring()));
    expect(headings(narration)).toEqual([undefined, "T1", "T5"]);
    const [outside, , t5] = narration.sections;
    expect(outside?.beats.map((beat) => beat.said)).toEqual(["Read it, then reword it."]);
    expect(calls(outside!)).toEqual([["Read", OVERVIEW]]);
    expect(calls(t5!)).toEqual([["Edit", OVERVIEW]]);
  });

  it("keeps the sentence outside where its run split between two declarations", () => {
    const turns = [
      said(STEP, at(0), "Both columns at once."),
      called(STEP, at(1), "a", "Edit", `${TREE}${LEFT} +3`),
      answered(STEP, at(2), "a"),
      called(STEP, at(3), "b", "Edit", `${TREE}${OVERVIEW} +8`),
      answered(STEP, at(4), "b"),
    ];
    const narration = read(turns, planOf(declaring()));
    expect(headings(narration)).toEqual([undefined, "T1", "T5"]);
    const [outside, t1, t5] = narration.sections;
    expect(outside?.beats.map((beat) => beat.said)).toEqual(["Both columns at once."]);
    expect(calls(outside!)).toEqual([]);
    expect(calls(t1!)).toEqual([["Edit", LEFT]]);
    expect(calls(t5!)).toEqual([["Edit", OVERVIEW]]);
  });

  // **A call and its answer are one thing that happened.** A failed Edit draws
  // two rows, and the second is the one somebody opened the log for — it has to
  // stand under the same heading as the call, not under one for work that did
  // not happen there.
  it("draws a failed edit's answer beside its call, and opens that task's beat", () => {
    const turns = [
      said(STEP, at(0), "Reword the stat."),
      called(STEP, at(1), "a", "Edit", `${TREE}${OVERVIEW} +8 -3`),
      answered(STEP, at(2), "a", true),
    ];
    const narration = read(turns, planOf(declaring()));
    expect(headings(narration)).toEqual(["T1", "T5"]);
    const [, t5] = narration.sections;
    // One beat, holding the sentence, the call and what came back.
    expect(t5?.beats).toHaveLength(1);
    const [beat] = t5?.beats ?? [];
    expect(beat?.said).toBe("Reword the stat.");
    expect(beat?.rows.map((row) => row.kind)).toEqual(["called", "answered"]);
    // `beat.wrong` is what draws a beat open and unfoldable, and it is T5's
    // beat that has to open rather than one over work that did not fail.
    expect(beat?.wrong).toBe(true);
  });

  // The behaviour the owner did not ask to change: a sentence said before the
  // Drone marked T1 working stays outside, even though every call under it is
  // T1's. The clock placed it, and a declaration only fills a gap the clock left.
  it("leaves a sentence outside where a window, not a declaration, placed its calls", () => {
    const turns = [
      said(STEP, at(59), "Now the left column."),
      called(STEP, at(61), "a", "Edit", `${TREE}${LEFT} +3`),
      answered(STEP, at(62), "a"),
    ];
    const narration = read(turns, planOf(declaring()));
    expect(headings(narration)).toEqual([undefined, "T1", "T5"]);
    const [outside, t1] = narration.sections;
    expect(outside?.beats.map((beat) => beat.said)).toEqual(["Now the left column."]);
    expect(calls(t1!)).toEqual([["Edit", LEFT]]);
  });

  it("leaves a Read of the same file outside, because it names no task it was for", () => {
    const turns = [
      called(STEP, at(1), "a", "Read", `${TREE}${OVERVIEW}`),
      answered(STEP, at(2), "a"),
      called(STEP, at(3), "b", "Edit", `${TREE}${OVERVIEW} +8 -3`),
      answered(STEP, at(4), "b"),
      called(STEP, at(5), "c", "Bash", `pnpm -C packages/screens test ${OVERVIEW}`),
      answered(STEP, at(6), "c"),
    ];
    const [outside, , t5] = read(turns, planOf(declaring())).sections;
    expect(calls(outside!)).toEqual([
      ["Read", OVERVIEW],
      ["Bash", `pnpm -C packages/screens test ${OVERVIEW}`],
    ]);
    expect(calls(t5!)).toEqual([["Edit", OVERVIEW]]);
  });

  it("does not let a declaration take an edit a window already covers", () => {
    const turns = [
      called(STEP, at(61), "a", "Edit", `${TREE}${OVERVIEW} +2`),
      answered(STEP, at(62), "a"),
    ];
    const [t1, t5] = read(turns, planOf(declaring())).sections;
    expect(t1?.task?.id).toBe("T1");
    expect(calls(t1!)).toEqual([["Edit", OVERVIEW]]);
    expect(t5?.beats).toEqual([]);
  });
});

describe("the task a declared path names", () => {
  const TREE = "~/Development/armada/.armada/worktrees/01K5/";

  it("matches at a path segment and not on a shared spelling", () => {
    const tasks: PlanTask[] = [{ id: "T1", title: "a", state: "open", scope: ["crates/ipc"] }];
    expect(declaredBy(`${TREE}crates/ipc/operations.toml`, tasks)).toBe("T1");
    expect(declaredBy(`${TREE}crates/ipc-extra/x.rs`, tasks)).toBeUndefined();
  });

  it("reads a declared directory as holding the files under it", () => {
    const tasks: PlanTask[] = [{ id: "T1", title: "a", state: "open", scope: ["crates/ipc/"] }];
    expect(declaredBy(`${TREE}crates/ipc/src/lib.rs`, tasks)).toBe("T1");
  });

  it("gives the file to the exact declaration over the directory holding it", () => {
    const tasks: PlanTask[] = [
      { id: "T1", title: "a", state: "open", scope: ["crates/ipc"] },
      { id: "T2", title: "b", state: "open", scope: ["crates/ipc/operations.toml"] },
    ];
    expect(declaredBy(`${TREE}crates/ipc/operations.toml`, tasks)).toBe("T2");
  });

  it("gives a tie to plan order, and says nothing where no task named the path", () => {
    const same: PlanTask[] = [
      { id: "T1", title: "a", state: "open", scope: ["crates/ipc/operations.toml"] },
      { id: "T2", title: "b", state: "open", scope: ["crates/ipc/operations.toml"] },
    ];
    expect(declaredBy(`${TREE}crates/ipc/operations.toml`, same)).toBe("T1");
    expect(declaredBy(`${TREE}crates/store/src/lib.rs`, same)).toBeUndefined();
  });

  it("says nothing for a task that declared nothing", () => {
    const bare: PlanTask[] = [{ id: "T1", title: "a", state: "open" }];
    expect(declaredBy(`${TREE}crates/ipc/x.rs`, bare)).toBeUndefined();
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
