// Where a Job that is over stopped, and what the gate found there. Named for
// what it answers now; it was `rail.ts` and held the rail as well.
//
// **What was here was the rail, and job detail no longer draws one.** The run
// is a tree in `run.ts` and a step's gates are the phase strip's in
// `phases.tsx`, so `railOf` and everything it needed went with them. The
// `WorkflowRail` component is still drawn, by the proposal-time workflow
// preview, which builds its rows in `preview.ts` and never came through here.
//
// What is left is the one line a Job that stopped owes: where it ended, and
// which Check ended it. Every part of it is served — the step's label, the
// run's outcome, and the file it wrote.

import { CHECK_ADVANCES, CHECK_OUTCOME } from "../../shared/generated/vocabulary";
import type { CheckRun, JobDetail as JobWhole } from "../../shared/protocol";
import { ordered } from "./facts";

/** Where a Job that is over stopped, and what the gate found there. */
export type StoppedAt = {
  /** The step's name, as Fleet gave it. */
  label: string;
  /** Whether that name is the `step_id`, so it renders in mono. */
  labelIsAnIdentifier: boolean;
  /** The Check that did not pass, and what it did. Absent where none did. */
  check?: string;
  /** Where that Check wrote its output. Absent where it wrote no file. */
  outputPath?: string;
};

/**
 * Which step the Job stopped at, and the Check that stopped it there.
 *
 * The rail already draws all of this a row at a time. What a Job that is over
 * owes is the one line saying where it ended, which is the first thing read on
 * the dead-end screen — and every part of it is served: the step's label, the
 * run's outcome, and the file it wrote.
 *
 * **The step is Fleet's, not a guess.** `current_step_id` is frozen at the
 * failed step, so a Job whose current step Fleet cannot name says nothing here
 * rather than picking a row off the states.
 */
export function stoppedAt(whole: JobWhole): StoppedAt | undefined {
  const step = ordered(whole).find((held) => held.step_id === whole.job.current_step_id);
  if (step === undefined) return undefined;
  const run = step.check_runs.find(didNotPass);
  const said = run === undefined ? undefined : resultOf(run);
  return {
    label: step.label,
    labelIsAnIdentifier: step.label === step.step_id,
    check: run === undefined ? undefined : said === undefined ? run.name : `${run.name} · ${said}`,
    outputPath: run?.output_path,
  };
}

/**
 * A Check that stopped the step, read off `check-outcomes.toml`'s own
 * `advances` rather than off a status token.
 *
 * **The token cannot answer this and used to be asked.** `skipped` and
 * `never_ran` both carry `--status-not-started`, and only one of them is a
 * failure: a Check the step's paths never reached did not stop anything, and a
 * Check whose command is missing stopped everything. Reading the palette named
 * the first as the reason a Job ended.
 *
 * **A spelling the registry does not hold is claimed neither way.** Calling an
 * unknown outcome a failure would name a Check as the reason a Job ended on no
 * evidence at all.
 */
function didNotPass(run: CheckRun): boolean {
  return CHECK_ADVANCES[run.outcome] === false;
}

/**
 * What one Check did, or nothing where the gate has not run it.
 *
 * **Six outcomes, each drawn as itself.** A pass carries no `produced` because
 * a pass measured nothing; four of the rest say different things about why a
 * step did not advance — an answer that was not what the step declared, a
 * signal, a budget that expired, a command that never started — and folding
 * them into "failed" would hide the one difference a reader acts on.
 *
 * **`skipped` is the sixth and it stopped nothing.** It reads "not run", with
 * `produced` naming the paths the Check covers and this step did not touch. It
 * has its own glyph on purpose: drawn as a pass it would claim a verification
 * that never happened, and drawn as `never_ran`'s `shield-minus` it would look
 * like a Check whose command is missing.
 *
 * The verb comes from the registry, which now has a row for all six.
 */
function resultOf(run: CheckRun): string | undefined {
  const outcome = CHECK_OUTCOME[run.outcome]?.verb ?? run.outcome;
  // What it was measured against, and what it did. Absent on a pass, where the
  // outcome is the whole sentence.
  const measured = [run.expected, run.produced].filter((part) => part !== undefined);
  return measured.length === 0 ? outcome : `${outcome} · ${measured.join(" → ")}`;
}
