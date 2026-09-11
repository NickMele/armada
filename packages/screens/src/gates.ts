// What a step's gates found, read once for both surfaces that draw it.
//
// **Two surfaces read `check_runs` and `judged` now, and one of them is new.**
// The phase strip says where a step stands; the Checks and Verdicts chapters
// say what the gates found and let a person argue with it. Those are the same
// rows read for two purposes, and reading them twice is the defect `#321`
// already found once in this file's neighbours — two surfaces stating one
// ordering separately, agreeing until one of them changed.
//
// **Nothing here is JSX, and that is the seam.** What a strip row looks like
// and what a verdict grid looks like are each surface's own answer. What they
// must not disagree about is which attempt's rows count, which member of a
// panel answered what, and whether a criterion was refused — so that is what
// lives here and nothing else does.
//
// The three rules this file holds, each of them from somewhere else:
//
// | Rule | Where it comes from |
// |---|---|
// | Only the current attempt's rows are the live gate | `check_runs` and `judged` hold every attempt since protocol 7.0 |
// | A panel is one row per member and reads as one row per criterion | `Judged.member`, since protocol 7.7 |
// | Any single refusal refuses the criterion | `docs/concepts/judge.md`, unanimity |

import { CHECK_ADVANCES, CRITERION_VERDICT_CHECK } from "@armada/components";
import type { CheckRun, Criterion, DeclaredCheck, Judged, StepDetail } from "@armada/protocol";

import { isSweepMarker, nameOf } from "./declared";
import { onlyCurrentAttempt } from "./facts";

/**
 * One declared Check, and what this attempt's run of it came to.
 *
 * **Declared first, run second.** The list is the step's declaration, so a
 * Check the gate has not reached keeps its row — the shape of what is still
 * coming is part of reading a running Job, and a list built from the runs
 * would make a Job look like it has fewer gates than it has.
 */
export type CheckRead = {
  /** The Check's name, or the built-in's kind. What a citation resolves against. */
  name: string;
  /** The declaration, for the command and the paths it covers. */
  check: DeclaredCheck;
  /** Absent where this attempt's gate has not reached it. */
  run: CheckRun | undefined;
};

/**
 * The step's Checks, each beside what it came to on the current attempt.
 *
 * **Empty is a step that gates on nothing, and so is a step whose workflow
 * Fleet does not hold.** Those are two different sentences and the phase
 * strip's note is where they are told apart — nothing that draws a list of
 * Checks can say either of them, so this answers with no rows and lets the
 * caller draw nothing rather than an empty region.
 */
export function checksOf(step: StepDetail): CheckRead[] {
  // The sweep marker never counts as a Check and never draws as one — it
  // declares that the step gates on everything the repository declares, and
  // nothing ever runs it. `run.ts`'s tier count reads this same filter, on
  // this same list, for the reason atop this file.
  const declared = (step.checks ?? []).filter((check) => !isSweepMarker(check));
  const runs = onlyCurrentAttempt(step, step.check_runs);
  return declared.map((check) => {
    const name = nameOf(check);
    return { name, check, run: runs.find((ran) => ran.name === name) };
  });
}

/** A Check that did not pass, read off the registry's own `advances`. */
export function didNotPass(run: CheckRun): boolean {
  return CHECK_ADVANCES[run.outcome] === false;
}

/**
 * What a tier the run has not got to stands at.
 *
 * **The registry's word, and `criterion_verdict_check` owns it**: the five
 * `check_outcome` carries are what a Check that *ran* did, and `icons.toml`
 * already calls "not reached" a Check state. Once here for every surface that
 * says it, rather than typed at each.
 */
export const NOT_REACHED = CRITERION_VERDICT_CHECK.not_reached?.verb ?? "not_reached";

