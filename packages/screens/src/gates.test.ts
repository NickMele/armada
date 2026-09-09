// The one reading of `check_runs` and `judged`, tested as the answer it is.
//
// **Arithmetic, so it runs in node.** Which attempt's rows count, how a panel
// groups, whether one veto refuses a criterion and which output an `o` press
// opens are all functions of the wire — a hundred cases cost what one costs,
// and none of them needs a browser. What the rows *look like* is
// `evidence.test.tsx`, which mounts them.
//
// **These are the readings two surfaces share.** The phase strip and the Checks
// and Verdicts chapters both call every function here, so a case that fails
// below is a case the two would have answered differently.

import { describe, expect, it } from "vitest";
import type { Criterion, Judged, StepDetail } from "@armada/protocol";

import { citationsOf, givenTo } from "./cited";
import { assertedIn } from "./asserted";
import { checksOf, outputOf, outputRunOf, panelSizeOf, panelsOf } from "./gates";
import { noteFor, regionOf, rowsOf } from "./outputs";

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "verify",
    label: "Verify",
    ordinal: 3,
    state: "stopped",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-09T09:30:00Z",
    updated_at: "2026-09-09T09:41:00Z",
    ...over,
  };
}

/** Two runs of the step, so "the current attempt" is a thing to get wrong. */
const TWICE: StepDetail["attempts"] = [
  { attempt: 1, outcome: "retrying", started_at: "2026-09-09T09:00:00Z" },
  { attempt: 2, outcome: "refused", started_at: "2026-09-09T09:20:00Z" },
];

const CRITERIA: Criterion[] = [
  { criterion_id: "c1", text: "The public API is byte-identical.", source: "brief" },
  { criterion_id: "c2", text: "Behaviour is unchanged.", source: "brief" },
];

/** One member's answer, with only what the case is about spelled out. */
function judged(over: Partial<Judged> & Pick<Judged, "criterion_id">): Judged {
  return { attempt: 1, verdict: "met", ...over };
}

describe("the Checks a step declares", () => {
  it("keeps a row for a Check the gate has not reached", () => {
    const reads = checksOf(
      step({ checks: [{ kind: "manifest_check", name: "build" }, { kind: "diff_nonempty" }] }),
    );
    expect(reads.map((read) => read.name)).toEqual(["build", "diff_nonempty"]);
    expect(reads.every((read) => read.run === undefined)).toBe(true);
  });

  it("joins a run to its declaration by name", () => {
    const reads = checksOf(
      step({
        checks: [{ kind: "manifest_check", name: "build" }],
        check_runs: [{ attempt: 1, name: "build", outcome: "failed", produced: "exit 101" }],
      }),
    );
    expect(reads[0]?.run?.produced).toBe("exit 101");
  });

  // `check_runs` has carried every attempt's rows since protocol 7.0, and both
  // surfaces draw the live gate. Read across all of them, a step retried once
  // reports the run before last.
  it("reads only the current attempt's runs", () => {
    const reads = checksOf(
      step({
        attempts: TWICE,
        checks: [{ kind: "manifest_check", name: "build" }],
        check_runs: [
          { attempt: 1, name: "build", outcome: "failed", produced: "exit 101" },
          { attempt: 2, name: "build", outcome: "passed" },
        ],
      }),
    );
    expect(reads[0]?.run?.outcome).toBe("passed");
  });
});

describe("which output a press opens", () => {
  it("takes the Check that did not pass, because that is why the step stopped", () => {
    expect(
      outputOf(
        step({
          check_runs: [
            { attempt: 1, name: "a", outcome: "passed", output_path: "checks/a.log" },
            { attempt: 1, name: "b", outcome: "failed", output_path: "checks/b.log" },
          ],
        }),
      ),
    ).toBe("checks/b.log");
  });

  it("takes the first output there is where nothing failed", () => {
    expect(
      outputOf(
        step({
          check_runs: [
            { attempt: 1, name: "a", outcome: "passed", output_path: "checks/a.log" },
            { attempt: 1, name: "b", outcome: "passed", output_path: "checks/b.log" },
          ],
        }),
      ),
    ).toBe("checks/a.log");
  });

  it("never offers a stale attempt's output", () => {
    expect(
      outputOf(
        step({
          attempts: TWICE,
          check_runs: [
            { attempt: 1, name: "build", outcome: "failed", output_path: "checks/1.log" },
            { attempt: 2, name: "build", outcome: "passed", output_path: "checks/2.log" },
          ],
        }),
      ),
    ).toBe("checks/2.log");
  });

  it("answers nothing where no Check kept a file, so the press is left alone", () => {
    expect(outputOf(step({ check_runs: [{ attempt: 1, name: "a", outcome: "passed" }] }))).toBe(
      undefined,
    );
  });

  // The header act, the `o` key and the chapter's own reader all come through
  // one call, so the row and the path cannot name two different Checks.
  it("names the row as well as the path, so a reader can say whose output it is", () => {
    const run = outputRunOf(
      step({
        check_runs: [
          { attempt: 1, name: "a", outcome: "passed", output_path: "checks/a.log" },
          { attempt: 1, name: "b", outcome: "failed", output_path: "checks/b.log" },
        ],
      }),
    );
    expect(run?.name).toBe("b");
    expect(run?.output_path).toBe("checks/b.log");
  });
});

