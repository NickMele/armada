// The fifth arrangement's own data, tested as the answer it is.
//
// **The fixture is shaped like the real Job the drawing used**,
// `01M27918MN0011N9KZEV9ZWHY3` — four steps, `scope` and `implement` and
// `tests` and `handoff`, `tests` overruled and `handoff` the delivering step a
// person approves at. **Every test here builds the whole Job and never picks
// a step** — that is the one thing this arrangement is not allowed to depend
// on, and a test that passed `open` would not catch its return.

import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";
import type { CheckRun as CheckRunRow, VerdictSheetProps } from "@armada/components";
import type { Diff, Evidence, JobDetail as JobWhole, JobSummary, Noted, Remarks, StepDetail } from "@armada/protocol";

import { humanGateStepOf, overruleReasonOf, verdictSlotAfterAnswer } from "./verdict-answered";

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "land",
    label: "Land",
    ordinal: 3,
    state: "advanced",
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

const NOW = Date.parse("2026-09-11T05:45:00Z");

function overruleNote(over: Partial<Noted> = {}): Noted {
  return {
    seq: 1,
    at: "2026-09-11T05:16:03.465Z",
    by: "fleet",
    level: "warn",
    msg: "a person overruled the gate and the step advanced",
    step: "tests",
    fields: [
      { name: "overruled", value: "gate_failure" },
      { name: "said", value: "Reasonable, and typed by a person." },
    ],
    ...over,
  };
}

describe("the step a person answers at", () => {
  it("finds the step gated human_always", () => {
    const gate = step({ step_id: "handoff", advance_gate: "human_always" });
    expect(humanGateStepOf([step({ advance_gate: "auto_if_judge_passes" }), gate])).toBe(gate);
  });

  it("is undefined where no step in the frozen workflow ever asks a person", () => {
    expect(humanGateStepOf([step({ advance_gate: "auto" }), step({ advance_gate: "auto" })])).toBeUndefined();
  });
});

describe("the reason a person gave for an overrule", () => {
  it("reads the said field off the note for that step", () => {
    expect(overruleReasonOf([overruleNote()], "tests")).toBe("Reasonable, and typed by a person.");
  });

  it("is undefined for a step the note is not about", () => {
    expect(overruleReasonOf([overruleNote()], "implement")).toBeUndefined();
  });

  it("is undefined where the log has no note about an overrule at all", () => {
    expect(overruleReasonOf([], "tests")).toBeUndefined();
  });

  it("is undefined where a person overruled and typed nothing", () => {
    const said = overruleNote({ fields: [{ name: "overruled", value: "gate_failure" }] });
    expect(overruleReasonOf([said], "tests")).toBeUndefined();
  });

  it("takes the most recent note where a step was overruled more than once", () => {
    const first = overruleNote({ seq: 1, fields: [{ name: "said", value: "first" }] });
    const second = overruleNote({ seq: 2, fields: [{ name: "said", value: "second" }] });
    expect(overruleReasonOf([first, second], "tests")).toBe("second");
  });
});

