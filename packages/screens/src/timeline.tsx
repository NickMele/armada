// A step as a timeline: one section per attempt, and inside it the phases in
// the order they happen — instructed, working, checks, judge. What the attempt
// wrote rides on working, with the turns that wrote it.
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
import type { ReactNode } from "react";

import type { ChangedFile, CheckRun, Judged, StepAttempt, StepDetail, Turn } from "@armada/protocol";
import type { StepActivity, StepChapter, StepTimelineAttempt } from "@armada/components";

import { span } from "./duration";
import { askedOf, didNotPass } from "./gates";
import { entriesOf } from "./story";

/** Which phase a row is. What an attempt wrote rides on `working`. */
export type TimelinePhase = "instructed" | "working" | "checks" | "judge";

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
  /**
   * What this attempt wrote, on `working`. **The owner's call, 11 Sep 2026**:
   * what a Drone did and what came out of it are one reading, so Produced is
   * not a row of its own — the files sit under the turns that wrote them.
   */
  produced?: ChangedFile[];
  /** What the attempt kept beside them, on `working`: deliverables, then frames. */
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
    // What this attempt wrote, as Fleet read it at the step boundary. The last
    // reading in the window wins, for the reason `run.ts` states: a step read
    // three times has three rows and only the newest describes the work.
    const wrote = producedIn(within);
    // **Counted the way the log counts**, through `entriesOf`, which drops a
    // Drone's echo of its own instruction. Counting the raw window here said
    // 1763 turns on a row whose own body said 1756, and one row cannot hold
    // two numbers for one fact.
    const readable = entriesOf(within, step.step_id).length;

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
        meta: workingSays(readable, span(attempt.started_at, ended ?? now), wrote.length + kept.length),
        ...(working ? { live: true } : {}),
        turns: within,
        ...(wrote.length === 0 ? {} : { produced: wrote }),
        ...(kept.length === 0 ? {} : { kept }),
      },
      checksRow(step, attempt, runs, current),
      judgeRow(step, attempt, ruled, current),
    ];
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

/**
 * `14 turns · 6m 01s · 4 files`, dropping what there is nothing to say about.
 *
 * The turn count is what a reader would count in the log, echoes already out.
 */
function workingSays(turns: number, took: string | null, files: number): string {
  return [
    `${turns} ${turns === 1 ? "turn" : "turns"}`,
    ...(took === null ? [] : [took]),
    ...(files === 0 ? [] : [`${files} ${files === 1 ? "file" : "files"}`]),
  ].join(" · ");
}

/**
 * What an attempt wrote, out of its own turns.
 *
 * **The last reading wins**, which is `run.ts`'s rule for the same event at
 * step level: Fleet takes one at every ruling, and only the newest describes
 * the work as it stands.
 */
function producedIn(turns: readonly Turn[]): ChangedFile[] {
  let files: ChangedFile[] = [];
  for (const turn of turns) {
    if (turn.saw.event === "produced") files = turn.saw.files;
  }
  return files;
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

/**
 * The timeline as the panel draws it: the phases of each attempt, with the
 * step's own chapters arranged into the rows they belong to.
 *
 * **Arranged, never rebuilt.** `chaptersOf` already builds the brief, the log,
 * what was produced and what the gates found, and each carries its own preview,
 * body and act. Building them a second time here would be two readings of one
 * step, which is the drift this repository deletes on sight — so the chapters
 * are placed into the phase that produced them and nothing is derived twice.
 *
 * **Only the attempt being read has bodies.** Every chapter narrows itself to
 * the current attempt, so an earlier attempt draws what this file derived for
 * it — its counts, its outcome and what its gate found — and no body it would
 * have to invent.
 */
export function stepTimelineOf(
  step: StepDetail,
  turns: readonly Turn[],
  now: number,
  /** The chapters the story already built, in the order it built them. */
  chapters: readonly StepChapter[],
): StepTimelineAttempt[] {
  const held = new Map(chapters.map((chapter) => [chapter.id, chapter]));
  return timelineOf(step, turns, now).map((attempt) => ({
    id: attempt.id,
    name: `Attempt ${attempt.attempt}`,
    ...(saidOf(attempt) === undefined ? {} : { said: saidOf(attempt) }),
    current: attempt.current,
    rows: attempt.rows.map((row) => ({
      id: `${attempt.id}-${row.id}`,
      name: row.name,
      activity: row.mark,
      ...(row.meta === undefined ? {} : { meta: row.meta }),
      ...(row.live === true ? { live: true } : {}),
      ...bodyOf(row.phase, attempt.current ? held : new Map()),
    })),
  }));
}

/** Which chapters belong to which phase, in the order the phase produced them. */
const CHAPTERS: Record<TimelinePhase, readonly string[]> = {
  instructed: ["instructions"],
  // What the Drone did and what came out of it are one reading — the owner's
  // call, 11 Sep 2026 — so the log, what it showed and what it wrote all sit
  // under the phase that produced them.
  working: ["log", "shown", "produced"],
  checks: ["checks"],
  judge: ["verdicts"],
};

/** A row's body and act, from the chapters the phase owns. */
function bodyOf(
  phase: TimelinePhase,
  held: Map<string, StepChapter>,
): { body?: ReactNode; act?: ReactNode } {
  const mine = CHAPTERS[phase].flatMap((id) => {
    const chapter = held.get(id);
    return chapter === undefined ? [] : [chapter];
  });
  if (mine.length === 0) return {};
  // One act to a row: the first chapter that carries one. A phase owning three
  // chapters has at most one thing worth leaving the panel for.
  const act = mine.find((chapter) => chapter.act !== undefined)?.act;
  return {
    body: mine.map((chapter) => (
      <section key={chapter.id}>
        {mine.length === 1 ? null : <span className="caps">{chapter.title}</span>}
        {chapter.preview}
        {chapter.content}
      </section>
    )),
    ...(act === undefined ? {} : { act }),
  };
}

/**
 * What became of an attempt, in words rather than in the wire's own.
 *
 * **`retrying` is "handed back".** It is the step's word for what happens next,
 * and on an attempt that has ended it reads as a Drone still working.
 */
function saidOf(attempt: TimelineAttempt): string | undefined {
  const said =
    attempt.outcome === "retrying"
      ? "handed back"
      : attempt.outcome === "advanced"
        ? "advanced"
        : attempt.outcome === "stopped"
          ? "stopped"
          : attempt.outcome;
  return attempt.took === undefined ? said : `${said} · ${attempt.took}`;
}
