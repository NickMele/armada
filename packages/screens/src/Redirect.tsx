// Redirecting the drone that is still there, and the dialog that collects the
// instruction.
//
// Split out of `Acts.tsx` when that file grew past the gate's 500-line warning,
// the same way `Acts.tsx` itself came out of `JobDetail.tsx`. **One file per
// control that owns a dialog**, which is what `Report.tsx` already was: the set
// of acts and how they are arranged is one subject, and what a single act asks
// a person for before it sends is another.

import { useState } from "react";
import { Button, Dialog, Textarea } from "@armada/components";

import { ACT_LABEL } from "./copy";

/**
 * The button that opens the redirect dialog, and the dialog itself.
 *
 * **The dialog is the confirmation.** There is no second "are you sure" after
 * it, because sending is the one thing the dialog's own button does — closing
 * it any other way sends nothing. `confirmDisabled` keeps the send control off
 * while the field is blank, matching the 422 Fleet would give it, rather than
 * letting the press round-trip to Fleet to learn that.
 */
export function RedirectControl({
  jobId,
  disabled,
  onRedirect,
}: {
  jobId: string;
  disabled: boolean;
  onRedirect: (jobId: string, instruction: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [instruction, setInstruction] = useState("");

  function close() {
    setOpen(false);
    setInstruction("");
  }

  return (
    <>
      <Button variant="secondary" disabled={disabled} onClick={() => setOpen(true)}>
        {ACT_LABEL.redirect}
      </Button>
      <Dialog
        open={open}
        tone="neutral"
        title="Redirect the drone on this job?"
        confirmLabel={ACT_LABEL.redirect}
        confirmDisabled={instruction.trim() === ""}
        onCancel={close}
        onConfirm={() => {
          const sent = instruction;
          close();
          onRedirect(jobId, sent);
        }}
      >
        <p>
          The instruction is sent to the drone as a new turn. The job stays at the same step, with
          the same session — nothing is spawned and nothing already done is thrown away.
        </p>
        {/* Said before the press, because the wait is the surprising half: a job
            that escalated without stopping a step does not move on the send, and
            a screen that changed nothing would read as a redirect that never
            arrived. What it did is on the job afterwards — `recovery.ts`. */}
        <p>
          Where a step stopped, the job runs again straight away. Where it escalated without
          stopping one, it stays escalated until the drone takes a turn — sending is not evidence
          that it read anything.
        </p>
        {/* No `autoFocus`: the dialog's own contract puts initial focus on
            Cancel, and a second claim on it here would only lose to it. */}
        <Textarea
          label="Instruction"
          rows={4}
          value={instruction}
          onChange={(event) => setInstruction(event.target.value)}
        />
      </Dialog>
    </>
  );
}