/** The Checks that ran on this attempt, and the ones among them that failed. */
export function howTheChecksWent(reads: readonly CheckRead[]): {
  ran: CheckRead[];
  failed: CheckRead[];
} {
  const ran = reads.filter((read) => read.run !== undefined);
  return { ran, failed: ran.filter((read) => read.run !== undefined && didNotPass(read.run)) };
}

/**
 * What the Checks tier stands at, in words — `3 of 3 passed`.
 *
 * **One sentence, because two surfaces say it.** The phase strip's tier and the
 * Checks chapter's header sit four inches apart, and a count written twice is a
 * count that disagrees with itself the day `skipped` stops advancing a step.
 */
export function checksStand(reads: readonly CheckRead[]): string {
  const { ran, failed } = howTheChecksWent(reads);
  if (ran.length === 0) return NOT_REACHED;
  return failed.length > 0
    ? `${failed.length} of ${reads.length} did not pass`
    : `${ran.length} of ${reads.length} passed`;
}

/**
 * Which Check's output an `o` press opens on this step, or none.
 *
 * **The failed one first.** A person reaching for an output on a step that
 * stopped wants the Check that says why; on a step where nothing failed there
 * is still a reading worth opening, so the first output there is answers
 * instead of nothing.
 *
 * **Narrowed to the current attempt, like everything else here.** Read across
 * every attempt this surfaced a stale run's output the moment a step had been
 * worked more than once — and the key and the Checks chapter's own act both
 * come through here, so the two cannot open different files.
 */
export function outputOf(step: StepDetail | undefined): string | undefined {
  return outputRunOf(step)?.output_path;
}

/**
 * Which Check's output a reader is offered on this step, whole.
 *
 * **One reading behind three surfaces.** The header act opens the file, `o`
 * opens the file, and the chapter now reads it into the panel — three ways of
 * asking one question, and three answers to it would put a person reading one
 * Check's output under a button that opens another's. `outputOf` is this,
 * narrowed to the path, and exists because two callers only ever wanted that.
 *
 * The row rather than the path, because a reader has to say whose output it is
 * showing: a console pane with a file name and no Check name is a transcript
 * nobody can attribute.
 */
export function outputRunOf(step: StepDetail | undefined): CheckRun | undefined {
  if (step === undefined) return undefined;
  const runs = onlyCurrentAttempt(step, step.check_runs).filter(
    (run) => run.output_path !== undefined,
  );
  return runs.find(didNotPass) ?? runs[0];
}

/**
 * One criterion, and what the panel that answered it made of it.
 *
 * **A panel sends one row per member and a person reads one row per
 * criterion.** At `panel_size: 3` three rows arrive carrying the same
 * `criterion_id` and differing only in `member`; drawn straight they are the
 * same sentence three times.
 */
export type Panel = {
  criterionId: string;
  /**
   * The Job's own criterion, where the id joins. **Absent is a criterion this
   * Job does not carry** — a verdict on something the frozen list has no row
   * for, which is a fact to draw rather than one to hide.
   */
  criterion: Criterion | undefined;
  /**
   * The criterion's frozen 1-based position in `acceptance_criteria`. What a
   * citation to `02` means, and never the row's place on screen. Absent where
   * the id does not join, because a position cannot be invented for it.
   */
  ordinal: number | undefined;
  /**
   * Every member that answered, in panel order. **One entry at `panel_size:
   * 1`**, whose `member` is absent — the convention `Judged.member` keeps.
   */
  members: Judged[];
  /** The members that refused. Empty is a criterion nothing objected to. */
  refused: Judged[];
  /** Unanimity, from `docs/concepts/judge.md`: one veto refuses the criterion. */
  verdict: "met" | "not_met";
};

/**
 * A step's verdicts as one entry per criterion, in the order they were asked.
 *
 * **First-appearance order, never sorted.** The order is the order the criteria
 * were asked, which is the order they were written, and a criterion may be
 * appended but never reordered — so position is stable and is what a citation
 * to `02` means.
 *
 * **Narrowed to the current attempt.** `judged` holds every attempt's rows
 * since protocol 7.0, and both surfaces draw the live gate.
 */
