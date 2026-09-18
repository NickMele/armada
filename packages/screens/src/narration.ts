// A step's rows as the Drone narrated them, placed under the plan task it had
// marked working when each happened. #1185.
//
// **A window places a row. Where no window covers an Edit, the task whose
// `scope` names the file does** — #1498: a task marked `done` without ever
// being marked `working` has no window, so its own declared files read as
// changed outside every task. A path written down in `scope` is a declaration
// and reading it is not guessing; a task's title and its prose are still never
// matched, and no call but an Edit or a Write is ever moved by one.
//
// **A sentence goes with a run a declaration moved whole**, so a reader is not
// left an `Outside any task` heading over nothing while the edits it introduced
// draw elsewhere. A window holding the sentence keeps it, even where every call
// under it then moved: the clock is what placed it, and a declaration only ever
// fills a gap the clock left.
import { toolFamily } from "@armada/components";
import type { PlanTask, Turn, WorkPlan } from "@armada/protocol";

import { instant, lasting } from "./duration";
import { editOf, type LogRow } from "./story";
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
 * The task whose `scope` names `path`, by id — what places an edit no window
 * covers. Nothing here reads the clock, so a window always answers first.
 *
 * **Matched at a path segment, not against the diff.** A call names the file
 * under a worktree and a declaration is repository-relative, and `narrationOf`
 * is never handed the diff to reconcile the two — `repoPathOf`'s rule, applied
 * without it. A declared directory holds the files under it, because
 * `declare_scope` takes both.
 *
 * **The more specific declaration wins, and a tie goes to plan order.** An
 * exact name beats a directory holding it, and a deeper directory beats a
 * shallower one.
 */
export function declaredBy(path: string, tasks: readonly PlanTask[]): string | undefined {
  let best: { id: string; exact: boolean; length: number } | undefined;
  for (const task of tasks) {
    for (const declared of task.scope ?? []) {
      const named = declared.endsWith("/") ? declared.slice(0, -1) : declared;
      if (named === "") continue;
      const exact = path === named || path.endsWith(`/${named}`);
      const under = path.startsWith(`${named}/`) || path.includes(`/${named}/`);
      if (!exact && !under) continue;
      const better =
        best === undefined ||
        (exact && !best.exact) ||
        (exact === best.exact && named.length > best.length);
      if (better) best = { id: task.id, exact, length: named.length };
    }
  }
  return best?.id;
}

/**
 * The task a changing call belongs to by its declared path, or nothing.
 *
 * **Only an Edit or a Write moves** — `toolFamily`'s `changing` roster, the
 * gate `editsIn` reads by. A `Read` or a `Bash` call names no task it was for
 * and stays where the clock put it.
 */
function declaredFor(turn: Turn, tasks: readonly PlanTask[]): string | undefined {
  const saw = turn.saw;
  if (saw.event !== "called" || toolFamily(saw.tool) !== "changing" || saw.detail === "") {
    return undefined;
  }
  return declaredBy(editOf(saw.detail, saw.truncated).path, tasks);
}

/** A row's task, and whether a window is what placed it there. */
type Placing = { id?: string; declared: boolean };

/** Whether a row ends the run under the sentence before it. */
function breaks(row: LogRow): boolean {
  return row.kind === "said" && row.actor !== "armada";
}

/**
 * The task each row belongs to, by row index.
 *
 * **Resolved ahead of the walk because a sentence looks forward.** A sentence
 * goes with a run of edits a declaration placed, and which task that is cannot
 * be known until the run has been read — so placement stopped being something
 * the walk could decide row by row.
 */
function placingOf(
  rows: readonly LogRow[],
  turnOf: Map<string, Turn>,
  tasks: readonly PlanTask[],
): (string | undefined)[] {
  /** What the clock, and then the declarations, say about one turn alone. */
  const placingAt = (turn: Turn): Placing => {
    const window = taskAt(turn.ts, tasks);
    if (window !== undefined) return { id: window, declared: false };
    const declared = declaredFor(turn, tasks);
    return declared === undefined ? { declared: false } : { id: declared, declared: true };
  };

  // **A call and its answer are one thing that happened** — `story.ts`'s own
  // rule for folding the two into one row, and the rule for placing the
  // failure it does not fold. The clock used to place them together because
  // their instants are a fraction apart; a declaration moves only the call, so
  // the answer has to be told where the call went or the row saying what came
  // back draws under a heading for work that did not happen there.
  const own: (Placing | undefined)[] = [];
  const byCall = new Map<string, Placing>();
  for (const row of rows) {
    const turn = turnOf.get(row.id);
    if (turn === undefined) {
      own.push(undefined);
      continue;
    }
    const saw = turn.saw;
    // A refused call answers the same way, and carries the same `call`.
    const answering =
      saw.event === "answered" || saw.event === "refused" ? byCall.get(saw.call) : undefined;
    const placing = answering ?? placingAt(turn);
    if (saw.event === "called") byCall.set(saw.call, placing);
    own.push(placing);
  }

  const placed = own.map((one) => one?.id);
  for (const [index, row] of rows.entries()) {
    // **A window still holds a sentence where it holds anything.** A sentence
    // said before the Drone marked a task working keeps drawing outside even
    // where every call under it then landed in that task: that is what the
    // clock says, and the declaration is only filling a gap the clock left.
    const mine = own[index];
    if (!breaks(row) || mine === undefined || mine.id !== undefined) continue;
    let together: string | undefined;
    let whole = true;
    for (let after = index + 1; after < rows.length && whole; after += 1) {
      if (breaks(rows[after] as LogRow)) break;
      // A row carrying no turn casts no vote: it is one the walk leaves where
      // the row before it went, so it cannot disagree about anything.
      const one = own[after];
      if (one === undefined) continue;
      whole = one.declared && (together === undefined || one.id === together);
      together = one.id;
    }
    if (whole && together !== undefined) placed[index] = together;
  }
  return placed;
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
  const placing = placingOf(rows, turnOf, tasks);
  let placed: TaskWork = outside;
  let beat: Beat | undefined;
  const spans = new Map<TaskWork, { from?: string; to?: string }>();
  for (const [index, row] of rows.entries()) {
    const drawn = index >= from;
    const turn = turnOf.get(row.id);
    if (turn !== undefined) {
      const id = placing[index];
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
