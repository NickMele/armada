// The two bulk acts on a finished job, and the dialogs they own.
//
// Split out of `Jobs.tsx` at the gate's 500-line warning, and split to here
// rather than anywhere else because it is a control that owns a dialog — the
// pattern `Redirect.tsx`, `Overrule.tsx` and `Report.tsx` already follow.
//
// **Two controls, not one with a mode.** `crates/ipc/operations.toml` argues
// the split on `forget_job`'s own row — one call with two unrelated things to
// fail at is worse than two calls, and a person clearing a board should not
// also have to think about a directory — and `#570` is what put the two
// controls beside each other here rather than folding the second into the
// first. Each keeps its own dialog, because what each act costs is different.

import { Button, Dialog } from "@armada/components";
import { useState } from "react";

/**
 * The bulk reclaim — `Clear`. **Its own dialog is the confirmation**, the
 * pattern `Acts.tsx`'s redirect and override controls use — there is nothing
 * to confirm a second time after it, only to send.
 *
 * **Every row survives.** This takes the disk and the branch a base already
 * reaches; `ForgetTerminalControl` beside it is the act that takes the
 * record, and confirms separately because it is the one that cannot be
 * undone.
 *
 * Not rendered by the caller unless `count > 0`, so this never has to draw its
 * own empty state.
 */
export function ClearTerminalControl({
  count,
  stale,
  onConfirm,
}: {
  count: number;
  stale: boolean;
  onConfirm: () => void;
}) {
  const [open, setOpen] = useState(false);
  const noun = count === 1 ? "job" : "jobs";

  return (
    <>
      <Button variant="secondary" size="sm" disabled={stale} onClick={() => setOpen(true)}>
        {`Clear ${count} finished ${noun}`}
      </Button>
      <Dialog
        open={open}
        tone="destructive"
        title={`Clear ${count} finished ${noun}?`}
        confirmLabel="Clear"
        onCancel={() => setOpen(false)}
        onConfirm={() => {
          setOpen(false);
          onConfirm();
        }}
      >
        <p>
          {`Every job that is done, failed, killed, rejected or superseded — ${count} right now — has `}
          its worktree and branch given back. The job and everything it recorded — its log, its
          checks, its judgments — stay on the board, under Cleared.
        </p>
        <p>
          A branch holding commits the base cannot reach is left standing rather than deleted —
          you will be told which ones.
        </p>
      </Dialog>
    </>
  );
}

/**
 * The bulk forget — `Delete record`. **The one bulk act on this board that
 * cannot be undone**, and it confirms on its own rather than sharing
 * `ClearTerminalControl`'s dialog: a person reaching for `Clear` must never
 * end up here by the same press.
 *
 * **Every row named goes, reclaimed or not.** A job `Clear` already reclaimed
 * is still a legal target — deleting the record it kept is the whole of what
 * this act is for.
 *
 * Not rendered by the caller unless `count > 0`, so this never has to draw its
 * own empty state.
 */
export function ForgetTerminalControl({
  count,
  stale,
  onConfirm,
}: {
  count: number;
  stale: boolean;
  onConfirm: () => void;
}) {
  const [open, setOpen] = useState(false);
  const possessive = count === 1 ? "job's" : "jobs'";

  return (
    <>
      <Button variant="secondary" size="sm" disabled={stale} onClick={() => setOpen(true)}>
        {`Delete ${count} ${possessive} records`}
      </Button>
      <Dialog
        open={open}
        tone="destructive"
        title={`Delete ${count} finished ${possessive} records?`}
        confirmLabel="Delete"
        onCancel={() => setOpen(false)}
        onConfirm={() => {
          setOpen(false);
          onConfirm();
        }}
      >
        <p>
          {`Every job that is done, failed, killed, rejected or superseded — ${count} right now — is `}
          removed from the board along with its whole record. There is no undo, and a deleted job
          cannot be opened again.
        </p>
        <p>Its worktree and branch are left as its drone left them, or as a reclaim already left them.</p>
      </Dialog>
    </>
  );
}
