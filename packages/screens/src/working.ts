// What a Drone did inside an attempt, folded to a size a panel can draw.
//
// **One derivation, two drawings.** `Drafts/Working grouped` and `Drafts/Working
// waterfall` are both drawn from this, so what separates them on screen is the
// presentation and not the arithmetic. The owner asked on 11 Sep 2026 for two
// shapes of the Working body to pick between, and two derivations would have
// made that a comparison of two different readings of the same step.
//
// # What the wire made us decide
//
// **A call and its answer are one thing that happened.** The transcript carries
// `called` and `answered` as two rows, and drawn as two they doubled the length
// of the densest body on the screen while saying one fact — the recorded step's
// 351 calls arrive as 702 rows. So the answer folds into its call and becomes
// the call's duration and its outcome.
//
// **The gaps are the finding, and nothing here hides them.** On the recorded
// `implement` step the calls were open for 8m 07s of a 43m 36s attempt, and
// 6m 31s of that is two `TaskOutput` waits. The other 35m is a model thinking.
// A body that sums call time and calls it the step's duration would report the
// wrong thing by a factor of five, so `wall` and `inCalls` are separate figures
// and a caller draws both.
import type { ChangedFile, CheckRun, Turn } from "@armada/protocol";

import { instant } from "./duration";
import { didNotPass } from "./gates";
import { isEcho } from "./story";

/** What one act is. The wire's own kinds, less the two that fold into others. */
export type WorkingKind =
  | "opened"
  | "instructed"
  | "call"
  | "said"
  | "checked"
  | "produced"
  | "refused"
  | "waited"
  | "ended"
  | "unreadable";

/** One thing the Drone did, with its answer already folded in. */
export type WorkingAct = {
  /** The socket's own sequence, so a row is stable across re-renders. */
  id: string;
  kind: WorkingKind;
  /** The raw instant. Formatted by whoever draws it, never here. */
  ts: string;
  /** The one line the act shows. */
  said: string;
  /** Machine-derived, so it is set in mono. */
  mono?: boolean;
  /** The tool a call reached for, which is what a run folds on. */
  tool?: string;
  /** How long the call was open, where its answer arrived. */
  ms?: number;
  /**
   * The answer's own row, where one arrived.
   *
   * **Because a surface drawing rows still has to draw it.** The answer folds
   * into its call *here*, which is what makes one act out of two rows — but a
   * caller that renders the transcript itself would drop 351 rows on one real
   * step if this did not say which they were.
   */
  answeredId?: string;
  /** The call failed, the Check did not pass, or the harness refused. */
  wrong?: boolean;
  /** The run, on `checked`, so a body can open its output. */
  run?: CheckRun;
  /** What was written, on `produced`. */
  files?: ChangedFile[];
  /** Why the harness refused, on `refused`. */
  because?: string;
};

/** A kind of row this Bridge cannot draw, and how many arrived. */
export type UnreadKind = { kind: string; count: number };

/** An attempt's work, folded. */
export type Working = {
  acts: WorkingAct[];
  /**
   * The rows no surface draws, by the wire's own kind.
   *
   * **Counted by kind rather than listed.** 993 of the recorded step's 1763
   * rows are these, 757 of them one kind, and a body that draws them is a body
   * nobody scrolls. `hideUnread` already drops them from the log for the same
   * reason; this says what was dropped so the hiding stays honest.
   */
  unread: UnreadKind[];
  /** First act to last, in milliseconds. What the attempt actually took. */
  wall: number;
  /** How much of that a call was open for. Never the same figure as `wall`. */
  inCalls: number;
};

/**
 * An attempt's turns as acts.
 *
 * **Armada's echo of its own instruction is out**, through `isEcho`, which is
 * the log's rule and not a second one written here.
 */
export function workingOf(turns: readonly Turn[], stepId: string | undefined): Working {
  const mine = turns
    .filter((turn) => stepId === undefined || turn.step === undefined || turn.step === stepId)
    .filter((turn) => !isEcho(turn));

  // Answers first, so a call knows its own outcome when it is drawn. Keyed by
  // call id, which is what the wire pairs them on.
  const opened = new Map<string, string>();
  const answered = new Map<string, { ms?: number; failed: boolean; id: string }>();
  for (const turn of mine) {
    if (turn.saw.event === "called") opened.set(turn.saw.call, turn.ts);
    if (turn.saw.event === "answered") {
      const from = opened.get(turn.saw.call);
      const took = from === undefined ? undefined : between(from, turn.ts);
      answered.set(turn.saw.call, {
        ...(took === undefined ? {} : { ms: took }),
        failed: turn.saw.failed,
        id: String(turn.seq),
      });
    }
  }

  const acts: WorkingAct[] = [];
  const unread = new Map<string, number>();
  for (const turn of mine) {
    const act = actOf(turn, answered);
    if (act !== undefined) acts.push(act);
    if (turn.saw.event === "unrecognised") {
      unread.set(turn.saw.kind, (unread.get(turn.saw.kind) ?? 0) + 1);
    }
  }

  return {
    acts,
    unread: [...unread.entries()]
      .map(([kind, count]) => ({ kind, count }))
      .sort((a, b) => b.count - a.count),
    wall: spanOf(acts),
    inCalls: acts.reduce((sum, act) => sum + (act.kind === "call" ? (act.ms ?? 0) : 0), 0),
  };
}

