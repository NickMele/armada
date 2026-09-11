// What a step's suite asserted, one row per Check, and how each stood last time.
//
// # `absent` is not `failed`, and it is the whole reason this is drawn
//
// `check-outcomes.toml` puts it in the registry's own words: *a step that
// advanced having skipped every Check verified nothing, and the record must be
// able to say that.* `skipped` advances and did not pass, so a chapter that
// counted advancing Checks would report a step that measured nothing as a step
// that passed. This is where the record says which.
//
// # The granularity is the Check, because nothing finer survives
//
// `AssertionSet` was drawn for the assertions inside a suite — one row per test,
// with the harness's own description on it. Nothing in Armada produces those.
// `crates/checks-runner/src/run.rs` says so in its own header: *nothing here
// reads the output*, and no other crate parses a Check's stdout either. So no
// identifier, no per-assertion outcome and no description exists to put on a
// row, and a reading that invented them would be the fixture this file replaces
// wearing a wire's clothes.
//
// What does survive is one row per declared Check, and every distinction the
// component turns on is on it: three outcomes that are not two, and a record
// that keeps the deliberate skip apart from the four failures.
//
// # The comparison is the previous attempt, not the parent commit
//
// `against` was drawn for *passed at HEAD~1 · absent at HEAD*, and nothing runs
// a step's Checks at the commit the work branched from — no Job records a base
// commit, and `commit_checks` is keyed by a merge commit rather than by a Job's
// parent. What the record does hold is every attempt's rows, so the comparison
// served is between runs of the same step: a Check that passed on attempt 1 and
// was skipped on attempt 2 is the same finding one axis over, and it is one the
// record can actually make.
//
// A Check with no earlier run and nothing else to say carries no `against`, and
// the row draws without one rather than with an invented one.

import { CHECK_ADVANCES, CHECK_OUTCOME } from "@armada/components";
import type { Assertion, AssertionNamed } from "@armada/components";
import type { CheckRun, StepDetail } from "@armada/protocol";

import { onlyCurrentAttempt } from "./facts";

/**
 * What this step's suite asserted, or nothing.
 *
 * **Only the Checks the gate actually decided on this attempt.** A declared
 * Check the gate has not reached is `queued` on the Checks list above this, and
 * it is none of the three states an assertion has — a set that drew it would be
 * claiming a measurement that has not happened.
 *
 * Empty is a step whose gate has not run, and the caller draws nothing rather
 * than a labelled region with no rows in it.
 */
export function assertedIn(step: StepDetail): Assertion[] {
  const shown = onlyCurrentAttempt(step.check_runs);
  const before = previousRuns(step, shown);
  // **The rows keep the order they ran in**, which is the order the workflow
  // declares its Checks and the order the store returns them. `AssertionSet`
  // sorts nothing: an assertion is a thing that happened at a moment, and
  // `against` is what marks a row worth reading rather than its position.
  return shown.map((run) => {
    const against = againstOf(run, before.get(run.name));
    return {
      identifier: run.name,
      named: namedOf(run),
      // `says` is deliberately absent on every row. It is what the assertion
      // asserts *in the harness's own words*, and a Manifest Check declares no
      // description — only a name and a command, and a command is mono
      // machinery rather than a sentence somebody wrote. The component draws
      // the identifier and states the fallback, which is true here and would
      // stop being true the day `armada.yml` grows a sentence per Check.
      ...(against === undefined ? {} : { against }),
    };
  });
}

/**
 * The three states, off the registry rather than off a list spelled here.
 *
 * `skipped` is `absent`: the Check declares which paths it covers, the step
 * touched none of them, and it advanced without measuring anything. The other
 * four not-passes are `failed` — they are four different things to *do* about,
 * which is what the Checks list above draws, and one thing to *read* here.
 */
function namedOf(run: CheckRun): AssertionNamed {
  if (run.outcome === SKIPPED) return "absent";
  return CHECK_ADVANCES[run.outcome] === false ? "failed" : "passed";
}

/**
 * How this Check stood last time the step ran, or why it did not run now.
 *
 * **Two sentences in one column, and both are measurements.** A Check with an
 * earlier run compares against it, which is what makes a set worth reading: a
 * row that says `passed at attempt 1 · absent at attempt 2` is a case that
 * stopped existing, and no count can carry that. A Check on its first run that
 * was skipped carries the record's own reason instead — the paths it covers and
 * this step did not touch, which is the first thing a reader asks and the whole
 * answer to it.
 *
 * `undefined` on a first-attempt Check that ran. There is nothing measured to
 * compare it with, and the parent-commit comparison this column was drawn for
 * has no producer anywhere in the workspace.
 */
function againstOf(run: CheckRun, before: CheckRun | undefined): string | undefined {
  if (before === undefined) {
    return run.outcome === SKIPPED ? run.produced : undefined;
  }
  const then = verbOf(before);
  const now = verbOf(run);
  return then === now
    ? `identical at attempt ${before.attempt} and attempt ${run.attempt}`
    : `${then} at attempt ${before.attempt} · ${now} at attempt ${run.attempt}`;
}

/**
 * The Check's own outcome as one word, from `enum-verbs.toml`.
 *
 * **Never a word chosen here.** `check-outcomes.toml` owns all six spellings
 * and the registry carries the verb for each; a column that wrote its own would
 * be the second place a Check's outcome is named.
 */
function verbOf(run: CheckRun): string {
  return CHECK_OUTCOME[run.outcome]?.verb ?? run.outcome;
}

/**
 * The last run of each Check *before* the attempt `assertedIn` is showing, by
 * name.
 *
 * **Before the shown attempt, not before the step's own current one.** A
 * rerun gate can leave the step's newest attempt with no Checks at all, in
 * which case `assertedIn` is already showing an earlier attempt's rows — and
 * comparing those against runs before the step's *raw* current attempt would
 * compare that attempt to itself. `shown` is `onlyCurrentAttempt`'s own
 * answer, so this reads "before the rows on screen" whichever attempt those
 * turned out to be.
 *
 * Later attempts overwrite earlier ones, so what is left is the most recent
 * earlier run — the one a reader means by "last time", not the first one ever.
 */
function previousRuns(step: StepDetail, shown: readonly CheckRun[]): Map<string, CheckRun> {
  const attempt = shown[0]?.attempt;
  const before = new Map<string, CheckRun>();
  if (attempt === undefined) return before;
  for (const run of step.check_runs) {
    if (run.attempt < attempt) before.set(run.name, run);
  }
  return before;
}

/** The one outcome that advances without measuring anything. */
const SKIPPED = "skipped";

/** What the set is called where it stands under the Checks it is about. */
export const WHAT_THE_SUITE_ASSERTED = "What the suite asserted";
