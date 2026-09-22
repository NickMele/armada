// One task, read whole for the inspector beside the run. `#1536`.
//
// **Its own agent is the subject.** A step's reading is about a step and a
// group's is about a boundary; this is about the Drone on one task — what it is
// doing now, what it was told, what it may touch, what it last wrote, what it
// has said, and the two things a person may do about it.
//
// **Cost only once that agent stopped** (`#1530`, 22 Sep). Turns are live and a
// figure for spend is not, so a running task shows turns and nothing else.

import type { JobDetail as JobWhole, Turn } from "@armada/protocol";

import type { GroupView } from "./draft/group";
import type { TaskView } from "./draft/task";
import { editsIn } from "./task-files";
import { taskAt } from "./narration";
import { entriesOf } from "./story";
import { spentSaid, tasksOf } from "./tab-plan-read";
import type { WorkflowReading } from "./workflow-inspector";

/** How many of a task's own lines the panel draws. The rest is the log sheet's. */
const MOST_LINES = 6;

/** Why a task has nothing to read here. Each names what is missing, never a blank. */
export const NO_TURNS_WATCHED =
  "Nothing is watching this Job's turns, so this task's own lines are not being read.";
export const NO_EDIT_READ = "Nothing this task wrote has been read on this step yet.";
export const NO_BRIEF = "The planner recorded no words of its own for this task.";

/**
 * What the task is doing now, as a sentence. **Read off the record** — turns
 * while it runs, the cost once its own agent stopped, and the reason where it
 * failed.
 */
export function doingOfTask(task: TaskView): string {
  const spent = spentSaid(task);
  switch (task.state) {
    case "working":
      return spent === undefined
        ? "Its agent is working. Nothing it has spent can be read until that agent stops."
        : `Its agent is working — ${spent} so far. What it cost reads once that agent stops.`;
    case "done":
      return spent === undefined
        ? "Its agent has stopped and the work is in."
        : `Its agent stopped after ${spent}.`;
    case "failed":
      return task.failed_reason ?? "Its agent stopped without finishing.";
    case "dropped":
      return task.reason === undefined ? "This task was dropped." : `Dropped — ${task.reason}`;
    default:
      return "Nothing has been dispatched at this task yet.";
  }
}

/**
 * What its Drone was told. The planner's `note` and what it expects, joined —
 * **the words as written**, never a paraphrase a person cannot check.
 */
export function briefOfTask(task: TaskView): string | undefined {
  const lines = [task.note, task.expects === undefined ? undefined : `Proves it: ${task.expects}`];
  const said = lines.filter((one): one is string => one !== undefined);
  return said.length === 0 ? undefined : said.join(" ");
}

/** The last file this task wrote, with the size of that edit. */
export function lastEditOf(
  whole: JobWhole | null,
  turns: readonly Turn[],
  taskId: string,
): { path: string; says?: string } | undefined {
  const edits = editsIn(turns, whole?.work_plan?.tasks ?? []).filter((one) => one.task === taskId);
  const last = edits[edits.length - 1];
  if (last === undefined) return undefined;
  const sizes = [
    last.added === undefined || last.added === 0 ? undefined : `+${last.added}`,
    last.deleted === undefined || last.deleted === 0 ? undefined : `−${last.deleted}`,
  ].filter((one): one is string => one !== undefined);
  return { path: last.path, ...(sizes.length === 0 ? {} : { says: sizes.join(" ") }) };
}

/**
 * This task's own lines, newest last, bounded — the log sheet holds the rest.
 *
 * **Placed by the turn's raw instant, never by the row's.** `LogRow.at` is
 * already a clock, and `taskAt` reads an instant.
 */
export function linesOfTask(
  whole: JobWhole | null,
  turns: readonly Turn[],
  stepId: string | undefined,
  taskId: string,
): { id: string; at?: string; said: string }[] {
  const tasks = whole?.work_plan?.tasks ?? [];
  const instantOf = new Map(turns.map((turn) => [String(turn.seq), turn.ts]));
  return entriesOf(turns, stepId)
    .filter((row) => {
      const ts = instantOf.get(row.id);
      return ts !== undefined && taskAt(ts, tasks) === taskId;
    })
    .slice(-MOST_LINES)
    .map((row) => ({ id: row.id, at: row.at, said: row.message }));
}

export type TaskInspectorReading = {
  whole: JobWhole | null;
  groups: readonly GroupView[];
  taskId: string;
  /** The Job's turns, where something is watching them. Empty is not "nothing happened". */
  turns: readonly Turn[];
  /** Whether anything is watching them, so an empty list can say which it is. */
  watching: boolean;
  /** The step the groups hang under, for narrowing the turns to it. */
  stepId?: string;
};

/**
 * One task, read for the inspector. `undefined` where the plan holds no task by
 * that id — a Job switched under a held selection.
 */
export function taskReadingOf({
  whole,
  groups,
  taskId,
  turns,
  watching,
  stepId,
}: TaskInspectorReading): WorkflowReading | undefined {
  const task = tasksOf(groups).find((one) => one.id === taskId);
  if (task === undefined) return undefined;
  const brief = briefOfTask(task);
  const edit = lastEditOf(whole, turns, taskId);
  const lines = linesOfTask(whole, turns, stepId, taskId);
  const drone = task.drone_id ?? whole?.job.assigned_drone;
  return {
    name: `${task.id} · ${task.title}`,
    kind: "task",
    doing: doingOfTask(task),
    ...(brief === undefined ? { briefAbsent: NO_BRIEF } : { brief }),
    scope: task.scope,
    beside: task.concurrent_with,
    ...(edit === undefined ? { lastEditAbsent: NO_EDIT_READ } : { lastEdit: edit }),
    log: lines,
    ...(lines.length === 0 && !watching ? { logAbsent: NO_TURNS_WATCHED } : {}),
    drones: drone === undefined ? [] : [{ id: drone, label: `Drone on ${task.id}` }],
  };
}
