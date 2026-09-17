// The activity log, with runs of one tool folded to a line.
//
// **The grouping is derived from the turns and drawn through the log.** What a
// row is, how it opens and which keyboard reaches it stay `Log.tsx`'s — this
// only decides which rows belong together and what the folded line says. So a
// group's body is the same component the log draws unfolded, and the region and
// payload names `detail-keys.ts` reads are untouched.
//
// **The ids line up because both sides use the socket's sequence.** A
// `WorkingAct` is named `String(turn.seq)` and so is a `LogRow`, which is what
// lets a run select its own rows without a second pass over the wire.
import {
  NarrationPlanBar,
  ToolName,
  WorkGroups,
  WorkNarration,
  type ChangedFile,
  type NarrationFile,
  type NarrationSection,
  type TaskMarkState,
  type WorkGroup,
} from "@armada/components";
import type { Turn, WorkPlan } from "@armada/protocol";

import type { Calls } from "./calls";
import type { DetailKeys } from "./detail-keys";
import { briefly, clock } from "./duration";
import { Log } from "./Log";
import { foldersOf, nameUnder } from "./change-summary";
import { callsSaid, narrationOf, planBarOf, workSaid } from "./narration";
import {
  DIFF_LINES,
  EDIT_SIZES,
  OUTSIDE_TASK_EDITS,
  editsIn,
  filesByTask,
  unownedOf,
  type TaskFile,
} from "./task-files";
import type { LogRow } from "./story";
import { runsOf, workingOf, type WorkingRun } from "./working";

export function WorkGrouped({
  rows,
  turns,
  stepId,
  unread,
  emptyNote,
  calls: fetched,
  log,
  most,
}: {
  /** The step's readable rows, in order — what the log would draw flat. */
  rows: LogRow[];
  /** The same turns those rows came from, which carry the tool and the timing. */
  turns: readonly Turn[];
  stepId: string;
  /** How many rows this Bridge has no drawing for, by the wire's own kind. */
  unread?: { kind: string; count: number }[];
  emptyNote: string;
  calls: Calls;
  /** The log's own keyboard binding, by region. Threaded to every group. */
  log: ReturnType<DetailKeys["inLog"]>;
  /**
   * How many groups to draw, newest last. Absent draws them all.
   *
   * **Counted in groups and not in rows**, which is the point of the folding: a
   * preview bounded at five rows showed five `Read` calls, and the same height
   * bounded at five groups shows five different things the Drone did.
   */
  most?: number;
}) {
  // **Which run each row belongs to**, the call's row and its answer's alike.
  // The derivation folds an answer into its call to count one thing that
  // happened; a surface drawing the transcript still has to draw both.
  const runOf = new Map<string, WorkingRun>();
  const calls = new Set<string>();
  for (const run of runsOf(workingOf(turns, stepId).acts)) {
    for (const act of run.acts) {
      runOf.set(act.id, run);
      if (act.kind === "call") calls.add(act.id);
      if (act.answeredId !== undefined) runOf.set(act.answeredId, run);
    }
  }

  // **Walked in row order, and the run is what breaks it.** Selecting rows by
  // the groups would silently drop any row no group claimed — an `unreadable`
  // line today, and whatever the wire adds next. Walking the rows cannot: a
  // row belonging to no run becomes a chunk that draws bare, and every row is
  // drawn exactly once, in the order it arrived.
  const chunks: { run?: WorkingRun; rows: LogRow[] }[] = [];
  for (const row of rows) {
    const run = runOf.get(row.id);
    const last = chunks[chunks.length - 1];
    if (last !== undefined && last.run === run) last.rows.push(row);
    else chunks.push({ ...(run === undefined ? {} : { run }), rows: [row] });
  }

  const groups: WorkGroup[] = chunks.map((chunk) => {
    // **Counted in calls, and only the ones in hand.** The sheet filters its
    // rows by actor, so a run of nine can arrive with two of its calls drawn —
    // and a heading counting the run rather than what is under it would say
    // nine above two rows.
    const held = chunk.rows.filter((row) => calls.has(row.id)).length;
    const first = chunk.rows[0];
    return {
      id: first === undefined ? "" : first.id,
      // The tool's own colour, the same one its rows carry underneath: a
      // folded run is the rows it hides, and the two reading differently is
      // the fold changing what a person sees. #1196.
      name: chunk.run?.tool === undefined ? "" : <ToolName tool={chunk.run.tool} />,
      mono: true,
      meta: `${held} ${held === 1 ? "call" : "calls"} · ${briefly(chunk.run?.ms ?? 0)}`,
      // Folded where the derivation says so, and never over a single call: a
      // heading over one row hides nothing and costs a press.
      folded: chunk.run?.folded === true && held > 1,
      body: <Log rows={chunk.rows} emptyNote={emptyNote} calls={fetched} {...log} />,
    };
  });

  return (
    <WorkGroups
      groups={most === undefined ? groups : groups.slice(-most)}
      {...(unread === undefined ? {} : { unread })}
      emptyNote={emptyNote}
    />
  );
}

