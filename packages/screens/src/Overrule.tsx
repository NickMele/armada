// Overruling a machine that stopped the work, and the dialog that collects the
// reason.
//
// Split out of `Acts.tsx` for `Redirect.tsx`'s reason, and beside it rather
// than with it: a redirect asks the drone for something and this tells the
// record a verifier was wrong, and the two dialogs share only their shape.
//
// **Nothing here reaches `gate_undecided`.** That trigger is a gate that ruled
// on nothing, so there is no decision to disagree with — asking it again is
// `Acts.tsx`'s plain control, and `recovery.ts` is where the two are
// partitioned.

import { useState } from "react";
import { Button, Dialog, GamingFlags, Textarea } from "@armada/components";

import { onwards, OVERRULING, type Overrule } from "./recovery";

/**
 * What the rows under it are, said once over them rather than once on each.
 *
 * A panel heading, which is where the Wh- rule lets a Wh- word stand. It names
 * no machine, because the paragraph above it already did — this says only that
 * what follows is the finding rather than more explanation.
 */
const WHAT_IT_FLAGGED = "What it flagged";

/**
 * The button that opens the override dialog, and the dialog itself.
 *
 * **The dialog is the confirmation, and the reason is why there is one.** A
 * person is recording that a verifier was wrong and that they took
 * responsibility for going past it, and `#154` will read those reasons to learn
 * whether the Judge or the criterion was at fault — so the send control stays
 * off while the field is blank, matching the 422 Fleet would answer, and there
 * is no path through this that produces an unexplained override.
 *
 * **Neutral tone, not destructive.** Nothing is destroyed: the work the gate
 * refused is exactly what survives. What the dialog owes instead is the cost —
 * the refusal stays on the record, the reason is written to the log beside it,
 * and the log is append-only.
 *
 * **It says which of the two things is about to happen.** Overruling a middle
 * step advances it and the Job carries on; overruling the last one makes Fleet
 * commit and deliver. `recourseOf` decided which, once, and `onwards` is the
 * sentence the screen behind this dialog already said about it.
 *
 * **And which of the two decisions is being overruled.** A Judge that refused a
 * criterion and a gaming check that called the evidence suspect are different
 * machines saying different things, so every word here comes from `OVERRULING`
 * keyed by the trigger rather than being written once for the refusal and
 * reused. The flag's case carries one thing the refusal's does not: what was
 * flagged, and where — a person taking responsibility for evidence a machine
 * distrusted must not have to leave the dialog to find out what it distrusted.
 * The same weight either way: the dialog and the required reason are what make
 * this a decision taken rather than a button pressed.
 */
export function OverruleControl({
  jobId,
  overrule,
  disabled,
  onOverrule,
}: {
  jobId: string;
  overrule: Overrule;
  disabled: boolean;
  onOverrule: (jobId: string, reason: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  const words = OVERRULING[overrule.trigger];
  // Only a flag has these, and a step can trip more than one pattern. Read
  // rather than counted: what was cited is the whole value of a flag, exactly
  // as a citation is the whole value of a refusal.
  const flagged = overrule.trigger === "evidence_suspect" ? overrule.step.flagged : [];

  function close() {
    setOpen(false);
    setReason("");
  }

  return (
    <>
      <Button variant="secondary" disabled={disabled} onClick={() => setOpen(true)}>
        {words.label}
      </Button>
      <Dialog
        open={open}
        tone="neutral"
        // Wide because this one carries findings rather than two sentences: a
        // citation is a path and an expression, and at the confirmation width
        // it wrapped mid-token.
        width="wide"
        title={words.asks}
        confirmLabel={words.label}
        confirmDisabled={reason.trim() === ""}
        // Pinned, outside the region that scrolls. The reason is what the
        // confirm control is waiting on, so it is reachable at any window
        // height — reading the explanation is optional and reaching the field
        // is not.
        field={
          // No `autoFocus`, for `RedirectControl`'s reason: the dialog's own
          // contract puts initial focus on Cancel.
          <Textarea
            label={words.field}
            rows={3}
            value={reason}
            onChange={(event) => setReason(event.target.value)}
          />
        }
        onCancel={close}
        onConfirm={() => {
          const said = reason;
          close();
          onOverrule(jobId, said);
        }}
      >
        {/* What the act is, said before what it does, and what it is not said
            beside it. The step is named because the person reading this came
            from a rail with several rows on it. */}
        <p>{words.dialog(overrule.step.label)}</p>
        {/* What the check actually found, on the flag's case only.
            **Drawn and not written into a sentence.** `Flagged` is two fields
            on the wire and this assembled them — "It flagged X in Y, Z in W."
            — so two flags arrived as one paragraph and the reader had to parse
            back out the two things that matter. `GamingFlags` draws the same
            rows the rail draws, reading the citations in full because this is
            the surface where a person signs for them.

            A flagged step that carries none is a Fleet that flagged without
            citing, and drawing nothing is the honest render of that. */}
        {flagged.length === 0 ? null : (
          <GamingFlags flags={flagged} said={WHAT_IT_FLAGGED} citation="whole" />
        )}
        {/* What happens next, and what it costs. `onwards` is the same sentence
            the screen behind this one already said, so the two cannot differ
            about whether this job is about to land. */}
        <p>
          {`${onwards(overrule)} Your reason is written to this job's log and stays there — the ` +
            "log is append-only, and nothing takes an override back. It is not sent to the drone, " +
            "which did nothing wrong and is told only that the step was accepted."}
        </p>
      </Dialog>
    </>
  );
}
