// The changed-files panel under a checkout run's result line, and the run's
// diff behind *Open the diff*. Pure, so the rules are unit tests rather than
// plays. Journey 9, *Running one*.

import type { RunDiffReading, RunPageProps } from "@armada/components";
import type { CheckoutRunDiffRead, CheckoutRunRecord } from "@armada/protocol";
import { said } from "./copy";
import { clockOf } from "./duration";
import { drawnOf } from "./review";

/**
 * Whether Undo is offered for a run. **Only the newest**: Undo belongs to the
 * run a person just did, and an older one's snapshot predates the runs after it
 * — Fleet's refusal is the backstop, not how a person learns that. `undoable`
 * is Fleet's answer, and a run already undone has nothing left to put back.
 */
export function checkoutUndoOffered(run: CheckoutRunRecord, newest: boolean): boolean {
  return newest && run.undoable && run.undone_at === undefined;
}

/**
 * The panel for one run — **the result line's run**. Files with *Open the
 * diff* and Undo where it changed something; a line and neither where it did
 * not. An earlier run's changes are reached from *Earlier runs*, without Undo.
 */
export function checkoutChangedOf(
  run: CheckoutRunRecord,
  /** Whether it is the newest run in the checkout — the only one Undo is offered on. */
  newest: boolean,
  acts: { onOpenDiff: (runId: string) => void; onUndo: (runId: string) => void },
): NonNullable<RunPageProps["changed"]> {
  // `changed` means nothing here, so "changed nothing" would be a false claim.
  if (run.changed_unreadable !== undefined) {
    return { files: [], unreadable: `What this run changed could not be read: ${run.changed_unreadable}.` };
  }
  if (run.changed.length === 0) return { files: [] };
  return {
    files: run.changed,
    // An undone run still has its snapshot, so its diff still reads.
    onOpenDiff: () => acts.onOpenDiff(run.id),
    ...(checkoutUndoOffered(run, newest) ? { onUndo: () => acts.onUndo(run.id) } : {}),
    ...(run.undone_at === undefined
      ? {}
      : { undone: `Undone at ${clockOf(run.undone_at)}. These files are back as they were before the run.` }),
  };
}

/** What a reading with no files in it says. Ordinary, and never an error. */
export const RUN_CHANGED_NOTHING = "Against the snapshot it took, this run changed nothing.";

/** Files changed and git's patch holds no line of them — a binary file or a mode change. */
export const RUN_CHANGED_NO_LINES =
  "The files this run changed have no lines git can draw — a binary file or a mode change. " +
  "The page behind this lists them.";

/** A cut patch. There is no worktree to send a reader to, so it says why instead. */
function checkoutCutSentence(drawnLines: number, total: number): string {
  return (
    `This is the first ${drawnLines.toLocaleString()} lines of a ${total.toLocaleString()}-line ` +
    "patch. The rest is not on screen — Bridge bounds a patch rather than freezing on one."
  );
}

/**
 * What `RunDiffSheet` draws for one answer. An answer for another run reads as
 * still reading, `DecidedDiff`'s reason; a gone snapshot is drawn as gone.
 */
export function checkoutRunDiffReadingOf(
  runId: string,
  read: "reading" | CheckoutRunDiffRead,
): RunDiffReading {
  if (read === "reading") return { state: "reading" };
  if (!read.ok) return { state: "failed", saying: `Fleet did not answer: ${said(read.outcome)}` };
  if (read.diff.id !== runId) return { state: "reading" };
  const reading = read.diff.reading;
  if (reading.state === "gone") return { state: "gone", why: reading.why };
  const drawn = reading.patch === undefined ? { files: [] } : drawnOf(reading.patch, checkoutCutSentence);
  return {
    state: "read",
    files: drawn.files,
    ...(drawn.cut === undefined ? {} : { cut: drawn.cut }),
    emptyNote: reading.files.length === 0 ? RUN_CHANGED_NOTHING : RUN_CHANGED_NO_LINES,
  };
}