/**
 * The Working area: the Drone's sentences, under the plan task each served.
 * #1185. **The Activity log sheet keeps `WorkGrouped`**, the raw record.
 *
 * The section holding the newest row is open, and in it the newest sentence.
 * A sentence holding a failure is open wherever it is.
 */
export function WorkNarrated({
  rows,
  turns,
  stepId,
  plan,
  live,
  emptyNote,
  calls: fetched,
  log,
  most,
  diff,
  jobTurns,
}: {
  rows: LogRow[];
  turns: readonly Turn[];
  stepId: string;
  /** The Job's plan. Absent draws the sentences with no task headings. */
  plan: WorkPlan | undefined;
  /** The step is being worked now, so the newest sentence reads `so far`. */
  live: boolean;
  emptyNote: string;
  calls: Calls;
  log: ReturnType<DetailKeys["inLog"]>;
  /** How many sentences the open section draws, newest last. */
  most?: number;
  /** The Job's diff, for each task's files and the row no task owns. #1187. */
  diff?: readonly ChangedFile[];
  /**
   * Every turn of the Job, so a task's files include what it wrote on another
   * run. Absent draws no row for the files no task owns.
   */
  jobTurns?: readonly Turn[];
}) {
  // Armada's instruction is the Instructed row above, and the log sheet keeps it.
  const worked = rows.filter((row) => row.kind !== "instructed");
  const narration = narrationOf(worked, turns, stepId, plan);
  const edits = editsIn(jobTurns ?? turns, plan?.tasks ?? []);
  const changed = diff ?? [];
  const byTask = filesByTask(edits, changed);
  const folders = foldersOf(changed.map((file) => file.path));
  const named = (file: TaskFile | ChangedFile): NarrationFile => ({
    path: file.path,
    name:
      folders.get(file.path) === undefined
        ? (file.path.split("/").at(-1) ?? file.path)
        : nameUnder(file.path, folders.get(file.path) as string),
    ...(file.added === undefined ? {} : { added: file.added }),
    ...(file.deleted === undefined ? {} : { deleted: file.deleted }),
  });
  const newestId = narration.sections.find((one) => one.newest)?.beats.at(-1)?.id;
  const sections: NarrationSection[] = narration.sections.map((work) => {
    const kept = most === undefined || !work.newest ? work.beats : work.beats.slice(-most);
    const left = work.beats.length - kept.length;
    const files = work.task === undefined ? [] : (byTask.get(work.task.id) ?? []);
    const meta = [workSaid(work), filesSaid(files.length)].filter((part) => part !== undefined).join(" · ");
    return {
      id: work.task?.id ?? "outside",
      ...(narration.plan === undefined
        ? {}
        : {
            heading:
              work.task === undefined
                ? { title: OUTSIDE_ANY_TASK }
                : { task: work.task.id, mark: markOf(work.task.state), title: work.task.title },
          }),
      ...(meta === "" ? {} : { meta }),
      ...(files.length === 0 ? {} : { files: files.map(named), filesSay: EDIT_SIZES }),
      open: work.newest || work.beats.some((beat) => beat.wrong),
      ...(left === 0 ? {} : { earlier: `${left} earlier, in the log` }),
      beats: kept.map((beat) => {
        const newest = beat.id === newestId;
        return {
          id: beat.id,
          ...(beat.ts === undefined ? {} : { at: clock(beat.ts) }),
          ...(beat.said === undefined ? {} : { said: beat.said }),
          meta: callsSaid(beat, newest && live),
          open: newest || beat.wrong,
          ...(beat.rows.length === 0
            ? {}
            : { body: <Log rows={beat.rows} emptyNote={emptyNote} calls={fetched} {...log} /> }),
        };
      }),
    };
  });
  // Only where there is a plan to own files and a whole Job to read edits in.
  const unowned = narration.plan === undefined || jobTurns === undefined ? [] : unownedOf(edits, changed);
  if (unowned.length > 0) {
    sections.push({
      id: "outside-task-edits",
      heading: { title: OUTSIDE_TASK_EDITS },
      meta: filesSaid(unowned.length),
      beats: [],
      files: unowned.map(named),
      filesSay: DIFF_LINES,
      apart: true,
    });
  }
  return <WorkNarration sections={sections} emptyNote={emptyNote} />;
}

/** `4 files`, or nothing. */
function filesSaid(count: number): string | undefined {
  return count === 0 ? undefined : `${count} ${count === 1 ? "file" : "files"}`;
}

/**
 * `2 of 5`, on the Working header: tasks done over tasks not dropped, the
 * Plan well's own figure and bar. Nothing where the Job has no plan.
 */
export function PlanBar({ plan }: { plan: WorkPlan | undefined }) {
  const bar = planBarOf(plan);
  return bar === undefined ? null : <NarrationPlanBar tasks={bar.states} done={bar.done} />;
}

/** The heading over work done while no task was marked working. */
export const OUTSIDE_ANY_TASK = "Outside any task";

function markOf(state: string): TaskMarkState {
  return state === "working" || state === "done" || state === "dropped" ? state : "open";
}