/**
 * One turn as an act, or nothing where it folds into another.
 *
 * **`answered` returns nothing**, because it is already its call's duration,
 * and **`unrecognised` returns nothing**, because it is counted instead.
 */
function actOf(
  turn: Turn,
  answered: Map<string, { ms?: number; failed: boolean; id: string }>,
): WorkingAct | undefined {
  const saw = turn.saw;
  const id = String(turn.seq);
  switch (saw.event) {
    case "called": {
      const answer = answered.get(saw.call);
      return {
        id,
        kind: "call",
        ts: turn.ts,
        said: saw.detail === "" ? saw.tool : `${saw.tool}  ${saw.detail}`,
        mono: true,
        tool: saw.tool,
        ...(answer?.ms === undefined ? {} : { ms: answer.ms }),
        ...(answer === undefined ? {} : { answeredId: answer.id }),
        ...(answer?.failed === true ? { wrong: true } : {}),
      };
    }
    case "said":
      return { id, kind: "said", ts: turn.ts, said: saw.text };
    case "instructed":
      return { id, kind: "instructed", ts: turn.ts, said: "Armada instructed the Drone" };
    case "checked":
      return {
        id,
        kind: "checked",
        ts: turn.ts,
        said: saw.run.name,
        mono: true,
        run: saw.run,
        // **The registry's own reading**, through `didNotPass`. `check-outcomes.toml`
        // carries five outcomes a run can end on and only it knows which advance,
        // so comparing to a spelling here would be a second vocabulary.
        ...(didNotPass(saw.run) ? { wrong: true } : {}),
      };
    case "produced":
      return { id, kind: "produced", ts: turn.ts, said: filesSaid(saw.files), files: saw.files };
    case "refused":
      return {
        id,
        kind: "refused",
        ts: turn.ts,
        said: `The harness refused ${saw.tool}`,
        wrong: true,
        because: saw.because,
      };
    case "started":
      return { id, kind: "opened", ts: turn.ts, said: `A Drone run opened on ${saw.model}` };
    case "ended":
      return { id, kind: "ended", ts: turn.ts, said: "The Drone run ended" };
    case "background_work":
      return {
        id,
        kind: "waited",
        ts: turn.ts,
        said: `${saw.outstanding} running in the background`,
      };
    case "unreadable":
      // **The log draws this and `hideUnread` keeps it**, so an act has to
      // exist for it or the row has nowhere to sit. The wording is `story.ts`'s
      // own, because two sentences for one row is two sentences to disagree.
      return { id, kind: "unreadable", ts: turn.ts, said: "A line the reader could not parse" };
    default:
      return undefined;
  }
}

/** `36 files`, the one line a produced act shows closed. */
function filesSaid(files: readonly ChangedFile[]): string {
  return `${files.length} ${files.length === 1 ? "file" : "files"}`;
}

/** Two instants apart, in milliseconds, or nothing where either will not parse. */
function between(from: string, to: string): number | undefined {
  const start = instant(from);
  const end = instant(to);
  return start === null || end === null ? undefined : Math.max(0, end - start);
}

/**
 * The first act's start to the last act's end.
 *
 * **A call's own duration is part of the attempt, and reading the last act's
 * start alone dropped it.** An attempt whose final act is a two-minute call ran
 * for two minutes longer than its rows say, and a lane scaled to the shorter
 * figure drew that call off its own right edge. The recorded step hid this: its
 * last act is `ended`, which takes no time.
 */
function spanOf(acts: readonly WorkingAct[]): number {
  const first = acts[0];
  if (first === undefined) return 0;
  const from = instant(first.ts);
  if (from === null) return 0;
  let wall = 0;
  for (const act of acts) {
    const at = instant(act.ts);
    if (at === null) continue;
    wall = Math.max(wall, at - from + (act.ms ?? 0));
  }
  return wall;
}

