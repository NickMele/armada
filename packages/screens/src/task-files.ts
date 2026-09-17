// Which files each plan task changed, and which changed files no task's edits
// account for. #1187.
//
// A task's files are its Edit and Write calls, placed by the working windows
// (`taskAt`), never matched to a task by words. **Their numbers are edit
// sizes**: the line counts `edited` in `crates/adapters/src/transcript.rs`
// writes, not diff lines, so they are labelled as such. A file Bash wrote names
// no task and goes in the row no task owns.
import { toolFamily, type ChangedFile } from "@armada/components";
import type { PlanTask, Turn } from "@armada/protocol";

import { taskAt } from "./narration";
import { editOf } from "./story";

/** One call that changed a file. */
export type Edit = {
  /** The row's id, `String(turn.seq)`. */
  id: string;
  /** As the row carries it: under `~` where it was under a home directory. */
  path: string;
  added?: number;
  deleted?: number;
  /** The task that was being worked when it was made, by id. */
  task?: string;
};

/** Every Edit and Write that went through, with its task. A failed or refused call changed nothing. */
export function editsIn(turns: readonly Turn[], tasks: readonly PlanTask[]): Edit[] {
  const failed = new Set<string>();
  for (const turn of turns) {
    if (turn.saw.event === "answered" && turn.saw.failed) failed.add(turn.saw.call);
    if (turn.saw.event === "refused") failed.add(turn.saw.call);
  }
  const edits: Edit[] = [];
  for (const turn of turns) {
    const saw = turn.saw;
    // The tools whose calls name a file they changed — `toolFamily`'s own
    // roster, so the set the log hues by and the set counted here cannot
    // disagree. Bash is not one: nothing ties a Bash call to a file.
    if (saw.event !== "called" || toolFamily(saw.tool) !== "changing" || failed.has(saw.call)) {
      continue;
    }
    if (saw.detail === "") continue;
    const task = taskAt(turn.ts, tasks);
    edits.push({
      id: String(turn.seq),
      ...editOf(saw.detail, saw.truncated),
      ...(task === undefined ? {} : { task }),
    });
  }
  return edits;
}

/**
 * The diff path a call's path names: equal, or the call's path ends in `/` and
 * it. A call names the file under a worktree Bridge is not told, the diff from
 * the repository's top. The longest match wins.
 */
export function repoPathOf(path: string, repo: readonly string[]): string | undefined {
  let best: string | undefined;
  for (const candidate of repo) {
    if (path !== candidate && !path.endsWith(`/${candidate}`)) continue;
    if (best === undefined || candidate.length > best.length) best = candidate;
  }
  return best;
}

/** One file a task changed, with its edits' sizes added up. */
export type TaskFile = {
  /** The repository path where the diff names it, the call's path otherwise. */
  path: string;
  inDiff: boolean;
  added?: number;
  deleted?: number;
};

/**
 * Each task's files, in the order it first touched them, by task id. **Sizes
 * are added only where every edit to the file carried one**: a total over some
 * edits would read as the whole.
 */
export function filesByTask(
  edits: readonly Edit[],
  diff: readonly ChangedFile[],
): Map<string, TaskFile[]> {
  const repo = diff.map((file) => file.path);
  const byTask = new Map<string, Map<string, TaskFile & { unsized: boolean }>>();
  for (const edit of edits) {
    if (edit.task === undefined) continue;
    const inRepo = repoPathOf(edit.path, repo);
    const path = inRepo ?? edit.path;
    const files = byTask.get(edit.task) ?? new Map<string, TaskFile & { unsized: boolean }>();
    byTask.set(edit.task, files);
    const was = files.get(path) ?? { path, inDiff: inRepo !== undefined, unsized: false };
    const unsized = was.unsized || edit.added === undefined;
    files.set(path, {
      path,
      inDiff: was.inDiff,
      unsized,
      ...(unsized ? {} : { added: (was.added ?? 0) + (edit.added ?? 0) }),
      ...(unsized ? {} : { deleted: (was.deleted ?? 0) + (edit.deleted ?? 0) }),
    });
  }
  return new Map(
    [...byTask.entries()].map(([task, files]) => [
      task,
      [...files.values()].map(({ unsized: _unsized, ...file }) => file),
    ]),
  );
}

/**
 * The diff's files no task's edit names. Read against the whole Job's edits, so
 * a file a task wrote on an earlier attempt is not taken for one nobody owns.
 */
export function unownedOf(edits: readonly Edit[], diff: readonly ChangedFile[]): ChangedFile[] {
  const repo = diff.map((file) => file.path);
  const owned = new Set<string>();
  for (const edit of edits) {
    if (edit.task === undefined) continue;
    const path = repoPathOf(edit.path, repo);
    if (path !== undefined) owned.add(path);
  }
  return diff.filter((file) => !owned.has(file.path));
}

/** What a task's file numbers are. */
export const EDIT_SIZES = "sizes of its edits, not the diff";

/** What the numbers in the row no task owns are. */
export const DIFF_LINES = "lines in the diff";

/** The heading over the files no task's edits account for. */
export const OUTSIDE_TASK_EDITS = "Changed outside any task's edits";
