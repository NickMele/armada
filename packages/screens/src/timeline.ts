// A step as a timeline: one section per attempt, and inside it the phases in
// the order they happen — instructed, working, checks, judge, and whatever the
// attempt kept.
//
// **Derived here and drawn in one draft.** `Drafts/Step timeline` is the only
// caller. The panel it is meant to replace still draws the strip and the
// chapters beneath it, and the owner asked to see this before either moves.
//
// # What the wire can and cannot say
//
// **An attempt is the spine.** `StepAttempt` carries the only times a step's
// insides have: when the attempt began and when it ended. Turns carry their own
// instant, so they fall into an attempt's window by their timestamp.
//
// **Checks and rulings carry an attempt and no time.** So inside an attempt
// their order is the phase order — instructed, working, checks, judge — which
// is the order the strip already draws and the order Fleet runs them in. A
// timeline that claimed measured times for them would be inventing them.
import type { CheckRun, Judged, StepAttempt, StepDetail, Turn } from "@armada/protocol";
import type { StepActivity } from "@armada/components";

import { span } from "./duration";
import { askedOf, didNotPass } from "./gates";

/** Which phase a row is. `kept` is what the attempt left behind. */
export type TimelinePhase = "instructed" | "working" | "checks" | "judge" | "kept";

/** One phase of one attempt. */
export type TimelineRow = {
  id: string;
  phase: TimelinePhase;
  name: string;
  /** The mark the row carries, from the step-activity set. */
  mark: StepActivity;
  /** What the row says folded. */
  meta?: string;
  /** The Drone is in this row now. */
  live?: boolean;
  /** This attempt's turns, on `working` and on `instructed`. */
  turns?: Turn[];
  /** This attempt's runs, on `checks`. */
  runs?: CheckRun[];
  /** This attempt's rulings, on `judge`. */
  judged?: Judged[];
  /** What the attempt kept, on `kept`: deliverables first, then frames. */
  kept?: string[];
};

/** One attempt of a step, with its phases inside it. */
export type TimelineAttempt = {
  id: string;
  attempt: number;
  /** The wire's own word: `advanced`, `retrying`, `stopped`, `running`. */
  outcome: string;
  /** Why it ended that way, where Fleet said. */
  why?: string;
  /** How long it ran. */
  took?: string;
  /** The last attempt, which is the one a person is reading. */
  current: boolean;
  rows: TimelineRow[];
};

/**
 * The step, attempt by attempt.
 *
 * **A step with no attempts still draws one.** Fleet sends none until the first
 * one is recorded, and a step a person is looking at has begun — so the step's
 * own state and `entered_at` stand in, rather than the panel drawing nothing.
 */
export function timelineOf(
  step: StepDetail,
  turns: readonly Turn[],
  now: number,
): TimelineAttempt[] {
  const spine: StepAttempt[] =
    step.attempts.length > 0
      ? [...step.attempts].sort((a, b) => a.attempt - b.attempt)
      : [{ attempt: 1, outcome: step.state, started_at: step.entered_at }];
  const mine = turns.filter((turn) => turn.step === undefined || turn.step === step.step_id);

  return spine.map((attempt, at) => {
    // An attempt that has not ended runs until the next one began, and the last
    // one runs until now. Fleet ends an attempt when it hands the step back, so
    // the two are the same instant on every attempt but the last.
    const ended = attempt.ended_at ?? spine[at + 1]?.started_at;
    const current = at === spine.length - 1;
    const within = mine.filter(
      (turn) => turn.ts >= attempt.started_at && (ended === undefined || turn.ts < ended),
    );
    const runs = step.check_runs.filter((run) => run.attempt === attempt.attempt);
    const ruled = step.judged.filter((one) => one.attempt === attempt.attempt);
    const kept = [
      ...(step.deliverables ?? [])
        .filter((one) => one.attempt === attempt.attempt)
        .map((one) => one.path),
      ...(step.frames ?? []).filter((one) => one.attempt === attempt.attempt).map((one) => one.name),
    ];
    const working = current && ended === undefined && WORKING.has(step.state);

    const rows: TimelineRow[] = [
      {
        id: `${attempt.attempt}-instructed`,
        phase: "instructed",
        name: "Instructed",
        mark: "advanced",
        turns: within.filter((turn) => turn.saw.event === "instructed"),
      },
      {
        id: `${attempt.attempt}-working`,
        phase: "working",
        name: "Working",
        mark: working ? "running" : within.length === 0 ? "not_started" : "advanced",
        meta: workingSays(within.length, span(attempt.started_at, ended ?? now)),
        ...(working ? { live: true } : {}),
        turns: within,
      },
      checksRow(step, attempt, runs, current),
      judgeRow(step, attempt, ruled, current),
    ];
    if (kept.length > 0) {
      rows.push({
        id: `${attempt.attempt}-kept`,
        phase: "kept",
        name: "Kept",
        mark: "advanced",
        meta: `${kept.length} ${kept.length === 1 ? "file" : "files"}`,
        kept,
      });
    }

    return {
      id: `attempt-${attempt.attempt}`,
      attempt: attempt.attempt,
      outcome: attempt.outcome,
      ...(attempt.why === undefined ? {} : { why: attempt.why }),
      ...(took(attempt, ended, now) === undefined ? {} : { took: took(attempt, ended, now) }),
      current,
      rows,
    };
  });
}

