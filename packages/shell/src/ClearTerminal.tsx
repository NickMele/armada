// The bulk clear, and the dialog it owns.
//
// Split out of `Jobs.tsx` at the gate's 500-line warning, and split to here
// rather than anywhere else because it is a control that owns a dialog — the
// pattern `Redirect.tsx`, `Overrule.tsx` and `Report.tsx` already follow.
//

import { Button, Dialog } from "@armada/components";
import { useState } from "react";

/**
 * The bulk clear. **Its own dialog is the confirmation**, the pattern
 * `Acts.tsx`'s redirect and override controls use — there is nothing to
 * confirm a second time after it, only to send. Unlike a kill this leaves no
 * record afterward, which is why it confirms at all where the row-level acts
 * mostly do too but for a milder reason.
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
          {`Every job that is done, failed, killed, rejected or superseded — ${count} right now — is `}
          removed from the board along with its whole record. There is no undo, and a cleared job
          cannot be opened again.
        </p>
        <p>Its worktree and branch are left as its drone left them — armada clean owns those, on its own schedule.</p>
      </Dialog>
    </>
  );
}
