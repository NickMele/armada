// Job detail's fixtures beyond `fixtures/build`: one Job moved to a moment a
// test needs. Moved here from the `Screens/Job detail` stories that built
// them — #1224.

import type { ClaimedBreakage, DeclaredJudge, JobSummary, Refusal, StepDetail } from "@armada/protocol";
import { escalatedEvidenceSuspect, review } from "@armada/screens/src/fixtures/build/index";
import { running } from "@armada/screens/src/fixtures/build/index";
import { advancedStep, BUILD_CHECK, diffRead, freshStep, watchedRead } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

/**
 * The same gate as `Review`, holding on a judge question instead of a clean
 * pass. The question outranks the rest of the slot: no merge answer, no
 * checks list, just the criterion and the three presses.
 */
export function reviewAtAQuestion(): JobFixture {
  const fixture = review();
  if (fixture.watched.state !== "read") return fixture;
  return {
    ...fixture,
    watched: watchedRead({
      ...fixture.watched.detail,
      judge_question: {
        step_id: "regression_verify",
        criterion_id: "c1",
        question: "Does the fix address the cause the note names?",
        expected: "packages/settings/src/selectors.ts imports no store type",
        produced: "The module still imports RootState directly, behind a re-export",
        consequence: "the regression this step exists to catch can still reach the selectors",
        asked_at: "2026-09-10T14:29:40Z",
      },
    }),
  };
}

export const BROKEN = "settings::selectors::visible_manifests_memoises";

/** `running`, with claimed breakages on its detail — #1001. */
export function withBreakages(breakages: (jobId: string) => ClaimedBreakage[]): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  const { detail } = fixture.watched;
  return {
    ...fixture,
    watched: { ...fixture.watched, detail: { ...detail, breakages: breakages(fixture.job.id) } },
  };
}

/** A fixture with its row, and the detail's copy of it, changed alike. */
export function withRow(fixture: JobFixture, over: Partial<JobSummary>): JobFixture {
  const job = { ...fixture.job, ...over };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  return { ...fixture, job, watched: watchedRead({ ...fixture.watched.detail, job }) };
}

/** `Fix`, running rather than finished — `running.ts`'s own shape, for the step taking over. */
export function runningStep(id: string, label: string, ordinal: number): StepDetail {
  return {
    ...freshStep(id, label, ordinal),
    state: "running",
    attempts: [{ attempt: 1, outcome: "running", started_at: "2026-09-10T14:30:00Z" }],
    entered_at: "2026-09-10T14:30:00Z",
    updated_at: "2026-09-10T14:30:00Z",
  };
}

/**
 * `running()`, one step on: Fix is done and Regression check is running —
 * the `job.step_advanced` Fleet would send, built by hand because these
 * stories are what a socket lands rather than a Job in a new state.
 */
export function advancedOnce(fixture: JobFixture): JobFixture {
  const whole = fixture.watched.state === "read" ? fixture.watched.detail : undefined;
  if (whole === undefined) return fixture;
  const steps = whole.steps.map((step) => {
    if (step.step_id === "fix") return advancedStep("fix", "Fix", 3, [BUILD_CHECK]);
    if (step.step_id === "regression_verify") return runningStep("regression_verify", "Regression check", 4);
    return step;
  });
  const job = { ...fixture.job, current_step_id: "regression_verify" };
  return { ...fixture, job, watched: watchedRead({ ...whole, job, steps }) };
}

export const TEST_FILE = "packages/settings/test/useColumnSelectors.test.ts";

/** A removed assertion, so the flag has a file and no line. */
export const PATCH = [
  `diff --git a/${TEST_FILE} b/${TEST_FILE}`,
  `--- a/${TEST_FILE}`,
  `+++ b/${TEST_FILE}`,
  "@@ -52,6 +52,5 @@",
  '   it("drops a column that was hidden", () => {',
  '     const next = reducer(state, hide("owner"));',
  '-    expect(selectVisible(next)).not.toContain("owner");',
  "     expect(next.version).toBe(state.version + 1);",
  "   });",
].join("\n");

export function refusedCommand(call: string, detail: string): Refusal {
  return { tool: "Bash", call, detail, truncated: false, because: "", offers: [], rules: [] };
}

/** The step's gaming check, with every pattern it looks for. Since protocol 13.50. */
export const GAMING: DeclaredJudge = {
  criteria: 2,
  panel_size: 3,
  gaming_check: true,
  gaming_patterns: ["assertion_weakened", "test_scope_narrowed", "tautological_test", "test_skipped", "test_deleted", "check_config_edited"],
};

/**
 * The owner's Job on 14 Sep, in the fixture's words: every Check passed, the
 * gaming check flagged a removed assertion, and three refused commands sat in
 * the same box. `recourse` is Fleet's reading of whether the Drone is still
 * there, which decides what Send it back is.
 */
export function heldByTheGamingCheck(recourse: string[]): JobFixture {
  const fixture = escalatedEvidenceSuspect();
  if (fixture.watched.state !== "read") return fixture;
  const whole = fixture.watched.detail;
  const steps = whole.steps.map(
    (step): StepDetail =>
      step.step_id !== "regression_verify"
        ? step
        : {
            ...step,
            judge_checks: [GAMING],
            judged: step.judged.map((one) => ({ ...one, verdict: "met" })),
            flagged: [
              {
                attempt: 1,
                pattern: "assertion_weakened",
                cited: '`expect(selectVisible(next)).not.toContain("owner")` was taken out, and nothing replaces it.',
                at: { file: TEST_FILE },
                asked:
                  "Does this change alter an existing assertion so that it asserts less than it did, " +
                  "and is that assertion made nowhere else in this change?",
                brief_path: ".armada/briefs/77-split-the-settings-reducer/regression_verify.1.gaming.txt",
              },
            ],
          },
  );
  return {
    ...fixture,
    watched: watchedRead({
      ...whole,
      steps,
      stuck: {
        ...whole.stuck!,
        recourse,
        refused: [
          refusedCommand("call_1", "cargo nextest run --package fleet 2>&1 | tail -80"),
          refusedCommand("call_2", "git stash"),
          refusedCommand("call_3", "git log --oneline -20"),
        ],
        refusals: 3,
      },
    }),
    recorded: {
      ...fixture.recorded,
      diff: diffRead([{ path: TEST_FILE, change: "modified" }], PATCH),
    },
  };
}
