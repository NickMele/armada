// What the Land board reads off a Job that finished — the arithmetic, which is
// where the old boards contradicted themselves.

import { describe, expect, it } from "vitest";

import { landed } from "./fixtures/build/arc-landed";
import type { JobFixture } from "./fixtures/fixture";
import { landedOf, type LandedInput, type LandedRead } from "./landed";

const MOMENT = landed();
const FIXTURE: JobFixture = MOMENT.fixtures[0]!;

/** The moment as the screen is handed it: the Job's reads, and its draft. */
function inputOf(over: Partial<LandedInput> = {}): LandedInput {
  return {
    job: FIXTURE.job,
    whole: FIXTURE.watched.state === "read" ? FIXTURE.watched.detail : null,
    draft: MOMENT.draft,
    manifest: FIXTURE.manifests[0],
    holding: FIXTURE.resources.state === "read" ? FIXTURE.resources.resources : null,
    ...over,
  };
}

function read(over: Partial<LandedInput> = {}): LandedRead {
  const board = landedOf(inputOf(over));
  expect(board, "arc/landed draws a board").toBeDefined();
  return board!;
}

/** One figure of what it cost, by its label. */
function figure(board: LandedRead, label: string): string {
  return board.cost.figures.find((one) => one.label === label)?.value ?? "";
}

describe("a Job that has not finished draws no board", () => {
  it("draws nothing while the Job is running", () => {
    expect(landedOf(inputOf({ job: { ...FIXTURE.job, status: "running" } }))).toBeUndefined();
  });

  it("draws nothing before the Job's own read has arrived", () => {
    expect(landedOf(inputOf({ whole: null }))).toBeUndefined();
  });
});

describe("the headline counts what the Job was held to", () => {
  it("counts the criteria a step returned a verdict for", () => {
    expect(read().count).toBe("2 of 2 met");
  });

  it("reads not covered where nothing answered a criterion, and never green", () => {
    const criteria = [
      ...MOMENT.draft.criteria!,
      { criterion_id: "a9", text: "Nobody judged this", verified_by: "judge" as const, origin: { origin: "prompt" as const } },
    ];
    const board = read({ draft: { ...MOMENT.draft, criteria } });
    expect(board.count).toBe("2 of 3 met");
    expect(board.criteria[2]?.verdict).toBe("not covered");
    expect(board.criteria[2]?.status).not.toBe("completed-success");
  });

  it("says a Job with one pull request completes when it lands", () => {
    expect(read().completes).toBe("Completes when its pull request lands.");
  });

  it("says a Job with members completes when every one of them has", () => {
    const landing = { ...MOMENT.draft.landing!, complete_when: "all_members_landed" as const };
    expect(read({ draft: { ...MOMENT.draft, landing } }).completes).toBe(
      "Completes when every member has landed.",
    );
  });
});

describe("every figure is derived with its retries", () => {
  // Group three retried once, so its two tasks ran twice: eight tasks, ten
  // agents. A count of eight would contradict the retry on the row below it.
  it("counts an agent per task, and again for the group that was retried", () => {
    expect(figure(read(), "Drones")).toBe("10");
  });

  // Four Checks at group one's boundary, seven at each of the others, and
  // group three's seven twice.
  it("counts a boundary's Checks once per run of that boundary", () => {
    expect(figure(read(), "Checks")).toBe("32");
  });

  it("adds the spend up from each task's own agent, since the Job carries no total", () => {
    expect(figure(read(), "Spend")).toBe("$7.53");
    expect(figure(read(), "Turns")).toBe("135");
  });

  it("names the Job's own totals where Fleet counted them", () => {
    const whole = FIXTURE.watched.state === "read" ? FIXTURE.watched.detail : null;
    const board = read({
      whole: {
        ...whole!,
        spend: {
          cost_micros: 9_000_000,
          cost_cap_micros: 20_000_000,
          turns: 140,
          turn_cap: 400,
          ran_ms: 1,
          drones: 10,
        },
      },
    });
    expect(figure(board, "Spend")).toBe("$9.00 of $20.00");
    expect(figure(board, "Turns")).toBe("140 of 400");
  });

  it("times the Job from its own two instants", () => {
    expect(figure(read(), "Run time")).toBe("2h 02m");
  });
});

describe("the groups", () => {
  it("draws one row per group, with the commit each left", () => {
    const groups = read().groups;
    expect(groups.map((one) => one.name)).toEqual([
      "Group one",
      "Group two",
      "Group three",
      "Group four",
    ]);
    expect(groups[3]?.commit).toBe("e0d47a1");
  });

  it("says the retried group ran its Checks twice", () => {
    expect(read().groups[2]?.checks).toBe("7 Checks, twice");
  });

  it("leaves the time taken absent, because nothing times a group", () => {
    expect(read().groups.every((one) => one.took === undefined)).toBe(true);
  });

  it("counts the files the plan claims, de-duplicated", () => {
    expect(read().groupsSummary).toBe("4 groups · 8 tasks · 9 files");
  });
});

describe("the run", () => {
  it("draws every step with what it came to", () => {
    const steps = read().steps.steps;
    expect(steps.map((one) => one.label)).toEqual([
      "Plan the change",
      "Implement",
      "Write tests",
      "Review the change",
    ]);
    expect(steps.every((one) => one.verdict === "passed")).toBe(true);
  });
});

describe("the test set", () => {
  it("draws the handoff run of every case, with who ran it", () => {
    const set = read().runs[0]!;
    expect(set.runs).toHaveLength(4);
    expect(set.runs.every((one) => one.who === "Fleet")).toBe(true);
  });

  it("reads a case with no spec as not covered, and says why it did not run", () => {
    const row = read().runs[0]!.runs.find((one) => one.spec.endsWith("Board.test.tsx"));
    expect(row?.outcome).toBe("not covered");
    expect(row?.meta).toBe("no spec covers Board.tsx");
  });

  it("says plainly that there is no before-run", () => {
    expect(read().runs[0]!.note).toContain("no before-run");
  });

  it("draws the run a person made themselves apart, and names them", () => {
    const set = read().runs[1]!;
    expect(set.runs).toHaveLength(1);
    expect(set.runs[0]?.who).toBe("you");
  });

  it("says nobody has run one where none was run by hand", () => {
    const board = read({ draft: { ...MOMENT.draft, runs: [] } });
    expect(board.runs[1]?.runs).toHaveLength(0);
    expect(board.runs[1]?.absent).toContain("Nobody has run one");
  });
});

describe("what it produced and what it left", () => {
  it("names the pull request and the branch it merged into", () => {
    const delivered = read().sections[0]!;
    expect(delivered.parts[0]?.value).toBe("https://git.example/armada/pull/1604");
    expect(delivered.parts[0]?.meta).toBe("merged into main");
  });

  it("names the branch, the worktree still on disk and the record", () => {
    const left = read().sections[1]!;
    expect(left.parts.map((one) => one.name)).toEqual(["Branch", "Worktree", "Record"]);
    expect(left.parts[1]?.meta).toContain("on disk");
    expect(left.parts[2]?.value).toContain(".armada/logs/");
  });

  it("says why there is no worktree where nothing has read one", () => {
    const left = read({ holding: null }).sections[1]!;
    expect(left.parts[1]?.value).toBeUndefined();
    expect(left.parts[1]?.absent).toContain("worktree is unknown");
  });
});
