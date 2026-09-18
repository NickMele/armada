// A step's rows as the Drone narrated them, placed under the plan task it had
// marked working when each happened. #1185.
//
// **Placed by the windows Fleet sends, and by nothing else.** A call is never
// matched to a task by its path or its words: a Drone that never calls
// `update_task` leaves no windows, and all of its work is outside any task.
import type { PlanTask, Turn, WorkPlan } from "@armada/protocol";

import { instant, lasting } from "./duration";
import type { LogRow } from "./story";
import { workingOf } from "./working";

/** One sentence the Drone said, and the rows after it up to the next. */
export type Beat = {
  /** The first row's id — the sentence's own, where there is one. */
  id: string;
  /** The raw instant of the first row. */
  ts?: string;
  /** The sentence, verbatim. Absent on work before the Drone said anything. */
  said?: string;
  /** The rows behind the sentence, without the sentence itself. */
  rows: LogRow[];
  calls: number;
  /** The tools those calls reached for, each once, in the order first used. */
  tools: string[];
  /** A row under it went wrong, so it is never drawn folded. */
  wrong: boolean;
};

/** One task's share of the step, or the share outside any task. */
export type TaskWork = {
  /** Absent is the work outside any task. */
  task?: PlanTask;
  /** The sentences drawn. A bound can leave this empty on a task fully worked. */
  beats: Beat[];
  /** Every call it held, counted whether or not its beat survived the bound. */
  calls: number;
  /** First row to last, where both instants read. Bounded or not, the whole of it. */
  took?: string;
  /** Holds the step's newest row, so it is the one drawn open. */
  newest: boolean;
};

export type Narration = {
  /** Absent where the Job has no plan: the sentences draw with no headings. */
  plan?: { done: number; total: number; states: ("open" | "working" | "done")[] };
  /** The work outside any task first, then every task in plan order. */
  sections: TaskWork[];
  /**
   * Entries the bound left out, every one of them older than what is drawn.
   * Zero where nothing was left out, which is every reading with no bound.
   */
  earlier: number;
};

/**
 * The task a turn at `ts` belongs to, by id.
 *
 * **Two tasks marked working at once overlap, and the one entered last wins** —
 * it is the one the Drone turned to most recently.
 */
export function taskAt(ts: string, tasks: readonly PlanTask[]): string | undefined {
  const at = instant(ts);
  if (at === null) return undefined;
  let best: { id: string; entered: number } | undefined;
  for (const task of tasks) {
    for (const window of task.working_windows ?? []) {
      const entered = instant(window.entered);
      const left = window.left === undefined ? null : instant(window.left);
      if (entered === null || at < entered || (left !== null && at >= left)) continue;
      if (best === undefined || entered >= best.entered) best = { id: task.id, entered };
    }
  }
  return best?.id;
}

/**
 * The step's rows as sentences, under the task each belongs to.
 *
 * `mostEntries` bounds what is drawn to that many of the step's own entries,
 * newest last — the log chapter's entries, the same ones its header counts.
 * **The bound is over the whole reading and not over one section**, because it
 * stands for what a person sees on the page: a cap applied per task would be
 * that many rows times however many tasks the step has touched. Absent draws
 * every row, which is the log sheet.
 *
 * **What is counted is only what is drawn; what is said is counted over
 * everything.** A task's heading still reads the calls and the time of all the
 * work it holds, because that figure is the task's, not the window's — only the
 * beats under it are trimmed.
 */