describe("what a step's suite asserted", () => {
  // `check-outcomes.toml`: a step that advanced having skipped every Check
  // verified nothing, and the record must be able to say that. `skipped`
  // advances and is not a pass, so it is neither `passed` nor `failed` here.
  it("puts a skipped Check apart from a passed one and from a failed one", () => {
    const rows = assertedIn(
      step({
        check_runs: [
          { attempt: 1, name: "a", outcome: "passed" },
          { attempt: 1, name: "b", outcome: "skipped", produced: "no changed file is under docs/" },
          { attempt: 1, name: "c", outcome: "failed" },
          { attempt: 1, name: "d", outcome: "never_ran" },
        ],
      }),
    );
    expect(rows.map((row) => row.named)).toEqual(["passed", "absent", "failed", "failed"]);
  });

  // The four not-passes are four different things to *do* about, which the
  // Checks list draws. They are one thing to *read* here.
  it("carries the record's own reason on a skip that has no earlier run", () => {
    const rows = assertedIn(
      step({
        check_runs: [
          { attempt: 1, name: "b", outcome: "skipped", produced: "no changed file is under docs/" },
        ],
      }),
    );
    expect(rows[0]?.against).toBe("no changed file is under docs/");
  });

  it("compares a Check against its own previous run of the step", () => {
    const rows = assertedIn(
      step({
        attempts: TWICE,
        check_runs: [
          { attempt: 1, name: "build", outcome: "passed" },
          { attempt: 2, name: "build", outcome: "skipped", produced: "nothing under crates/" },
        ],
      }),
    );
    expect(rows).toHaveLength(1);
    expect(rows[0]?.against).toBe("passed at attempt 1 · not run at attempt 2");
  });

  it("says so when nothing moved between two runs", () => {
    const rows = assertedIn(
      step({
        attempts: TWICE,
        check_runs: [
          { attempt: 1, name: "build", outcome: "passed" },
          { attempt: 2, name: "build", outcome: "passed" },
        ],
      }),
    );
    expect(rows[0]?.against).toBe("identical at attempt 1 and attempt 2");
  });

  // Nothing runs a step's Checks at the commit its work branched from, and no
  // Job records a base commit — so a first run has nothing measured to compare
  // with, and the column is absent rather than invented.
  it("leaves the comparison off a first run that has nothing to compare with", () => {
    const rows = assertedIn(step({ check_runs: [{ attempt: 1, name: "a", outcome: "passed" }] }));
    expect(rows[0]?.against).toBe(undefined);
  });

  // A declared Check the gate has not reached is `queued` on the Checks list
  // and is none of the three states an assertion has.
  it("draws nothing for a step whose gate has not run", () => {
    expect(assertedIn(step())).toEqual([]);
  });

  // No Manifest Check declares a description, so every row falls back to its
  // identifier and the component says why. A sentence invented here would read
  // as one somebody could check against the Check.
  it("never invents a sentence a Check did not write", () => {
    const rows = assertedIn(step({ check_runs: [{ attempt: 1, name: "a", outcome: "passed" }] }));
    expect(rows[0]?.says).toBe(undefined);
    expect(rows[0]?.identifier).toBe("a");
  });
});

describe("a Check's output, as a reading", () => {
  const window = {
    attempt: 1,
    name: "check:test_suite",
    path: ".armada/checks/01M130/verify.1.0.log",
    lines: ["--- stdout ---", "    left:  1970-01-01T00:00:00Z"],
    from_line: 1_939,
    total_lines: 2_180,
    bytes: 61_204,
    whole: false,
  };

  // A region is a window onto a file and a citation names the file, so a row
  // numbered from one would send a reader to the wrong line.
  it("numbers the rows as the file numbers them", () => {
    expect(rowsOf(window).map((row) => row.at)).toEqual([1_939, 1_940]);
  });

  it("keeps a line exactly as it was written", () => {
    const rows = rowsOf(window);
    expect(rows[1]).toMatchObject({ row: "line", text: "    left:  1970-01-01T00:00:00Z" });
  });

  it("says the reading is truncated, and whose output it is", () => {
    expect(regionOf(window).says).toBe("check:test_suite · lines 1,939–1,940 of 2,180");
    expect(regionOf(window).path).toBe(".armada/checks/01M130/verify.1.0.log");
  });

  it("says the whole of a short file is the whole of it", () => {
    const short = { ...window, from_line: 1, total_lines: 2, whole: true };
    expect(regionOf(short).says).toBe("check:test_suite · 2 lines");
  });

  // Four states and four sentences. One sentence for all of them would tell a
  // person a Check printed nothing when what is true is that nobody has asked.
  it("tells a reading nobody asked for from one that came back empty", () => {
    expect(noteFor(undefined)).not.toBe(noteFor({ state: "fetching" }));
    expect(noteFor({ state: "got", output: { ...window, lines: [] } })).toContain("printed nothing");
    expect(noteFor({ state: "absent", note: "gone" })).toBe("gone");
  });
});