/** The step states in which a Drone is working right now. */
const WORKING = new Set(["running", "retrying"]);

/** `14 turns · 6m 01s`, or just the count where there is no span to say. */
function workingSays(turns: number, took: string | null): string {
  const counted = `${turns} ${turns === 1 ? "turn" : "turns"}`;
  return took === null ? counted : `${counted} · ${took}`;
}

/** How long an attempt ran, or nothing where it has not begun to. */
function took(attempt: StepAttempt, ended: string | undefined, now: number): string | undefined {
  return span(attempt.started_at, ended ?? now) ?? undefined;
}

/**
 * This attempt's Checks.
 *
 * **`not_started` is a Check that was not reached, and it is not `skipped`** —
 * that word is an outcome the registry owns for a Check the gate decided not to
 * run, and a gate that never got this far decided nothing.
 */
function checksRow(
  step: StepDetail,
  attempt: StepAttempt,
  runs: CheckRun[],
  current: boolean,
): TimelineRow {
  const declared = step.checks?.length ?? 0;
  const running = current && step.checking?.attempt === attempt.attempt;
  const failed = runs.filter(didNotPass);
  const mark: StepActivity =
    runs.length === 0 ? (running ? "running" : "not_started") : failed.length > 0 ? "failed" : "advanced";
  const meta =
    runs.length === 0
      ? declared === 0
        ? "none declared"
        : running
          ? "running"
          : "not reached"
      : failed.length > 0
        ? `${failed.map((run) => run.name).join(", ")} · ${failed.length} of ${runs.length} did not pass`
        : `${runs.length} of ${Math.max(declared, runs.length)} passed`;
  return {
    id: `${attempt.attempt}-checks`,
    phase: "checks",
    name: "Checks",
    mark,
    meta,
    ...(running ? { live: true } : {}),
    runs,
  };
}

/**
 * This attempt's Judge.
 *
 * **A criterion is refused where any member of its panel did not meet it**,
 * which is the rule `panelsOf` states for the gate's own reading. Counted by
 * criterion rather than by ruling, so a panel of three does not read as three
 * refusals of one criterion.
 */
function judgeRow(
  step: StepDetail,
  attempt: StepAttempt,
  ruled: Judged[],
  current: boolean,
): TimelineRow {
  const asked = askedOf(step);
  const asking = current && step.judging !== undefined;
  const criteria = new Map<string, boolean>();
  for (const one of ruled) {
    criteria.set(one.criterion_id, (criteria.get(one.criterion_id) ?? true) && one.verdict === "met");
  }
  const met = [...criteria.values()].filter(Boolean).length;
  const refused = criteria.size - met;
  const mark: StepActivity =
    criteria.size === 0 ? (asking ? "running" : "not_started") : refused > 0 ? "failed" : "advanced";
  const meta =
    criteria.size === 0
      ? asked === 0
        ? "no judge declared"
        : asking
          ? `asking · ${asked} ${asked === 1 ? "criterion" : "criteria"}`
          : `${asked} ${asked === 1 ? "criterion" : "criteria"}, not asked`
      : refused > 0
        ? `${refused} of ${criteria.size} refused`
        : `${met} of ${criteria.size} met`;
  return {
    id: `${attempt.attempt}-judge`,
    phase: "judge",
    name: "Judge",
    mark,
    meta,
    ...(asking ? { live: true } : {}),
    judged: ruled,
  };
}
