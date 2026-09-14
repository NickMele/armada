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
 *
 * **One control on two jobs, and one paragraph differs.** Since #145 a redirect
 * reaches a healthy drone as well as one holding at a step that stopped. What
 * it does is the same act either way — a turn into a session that is already
 * open — and what happens after the press is not, so the caller says which job
 * this is rather than the screen offering a second control for it.
 */
export function RedirectControl({
  jobId,
  drone,
  disabled,
  pending = false,
  onRedirect,
}: {
  jobId: string;
  /**
   * Which drone this is. `holding` is one standing at a step that stopped,
   * which is `stuck.recourse`'s reading; `working` is one that is getting on
   * with it, which is `steering.ts`'s. **It changes the wait and nothing
   * else** — not the act, not the label and not the field.
   *
   * **Required, and no default.** A caller that has not said which job it is
   * holding would get a paragraph about escalation over a job that is running,
   * which is the one sentence in this dialog a person could act wrongly on.
   */
  drone: "holding" | "working";
  disabled: boolean;
  /**
   * This is the control that opened the dialog, and `redirect` is out. The
   * dialog has already closed on its own confirm — this is what still shows
   * the wait. #1117.
   */
  pending?: boolean;
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
      <Button variant="secondary" pending={pending} disabled={disabled} onClick={() => setOpen(true)}>
        {pending ? "Redirecting…" : ACT_LABEL.redirect}
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
        {/* One column, at the dialog's own top-level rhythm. Body and field
            are one `children` here rather than the `field` slot Overrule and
            the raise dialogs use, so nothing above pinned them apart —
            without this the explanation ran straight into the field under
            it, unseparated, because a `<p>` carries no margin under this
            app's reset. */}
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
          {/* Said before the press, because the wait is the surprising half: a
              job that does not move on the send would otherwise read as a
              redirect that never arrived. Which half is true is the one fact
              that changes the decision to send, so it is what survives here —
              everything else that used to be said is machinery a person does
              not need to decide. */}
          {drone === "holding" ? (
            <p>
              A stopped drone starts again right away. An escalated one waits until it takes your
              instruction up.
            </p>
          ) : (
            <p>
              The working drone gets your instruction now. Nothing restarts, and its limits keep
              counting.
            </p>
          )}
          {/* No `autoFocus`: the dialog's own contract puts initial focus on
              Cancel, and a second claim on it here would only lose to it. */}
          <Textarea
            label="Instruction"
            rows={4}
            value={instruction}
            onChange={(event) => setInstruction(event.target.value)}
          />
        </div>
      </Dialog>
    </>
  );
}