describe("a panel, read as one row per criterion", () => {
  it("groups a panel's members onto the criterion they answered", () => {
    const panels = panelsOf(
      step({
        judged: [
          judged({ criterion_id: "c1", member: 1 }),
          judged({ criterion_id: "c1", member: 2 }),
          judged({ criterion_id: "c1", member: 3 }),
        ],
      }),
      CRITERIA,
    );
    expect(panels.length).toBe(1);
    expect(panels[0]?.members.length).toBe(3);
  });

  // Unanimity, from `docs/concepts/judge.md`. A row that took the majority
  // would draw a criterion two judges met as met, and it is refused.
  it("refuses the criterion on one veto out of three", () => {
    const panels = panelsOf(
      step({
        judged: [
          judged({ criterion_id: "c1", member: 1 }),
          judged({ criterion_id: "c1", member: 2, verdict: "not_met" }),
          judged({ criterion_id: "c1", member: 3 }),
        ],
      }),
      CRITERIA,
    );
    expect(panels[0]?.verdict).toBe("not_met");
    expect(panels[0]?.refused.length).toBe(1);
  });

  it("puts the members in panel order, whatever order they arrived in", () => {
    const panels = panelsOf(
      step({
        judged: [
          judged({ criterion_id: "c1", member: 3 }),
          judged({ criterion_id: "c1", member: 1 }),
          judged({ criterion_id: "c1", member: 2 }),
        ],
      }),
      CRITERIA,
    );
    expect(panels[0]?.members.map((one) => one.member)).toEqual([1, 2, 3]);
  });

  // The ordinal is the frozen position a citation names — `02` — and never the
  // row's place on screen.
  it("carries each criterion's frozen position and its own words", () => {
    const panels = panelsOf(step({ judged: [judged({ criterion_id: "c2" })] }), CRITERIA);
    expect(panels[0]?.ordinal).toBe(2);
    expect(panels[0]?.criterion?.text).toBe("Behaviour is unchanged.");
  });

  it("leaves a criterion the Job does not carry without a position, rather than inventing one", () => {
    const panels = panelsOf(step({ judged: [judged({ criterion_id: "gone" })] }), CRITERIA);
    expect(panels[0]?.ordinal).toBe(undefined);
    expect(panels[0]?.criterion).toBe(undefined);
  });

  it("reads only the current attempt, so a rerun does not draw last run's verdicts", () => {
    const panels = panelsOf(
      step({
        attempts: TWICE,
        judged: [
          judged({ criterion_id: "c1", attempt: 1, verdict: "not_met" }),
          judged({ criterion_id: "c1", attempt: 2 }),
        ],
      }),
      CRITERIA,
    );
    expect(panels.length).toBe(1);
    expect(panels[0]?.verdict).toBe("met");
  });

  it("keeps the criteria in the order they were asked in, never sorted", () => {
    const panels = panelsOf(
      step({
        judged: [judged({ criterion_id: "c2" }), judged({ criterion_id: "c1" })],
      }),
      CRITERIA,
    );
    expect(panels.map((panel) => panel.criterionId)).toEqual(["c2", "c1"]);
  });
});

describe("the panel's shape", () => {
  // `panel_size` is absent at one, which is the convention `Judged.member`
  // keeps: a value always means a panel.
  it("is one where the declaration names none", () => {
    expect(panelSizeOf(step({ judge_checks: [{ criteria: 1, gaming_check: false }] }), [])).toBe(1);
  });

  it("is the declared size before a single verdict arrives", () => {
    expect(
      panelSizeOf(
        step({ judge_checks: [{ criteria: 2, panel_size: 3, gaming_check: false }] }),
        [],
      ),
    ).toBe(3);
  });

  it("is never narrower than the members that actually answered", () => {
    const showing = step({
      judge_checks: [{ criteria: 1, gaming_check: false }],
      judged: [judged({ criterion_id: "c1", member: 1 }), judged({ criterion_id: "c1", member: 2 })],
    });
    expect(panelSizeOf(showing, panelsOf(showing, CRITERIA))).toBe(2);
  });
});