describe("the verdict sheet once a Job is done and a person answered it", () => {
  function job(over: Partial<JobSummary> = {}): JobSummary {
    return {
      id: "01M27918MN0011N9KZEV9ZWHY3",
      handle: "2-a-job",
      title: "Refuse a merge press whose chosen comments won't fit the brief",
      status: "completed_success",
      workflow_id: "feature",
      owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
      origin: "manual",
      urgency: "normal",
      atomic: true,
      model: "sonnet",
      created_at: "2026-09-11T03:43:58.613Z",
      branch: "armada/2-refuse-a-merge-press-whose-chosen-comments",
      current_step_id: "handoff",
      ...over,
    };
  }

  const PASSED_MANIFEST_CHECKS: StepDetail["checks"] = [
    { kind: "manifest_check", name: "build" },
    { kind: "manifest_check", name: "test" },
    { kind: "manifest_check", name: "format" },
  ];
  const PASSED_RUNS: StepDetail["check_runs"] = [
    { attempt: 1, name: "build", outcome: "passed" },
    { attempt: 1, name: "test", outcome: "passed" },
    { attempt: 1, name: "format", outcome: "passed" },
  ];

  function scopeStep(): StepDetail {
    return step({
      step_id: "scope",
      ordinal: 0,
      judge_checks: [{ criteria: 2, gaming_check: false }],
      judged: [
        { attempt: 1, criterion_id: "addresses_the_request", verdict: "met" },
        { attempt: 1, criterion_id: "names_what_it_will_touch", verdict: "met" },
      ],
    });
  }

  function implementStep(): StepDetail {
    return step({
      step_id: "implement",
      ordinal: 1,
      checks: PASSED_MANIFEST_CHECKS,
      check_runs: PASSED_RUNS,
    });
  }

  function testsStep(overridden: boolean): StepDetail {
    return step({
      step_id: "tests",
      ordinal: 2,
      checks: PASSED_MANIFEST_CHECKS,
      check_runs: PASSED_RUNS,
      judge_checks: [{ criteria: 1, gaming_check: true }],
      judged: [
        { attempt: 1, criterion_id: "tests_exercise_behaviour", verdict: "met" },
        {
          attempt: 1,
          criterion_id: "declared_plan_drift",
          verdict: "not_met",
          expected: "The step should not touch crates/ipc/operations.toml.",
        },
      ],
      overridden,
    });
  }

  // The real Job's own shape: a person asked the gate again after
  // `gate_undecided`, and the second attempt answered `gate_failure` off the
  // same Checks and the same Judge call — writing no `check_runs` and no
  // `judged` row of its own. `onlyCurrentAttempt` (`facts.ts`) has to find
  // attempt 1's rows for a step shaped like that, with no help from this file.
  function testsStepRerun(): StepDetail {
    return step({
      ...testsStep(true),
      attempts: [
        { attempt: 1, outcome: "stopped", why: "gate_undecided", started_at: "2026-09-11T04:10:10.968Z" },
        { attempt: 2, outcome: "stopped", why: "gate_failure", started_at: "2026-09-11T04:50:53.326Z" },
      ],
    });
  }

  function handoffStep(): StepDetail {
    return step({
      step_id: "handoff",
      ordinal: 3,
      advance_gate: "human_always",
      delivers: true,
      updated_at: "2026-09-11T05:29:16.834Z",
    });
  }

  const NO_DIFF: Diff = { state: "none" };
  const NO_REMARKS: Remarks = { state: "none" };

  function evidenceWith(...steps: { step_id: string; claimed: string }[]): Evidence {
    return {
      state: "read",
      jobId: job().id,
      steps: steps.map((one) => ({
        step_id: one.step_id,
        evidence_type: "diff",
        claimed: one.claimed,
        shown_by: "a run",
      })),
    };
  }

  function slotFor(args: {
    overridden?: boolean;
    evidence?: Evidence;
    notes?: readonly Noted[];
    /** The `tests` step reran without redoing its Checks or its Judge call. */
    rerun?: boolean;
  }): ReactElement<VerdictSheetProps> {
    const tests = args.rerun === true ? testsStepRerun() : testsStep(args.overridden ?? true);
    const whole: JobWhole = {
      job: job(),
      created_at: "2026-09-11T03:43:58.613Z",
      steps: [scopeStep(), implementStep(), tests, handoffStep()],
      acceptance_criteria: [],
      dependencies: [],
      spend: {
        cost_micros: 3_891_121,
        cost_cap_micros: 10_000_000,
        turns: 126,
        turn_cap: 300,
        ran_ms: 5_569_564,
        drones: 5,
      },
      delivery: { pull_request: "https://forge.invalid/armada/armada/pull/630", landed: "merged" },
    };
    return verdictSlotAfterAnswer({
      job: job(),
      whole,
      recorded: {
        diff: NO_DIFF,
        evidence:
          args.evidence ??
          evidenceWith(
            { step_id: "tests", claimed: "The tests step's own claim." },
            { step_id: "handoff", claimed: "The delivering step's own claim." },
          ),
        remarks: NO_REMARKS,
      },
      opensRecords: { jobId: job().id, open: async () => ({ ok: true }), onSaid: () => {} },
      now: NOW,
      notes: args.notes ?? [overruleNote({ fields: [{ name: "said", value: "The note is correct and needed." }] })],
      onOpenPullRequest: async () => ({ ok: true }),
    }) as ReactElement<VerdictSheetProps>;
  }

  function rowsOf(slot: ReactElement<VerdictSheetProps>): CheckRunRow[] {
    const provesIt = slot.props.provesIt as ReactElement<{ rows: CheckRunRow[] }>;
    return provesIt.props.rows;
  }

  // Item 1 — "what came back" reads the whole Job, not the open step.
  describe("what came back", () => {
    it("reads the delivering step's own claim", () => {
      const slot = slotFor({});
      expect(slot.props.cameBack).toBe("The delivering step's own claim.");
    });

    it("falls back to the last step that claimed anything, where the delivering step claimed nothing", () => {
      const slot = slotFor({ evidence: evidenceWith({ step_id: "tests", claimed: "What tests found." }) });
      expect(slot.props.cameBack).toBe("What tests found.");
    });
  });

  // Item 2 — "what proves it" reads every step, and an overruled verdict from
  // any step always appears, whichever step is open by default.
  describe("what proves it, across the whole Job", () => {
    it("folds every passing step's Checks into one row naming which steps and which Checks", () => {
      const rows = rowsOf(slotFor({}));
      const passed = rows.find((row) => row.id === "checks-passed");
      expect(passed?.says).toBe("Implement and tests passed their Checks");
      expect(passed?.identifier).toBe("build · test · format");
    });

    it("gives every judged criterion, on every step, its own row", () => {
      const rows = rowsOf(slotFor({}));
      const metOnScope = rows.filter(
        (row) => row.identifier === "Judge: met" && (row.says === "addresses_the_request" || row.says === "names_what_it_will_touch"),
      );
      expect(metOnScope).toHaveLength(2);
      expect(rows.some((row) => row.says === "tests_exercise_behaviour" && row.identifier === "Judge: met")).toBe(
        true,
      );
    });

    it("carries the overruled step's row even though the delivering step — the default open one — carries none", () => {
      const rows = rowsOf(slotFor({ overridden: true }));
      const overruled = rows.find((row) => row.named === "overruled");
      expect(overruled?.says).toBe("declared_plan_drift");
      expect(overruled?.identifier).toBe("Judge: not met · overruled by you");
    });

    it("draws the ordinary not-met row, and no overruled row, where the step was not overridden", () => {
      const rows = rowsOf(slotFor({ overridden: false }));
      expect(rows.some((row) => row.named === "overruled")).toBe(false);
      const refused = rows.find((row) => row.says === "declared_plan_drift");
      expect(refused?.identifier).toBe("Judge: not met");
    });

    // A rerun gate leaves the step's own current attempt (2) with no rows of
    // its own — the real Job's own shape. This is what `withDataAttempt` used
    // to patch around locally; the shared read in `facts.ts` now does it for
    // every caller.
    it("still shows a reran step's Checks and its overruled criterion", () => {
      const rows = rowsOf(slotFor({ rerun: true }));
      expect(rows.some((row) => row.id === "checks-passed" && row.says?.toString().includes("tests"))).toBe(true);
      const overruled = rows.find((row) => row.named === "overruled");
      expect(overruled?.says).toBe("declared_plan_drift");
    });
  });

  // Item 3 — plain words, no counts, no jargon.
  describe("the overruled criterion's own row, in plain words", () => {
    it("says the criterion, then the verdict, never a count or the internal jargon", () => {
      const rows = rowsOf(slotFor({ overridden: true }));
      const overruled = rows.find((row) => row.named === "overruled");
      expect(overruled?.says).not.toMatch(/refused|of \d+ criteri|gaming check/i);
      expect(overruled?.identifier).toBe("Judge: not met · overruled by you");
    });

    it("labels the Judge's own reason and the person's own reason apart", () => {
      const rows = rowsOf(slotFor({ overridden: true }));
      const overruled = rows.find((row) => row.named === "overruled");
      const detail = overruled?.detail as ReactElement | undefined;
      const text = JSON.stringify(detail);
      expect(text).toMatch(/Judge’s reason: The step should not touch crates\/ipc\/operations\.toml\./);
      expect(text).toMatch(/Your reason: “The note is correct and needed\.”/);
    });

    it("carries the Judge's own reason alone where the log kept no reason", () => {
      const rows = rowsOf(slotFor({ overridden: true, notes: [] }));
      const overruled = rows.find((row) => row.named === "overruled");
      const text = JSON.stringify(overruled?.detail);
      expect(text).toMatch(/Judge’s reason:/);
      expect(text).not.toMatch(/Your reason:/);
    });
  });

  it("carries the header off the human gate step, and the settled pull request", () => {
    const slot = slotFor({});
    expect(slot.props.header).toEqual({ done: "Done", when: expect.stringContaining("approved") });
    expect(slot.props.pullRequest).toBeDefined();
    expect(slot.props.actions).toBeUndefined();
    expect(slot.props.recordNote).toBeDefined();
  });

  it("adds how many Drones ran to the figures", () => {
    const slot = slotFor({});
    expect(slot.props.figures.some((figure) => figure.label === "Drones" && figure.value === "5")).toBe(true);
  });
});