/**
 * A run of acts drawn as one line.
 *
 * **What a build log does when it is too long to scroll**: consecutive work of
 * one kind collapses to a heading with a count and a duration, and opens on a
 * press. The recorded step's 351 calls fall into 202 runs, the longest eleven
 * `Bash` calls in a row.
 */
export type WorkingRun = {
  id: string;
  /** The acts in the run. One is a run of one, and draws as itself. */
  acts: WorkingAct[];
  /** The tool they share, where they are calls. */
  tool?: string;
  /** Drawn folded. False on a run of one, and on any run holding a failure. */
  folded: boolean;
  /** The run's calls added up. */
  ms: number;
  /** How many of the acts went wrong. */
  wrong: number;
};

/**
 * Consecutive calls of one tool as one run.
 *
 * **A run holding a failure never folds.** It is the rule every CI log that
 * folds anything keeps: the group that failed is the group you opened the page
 * for, so it opens itself rather than hiding behind a count.
 */
export function runsOf(acts: readonly WorkingAct[]): WorkingRun[] {
  const runs: WorkingRun[] = [];
  for (const act of acts) {
    const last = runs[runs.length - 1];
    const joins =
      last !== undefined &&
      act.kind === "call" &&
      last.acts[0]?.kind === "call" &&
      last.tool === act.tool;
    if (joins && last !== undefined) {
      last.acts.push(act);
      last.ms += act.ms ?? 0;
      last.wrong += act.wrong === true ? 1 : 0;
    } else {
      runs.push({
        id: act.id,
        acts: [act],
        ...(act.tool === undefined ? {} : { tool: act.tool }),
        folded: false,
        ms: act.ms ?? 0,
        wrong: act.wrong === true ? 1 : 0,
      });
    }
  }
  return runs.map((run) => ({ ...run, folded: run.acts.length > 1 && run.wrong === 0 }));
}

/** One tool, everything it was called for. */
export type ToolTotal = { tool: string; calls: number; ms: number; wrong: number };

/**
 * The calls added up by tool, slowest first.
 *
 * **Kept from the lane that was drawn beside this and rejected**, because it is
 * the one reading grouping cannot give: 351 rows become nine, and the nine say
 * two `TaskOutput` waits hold four fifths of the step's call time. It has no
 * surface yet — the owner asked for it to be kept when the lane went.
 */
export function byToolOf(acts: readonly WorkingAct[]): ToolTotal[] {
  const totals = new Map<string, ToolTotal>();
  for (const act of acts) {
    if (act.kind !== "call" || act.tool === undefined) continue;
    const held = totals.get(act.tool) ?? { tool: act.tool, calls: 0, ms: 0, wrong: 0 };
    held.calls += 1;
    held.ms += act.ms ?? 0;
    held.wrong += act.wrong === true ? 1 : 0;
    totals.set(act.tool, held);
  }
  return [...totals.values()].sort((a, b) => b.ms - a.ms);
}

/** How many files one kind of change covers. The spelling is the wire's. */
export type ChangeCount = { change: string; files: number };

/**
 * What the attempt wrote — the step's own output, read as one thing.
 *
 * **Produced is not another act, and drawing it as one lost it.** A 36-file
 * write takes no time, so on a lane scaled by duration it is a single tick
 * between two calls, and in a folded list it is one line among two hundred. It
 * is the thing the step is *for*, so both drafts give it a band of its own.
 */
export type Produced = {
  /** Every file, as Fleet last read them. */
  files: ChangedFile[];
  /** Counted by the change, most first. Never a spelling invented here. */
  changes: ChangeCount[];
  /** How many fall outside the plan the step declared. */
  outsidePlan: number;
  /** When Fleet took the reading. */
  at?: string;
};

/**
 * The attempt's output.
 *
 * **The last reading wins**, which is `run.ts`'s rule for the same event: Fleet
 * takes one at every ruling, and only the newest describes the work as it
 * stands. An attempt that wrote nothing gives an empty reading, never `null` —
 * a band that says "no files" is a fact, and an absent band is a question.
 */
export function producedOf(acts: readonly WorkingAct[]): Produced {
  let last: WorkingAct | undefined;
  for (const act of acts) {
    if (act.kind === "produced") last = act;
  }
  const files = last?.files ?? [];
  const counts = new Map<string, number>();
  for (const file of files) counts.set(file.change, (counts.get(file.change) ?? 0) + 1);
  return {
    files,
    changes: [...counts.entries()]
      .map(([change, count]) => ({ change, files: count }))
      .sort((a, b) => b.files - a.files || a.change.localeCompare(b.change)),
    outsidePlan: files.filter((file) => file.outside_plan === true).length,
    ...(last === undefined ? {} : { at: last.ts }),
  };
}