// `cited.ts`'s two readings, over the same fixtures. They are the Verdicts
// chapter's alone rather than shared, so they are not in `gates.ts` — but they
// are read off a `Panel`, which is, and a second `step()` beside this one is
// exactly the drift the file above exists to stop.

/** What a member's call was handed, as one criterion's panel all got it. */
const HANDED = { digest: "9c41ab0e77d25631", size: 12_880, model: "haiku" };

describe("what each member of a panel quoted", () => {
  // Absent and empty are different facts. A Fleet before 8.3 recorded no
  // citations for anybody, and a screen drawing "nothing was quoted" for that
  // would make a claim about how the refusals were worded.
  it("is null where no member recorded any, and a list where one did", () => {
    const none = panelsOf(step({ judged: [judged({ criterion_id: "c1" })] }), CRITERIA);
    expect(citationsOf(none, 1, () => {})).toBeNull();

    const some = panelsOf(
      step({ judged: [judged({ criterion_id: "c1", cited: [] })] }),
      CRITERIA,
    );
    expect(citationsOf(some, 1, () => {})).toEqual([]);
  });

  it("names the judge and the criterion at a panel, and the criterion alone at one", () => {
    const one = panelsOf(
      step({
        judged: [
          judged({
            criterion_id: "c2",
            cited: [{ region: "diff", from_line: 88, to_line: 90 }],
          }),
        ],
      }),
      CRITERIA,
    );
    const alone = citationsOf(one, 1, () => {}) ?? [];
    expect(alone[0]?.who).toBe("02");
    expect(alone[0]?.where).toBe("diff lines 88\u201390");

    const panelled = panelsOf(
      step({
        judged: [
          judged({ criterion_id: "c2", member: 1 }),
          judged({
            criterion_id: "c2",
            member: 2,
            cited: [{ region: "request", from_line: 12, to_line: 12 }],
          }),
        ],
      }),
      CRITERIA,
    );
    const rows = citationsOf(panelled, 2, () => {}) ?? [];
    expect(rows.length).toBe(1);
    expect(rows[0]?.who).toBe("j2 \u00b7 02");
    expect(rows[0]?.where).toBe("request line 12");
  });
});

describe("what each member of a panel was handed", () => {
  it("is nothing at all where no member recorded it", () => {
    const panels = panelsOf(step({ judged: [judged({ criterion_id: "c1" })] }), CRITERIA);
    expect(givenTo(panels[0] as (typeof panels)[number])).toBeNull();
  });

  it("says the panel was a panel where every member holds the same object", () => {
    const panels = panelsOf(
      step({
        judged: [
          judged({ criterion_id: "c1", member: 1, given: HANDED }),
          judged({ criterion_id: "c1", member: 2, given: HANDED }),
        ],
      }),
      CRITERIA,
    );
    const handed = givenTo(panels[0] as (typeof panels)[number]);
    expect(handed?.identical).toBe(true);
    expect(handed?.rows.map((row) => row.value)).toEqual([
      "9c41ab0e77d25631",
      "12880 characters",
      "haiku",
    ]);
  });

  // The mark is what carries the comparison into the view that has lost it:
  // reading j2's object alone, nothing about a digest says it is the odd one.
  it("marks the field they were not handed alike, in the per-judge views too", () => {
    const panels = panelsOf(
      step({
        judged: [
          judged({ criterion_id: "c1", member: 1, given: HANDED }),
          judged({
            criterion_id: "c1",
            member: 2,
            given: { ...HANDED, digest: "0000000000000000" },
          }),
        ],
      }),
      CRITERIA,
    );
    const handed = givenTo(panels[0] as (typeof panels)[number]);
    expect(handed?.identical).toBe(false);
    expect(handed?.rows.map((row) => row.differs)).toEqual([true, undefined, undefined]);
    expect(handed?.each.map((one) => one.judge)).toEqual(["j1", "j2"]);
    expect(handed?.each.every((one) => one.rows[0]?.differs === true)).toBe(true);
  });

  // A member with no `given` beside members that have one cannot be compared,
  // so the panel is not claimed to have been handed one object.
  it("does not claim a panel where one member recorded nothing", () => {
    const panels = panelsOf(
      step({
        judged: [
          judged({ criterion_id: "c1", member: 1, given: HANDED }),
          judged({ criterion_id: "c1", member: 2 }),
        ],
      }),
      CRITERIA,
    );
    expect(givenTo(panels[0] as (typeof panels)[number])?.identical).toBe(false);
  });
});