export function panelsOf(step: StepDetail, criteria: readonly Criterion[]): Panel[] {
  const held = new Map<string, Judged[]>();
  for (const one of onlyCurrentAttempt(step, step.judged)) {
    const already = held.get(one.criterion_id);
    if (already === undefined) held.set(one.criterion_id, [one]);
    else already.push(one);
  }
  return [...held].map(([criterionId, answered]) => {
    // In panel order rather than in arrival order. `member` is a position, and
    // a grid whose columns changed order between two criteria would put one
    // judge's mark under another judge's heading.
    const members = [...answered].sort((a, b) => (a.member ?? 1) - (b.member ?? 1));
    const at = criteria.findIndex((held) => held.criterion_id === criterionId);
    const refused = members.filter((one) => one.verdict !== "met");
    return {
      criterionId,
      criterion: at === -1 ? undefined : criteria[at],
      ordinal: at === -1 ? undefined : at + 1,
      members,
      refused,
      verdict: refused.length === 0 ? "met" : "not_met",
    };
  });
}

/** How many criteria the step's declaration says the panel will answer. */
export function askedOf(step: StepDetail): number {
  return (step.judge_checks ?? []).reduce((sum, judge) => sum + judge.criteria, 0);
}

/**
 * Whether this attempt's panel was asked and never answered, rather than
 * never asked at all. Both draw `judged` empty for the current attempt, and
 * they are not the same sentence: one is a call still ahead of the step, the
 * other is one Fleet already made and could not read back.
 *
 * **`last_verdict.trigger` and not a live `judging` read**, because a step
 * open after the Job stopped carries no in-flight call — `judging` is
 * "right now", and there is no "now" left to name once the Job is over.
 */
export function stoppedUndecided(step: StepDetail): boolean {
  return step.last_verdict?.trigger === "gate_undecided";
}

/**
 * Fleet's own sentence, corrected to this screen's case. Fleet writes what it
 * logged, lower-case and unpunctuated like every other log line; everywhere
 * this crosses onto a sentence of its own the two surfaces that draw it would
 * otherwise punctuate it two different ways.
 */
export function sentenceOf(said: string): string {
  const capped = said.charAt(0).toUpperCase() + said.slice(1);
  return /[.!?]$/.test(capped) ? capped : `${capped}.`;
}

/**
 * How many judges answer each criterion.
 *
 * **The declaration first, the rows second.** `panel_size` is what the header
 * of a verdict grid is drawn from before a single row arrives, and it is
 * **absent at one** — the convention `Judged.member` keeps, so a value always
 * means a panel. The rows are the floor under it: a panel that answered with
 * more members than were declared is a fact, not a reason to draw fewer marks
 * than there are.
 */
export function panelSizeOf(step: StepDetail, panels: readonly Panel[]): number {
  const declared = (step.judge_checks ?? []).map((judge) => judge.panel_size ?? 1);
  return Math.max(1, ...declared, ...panels.map((panel) => panel.members.length));
}

/**
 * What one member of a panel is called — `j2`.
 *
 * **A position, not a person, and never part of a citation.** `Judged.member`
 * says so, and it is absent at `panel_size: 1`, where the panel is one judge
 * and the number would be a count nobody asked for.
 */
export function judgeNamed(one: Judged): string {
  return `j${one.member ?? 1}`;
}

/**
 * A run of judges, in a sentence — `j2`, `j1 and j3`, `j1, j2 and j3`.
 *
 * Written here rather than at each caller because the Checks row, the split and
 * the silence line all name the same set and would each have found their own
 * comma.
 */
export function judgesSaid(members: readonly Judged[]): string {
  const named = members.map(judgeNamed);
  if (named.length <= 1) return named.join("");
  return `${named.slice(0, -1).join(", ")} and ${named[named.length - 1]}`;
}
