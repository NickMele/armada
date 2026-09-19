// The confirmation every destructive act passes through — #1528. Lifted out of
// `App.tsx` whole: the words, the one act that collects a note, and nothing
// else. `App.tsx` keeps which act is being confirmed, because that is window
// state and this is the dialog for it.

import { Dialog, Textarea } from "@armada/components";
import { ACT_LABEL, CONFIRM, RESTART_NOTE, type ConfirmableAct } from "@armada/screens";

export type Confirming = { act: ConfirmableAct; jobId: string };

export type ConfirmActProps = {
  /** Nothing to confirm draws nothing. */
  confirming: Confirming | null;
  /** The restart note in progress, held by the caller so cancelling clears it. */
  restartNote: string;
  onRestartNote: (said: string) => void;
  onCancel: () => void;
  onConfirm: (act: ConfirmableAct, jobId: string) => void;
};

/**
 * **It states what happens and what survives rather than asking "are you
 * sure".** Cancel holds initial focus; the dialog owns that rule and this only
 * supplies the words.
 */
export function ConfirmAct({
  confirming,
  restartNote,
  onRestartNote,
  onCancel,
  onConfirm,
}: ConfirmActProps) {
  if (confirming === null) return null;
  return (
    <Dialog
      open
      tone={CONFIRM[confirming.act].tone ?? "destructive"}
      title={CONFIRM[confirming.act].title}
      confirmLabel={ACT_LABEL[confirming.act]}
      onCancel={onCancel}
      onConfirm={() => onConfirm(confirming.act, confirming.jobId)}
    >
      {CONFIRM[confirming.act].body}
      {/* The one confirmation that collects anything, and what it collects is
          optional — the button is never disabled on it, because leaving the
          field alone is the restart this dialog has always been. No
          `autoFocus`: the dialog puts initial focus on Cancel, and a second
          claim on it here would only lose to it. */}
      {confirming.act !== "restart_step" ? null : (
        <>
          <p>{RESTART_NOTE.says}</p>
          <Textarea
            label={RESTART_NOTE.label}
            rows={4}
            value={restartNote}
            onChange={(event) => onRestartNote(event.target.value)}
          />
        </>
      )}
    </Dialog>
  );
}
