// Giving one job more turns, and the dialog that collects the figure.
//
// One file per control that owns a dialog, which is `RaiseCap.tsx`'s rule and
// `Redirect.tsx`'s before it.
//
// **Its own file beside the cost cap's rather than a unit switch inside it.**
// The two ceilings hold a job apart: money is spent and turns are taken, only
// one of them is what stopped this job, and a dialog that could raise either
// would ask a person to pick a ceiling before it told them which one caught
// them. The figures differ too — a cap in dollars is converted and a cap in
// turns is not.

import { useState } from "react";
import { Button, Dialog, Input } from "@armada/components";

import type { JobSpend } from "@armada/protocol";
import { RAISE_TURN_CAP_LABEL } from "./copy";

/**
 * How many more turns than the cap in force the field starts at.
 *
 * **A starting point, not a rule**, for `RaiseCap.tsx`'s reason: it is the
 * multiple Fleet bounds Helm to, so a person who takes the offered figure has
 * asked for what an agent could have asked for on its own.
 */
const OFFERED = 2;

/**
 * The button that opens the raise dialog, and the dialog itself.
 *
 * **The dialog is the confirmation**, which is the field-collecting exception
 * the design contract names. `confirmDisabled` keeps the send control off
 * unless the figure is above the cap in force, matching the 422 Fleet would
 * answer rather than round-tripping to learn it.
 *
 * **It states both figures before it asks for one**, and neither is hedged: a
 * turn is counted, and `facts.ts` reserves the tilde for the spend, which is
 * what a run would have cost at list price.
 *
 * **It names the other remedy.** A job going in circles and a job held out of
 * its last cheap step are over the same number, and only one of them wants a
 * new brief.
 */
export function RaiseTurnCapControl({
  jobId,
  spend,
  disabled,
  open,
  onOpen,
  onRaise,
}: {
  jobId: string;
  /**
   * What the job has taken and what it is allowed, from `GET /jobs/:job_id`.
   *
   * **Required, and no default**, for `RaiseCapControl`'s reason: a dialog that
   * could not name the cap in force would be asking for a number against
   * nothing.
   */
  spend: JobSpend;
  disabled: boolean;
  /**
   * Whether the dialog is up. **Held by the screen and not here**: `T` opens it
   * too, and the keyboard is bound one level up — see `detail-keys.ts`.
   */
  open: boolean;
  onOpen: (up: boolean) => void;
  /** The new ceiling, as a turn count — the unit `spend` reads it in. */
  onRaise: (jobId: string, turnCap: number) => void;
}) {
  // Held as the string rather than a number, for the reason `RaiseCap.tsx`
  // holds its dollars as one: parsing on every keystroke rewrites what somebody
  // is halfway through typing.
  const [typed, setTyped] = useState("");

  const suggested = String(spend.turn_cap * OFFERED);
  // **The offered figure stands in for an untouched field**, because `T` opens
  // this without going through the button and there is no press to seed on.
  const field = typed === "" ? suggested : typed;
  const asked = Number(field);
  // Whole turns only. A drone takes a turn or it does not, and a fractional cap
  // is a figure Fleet would round somewhere this screen could not show.
  const raises = Number.isInteger(asked) && asked > spend.turn_cap;

  function close() {
    onOpen(false);
    setTyped("");
  }

  return (
    <>
      <Button variant="secondary" disabled={disabled} onClick={() => onOpen(true)}>
        {RAISE_TURN_CAP_LABEL}
      </Button>
      <Dialog
        open={open}
        tone="neutral"
        title="Raise this job's turn cap?"
        confirmLabel={RAISE_TURN_CAP_LABEL}
        confirmDisabled={!raises}
        // Pinned outside the region that scrolls: the figure is what the
        // confirm control waits on, so it stays reachable at any window height.
        field={
          // No `autoFocus`: the dialog's own contract puts initial focus on
          // Cancel, and a second claim on it here would only lose to it.
          <Input
            label="New cap, in turns"
            mono
            inputMode="numeric"
            value={field}
            invalid={!raises}
            message={`A new cap has to be a whole number above ${spend.turn_cap}, which is what this job is held to now.`}
            onChange={(event) => setTyped(event.target.value)}
          />
        }
        onCancel={close}
        onConfirm={() => {
          const sent = asked;
          close();
          onRaise(jobId, sent);
        }}
      >
        {/* The two figures the new one is decided against, said here rather
            than left on the header behind the dialog. Neither is hedged: a turn
            is counted, unlike the spend beside it on the same header. */}
        <p>
          {`This job has taken ${spend.turns} of ${spend.turn_cap} turns, across ` +
            `${spend.drones} ${spend.drones === 1 ? "drone" : "drones"}. Raising the cap lets the next drone ` +
            "start; nothing already done is thrown away and the job stays exactly where it is."}
        </p>
        {/* Both remedies, because the number cannot tell them apart. A job that
            finished its work and stopped short of a cheap last step reads
            exactly like a job going round and round, and a redispatch on the
            first throws away work every gate has already passed. */}
        <p>
          A job over its turn cap has either run out of room to finish or gone in circles. Raise the
          cap to let it finish. Redispatch it instead if the turns were going nowhere, and it starts
          again on a new brief.
        </p>
      </Dialog>
    </>
  );
}