export function narrationOf(
  rows: readonly LogRow[],
  turns: readonly Turn[],
  stepId: string,
  plan: WorkPlan | undefined,
  mostEntries?: number,
): Narration {
  const turnOf = new Map(turns.map((turn) => [String(turn.seq), turn]));
  const wrong = new Set<string>();
  for (const act of workingOf(turns, stepId).acts) {
    if (act.wrong !== true) continue;
    wrong.add(act.id);
    if (act.answeredId !== undefined) wrong.add(act.answeredId);
  }

  const tasks = plan?.tasks ?? [];
  const outside: TaskWork = { beats: [], calls: 0, newest: false };
  const byTask = new Map<string, TaskWork>(
    tasks.map((task) => [task.id, { task, beats: [], calls: 0, newest: false }]),
  );

  // The first entry the bound keeps. Everything before it is walked for the
  // headings' own figures and then dropped.
  const from = mostEntries === undefined ? 0 : Math.max(0, rows.length - mostEntries);

  // Walked in row order, so every row is placed exactly once. A row whose
  // instant will not read stays where the row before it went.
  let placed: TaskWork = outside;
  let beat: Beat | undefined;
  const spans = new Map<TaskWork, { from?: string; to?: string }>();
  for (const [index, row] of rows.entries()) {
    const drawn = index >= from;
    const turn = turnOf.get(row.id);
    if (turn !== undefined) {
      const id = taskAt(turn.ts, tasks);
      const next = id === undefined ? outside : (byTask.get(id) ?? outside);
      if (next !== placed) beat = undefined;
      placed = next;
      const span = spans.get(placed) ?? {};
      spans.set(placed, { from: span.from ?? turn.ts, to: turn.ts });
    }
    if (row.kind === "said" && row.actor !== "armada") {
      // **A sentence older than the bound takes its heading with it.** The
      // calls it still owns start a beat of their own, so they draw under
      // their own fold line rather than under a sentence a reader cannot see
      // — and the sentence is one press away in the log.
      beat = undefined;
      if (!drawn) continue;
      beat = {
        id: row.id,
        ...(turn === undefined ? {} : { ts: turn.ts }),
        said: turn?.saw.event === "said" ? turn.saw.text : row.message,
        rows: [],
        calls: 0,
        tools: [],
        wrong: false,
      };
      placed.beats.push(beat);
      continue;
    }
    // The section's own figure, counted over every row it holds and not over
    // the ones that survived the bound.
    if (turn?.saw.event === "called") placed.calls += 1;
    if (!drawn) continue;
    if (beat === undefined) {
      beat = {
        id: row.id,
        ...(turn === undefined ? {} : { ts: turn.ts }),
        rows: [],
        calls: 0,
        tools: [],
        wrong: false,
      };
      placed.beats.push(beat);
    }
    beat.rows.push(row);
    if (wrong.has(row.id)) beat.wrong = true;
    if (turn?.saw.event === "called") {
      beat.calls += 1;
      if (!beat.tools.includes(turn.saw.tool)) beat.tools.push(turn.saw.tool);
    }
  }
  placed.newest = rows.length > 0;

  for (const [work, span] of spans) {
    const from = span.from === undefined ? null : instant(span.from);
    const to = span.to === undefined ? null : instant(span.to);
    if (from !== null && to !== null && work.task !== undefined) work.took = lasting(to - from);
  }

  const bar = planBarOf(plan);
  return {
    ...(bar === undefined ? {} : { plan: bar }),
    sections: [
      // **The outside row goes when it has no beats, and the bound can be what
      // took them.** Unlike a task it has no heading to stand on its own, so a
      // row saying `Outside any task` over nothing would be a label for work
      // that is only in the log.
      ...(outside.beats.length === 0 ? [] : [outside]),
      ...tasks.map((task) => byTask.get(task.id) as TaskWork),
    ],
    earlier: from,
  };
}

/** Tasks done over tasks not dropped, and each one's segment, or nothing with no plan. */
export function planBarOf(plan: WorkPlan | undefined): Narration["plan"] {
  if (plan === undefined) return undefined;
  const counted = plan.tasks.filter((task) => task.state !== "dropped");
  return {
    done: counted.filter((task) => task.state === "done").length,
    total: counted.length,
    states: counted.map((task) =>
      task.state === "done" ? "done" : task.state === "working" ? "working" : "open",
    ),
  };
}

/** `3 calls · Edit, Read`, and `so far` on the sentence still being worked. */
export function callsSaid(beat: Beat, live: boolean): string {
  const count = `${beat.calls} ${beat.calls === 1 ? "call" : "calls"}${live ? " so far" : ""}`;
  // The log chapter's own word for what it counts.
  if (beat.calls === 0) return `${beat.rows.length} ${beat.rows.length === 1 ? "entry" : "entries"}`;
  return [count, beat.tools.join(", ")].join(" · ");
}

/**
 * `31 calls · 5m 02s`, a task's heading. Nothing on a task not worked here.
 *
 * **Read off what the task held and not off what is drawn.** It was the beats,
 * which said the same thing until a bound could take every one of them: a task
 * worked for five minutes then drew as a task nobody had touched.
 */
export function workSaid(work: TaskWork): string | undefined {
  if (work.took === undefined && work.calls === 0) return undefined;
  const calls = `${work.calls} ${work.calls === 1 ? "call" : "calls"}`;
  return work.took === undefined ? calls : `${calls} · ${work.took}`;
}
