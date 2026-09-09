// Giving one job more money, and the dialog that collects the figure.
//
// One file per control that owns a dialog, which is `Redirect.tsx` and
// `Overrule.tsx`'s rule: the set of acts and how they are arranged is one
// subject, and what a single act asks a person for before it sends is another.
//
// **One of the two dialogs here that collect a number rather than a sentence**,
// `RaiseTurnCap.tsx` being the other. The rest take a person's own words; this
// takes a figure, and everything below is about making the figure decidable —
// what the job has spent, what it is held to, and what the new ceiling would be
// — so that nobody has to leave the dialog to work out what to type.

import { useState } from "react";
import { Button, Dialog, Input } from "@armada/components";

import type { JobSpend } from "@armada/protocol";
import { RAISE_CAP_LABEL } from "./copy";

/** Millionths of a dollar in a dollar. The unit the wire and the record use. */
const MICROS = 1_000_000;

/**
 * How much more than the cap in force the field starts at.
 *
 * **A starting point, not a rule.** It is the same multiple Fleet bounds Helm
 * to, deliberately: a person who takes the offered figure has asked for what an
 * agent could have asked for on its own, and anything past it is a person
 * deciding on purpose. Nothing refuses a larger number — a person's raise is
 * unbounded, and this only saves them the arithmetic.
 */
const OFFERED = 2;

/**
 * A cap, in dollars, exactly. **Never hedged**, unlike a spend: `money` in
 * `facts.ts` writes `~$2.40` because what a run cost is notional, and a ceiling
 * is a number somebody typed. Two places, because a cap is set in dollars or in
 * cents and never in fractions of a penny.
 */
function cap(micros: number): string {
  return `$${(micros / MICROS).toFixed(2)}`;
}

/**
 * The button that opens the raise dialog, and the dialog itself.
 *
 * **The dialog is the confirmation**, which is the field-collecting exception
 * the design contract names: nothing is destroyed by pressing it, and closing
 * it any other way sends nothing. `confirmDisabled` keeps the send control off
 * unless the figure is above the cap in force, matching the 422 Fleet would
 * answer rather than round-tripping to learn it.
 *
 * **It states both figures before it asks for one.** What the job has spent and
 * what it is allowed are on the job's own header, but somebody deciding how
 * much more to give it should not have to close this to read them — and the new
 * cap is the one number on this screen that means nothing without the other
 * two.
 *
 * **It says what it does not change.** The tiers above this job stay where they
 * are and so does the turn cap: a raise read as loosening the repository would
 * be the opposite of the reason this act is per job.
 */
export function RaiseCapControl({
  jobId,
  spend,
  disabled,
  open,
  onOpen,
  onRaise,
}: {
  jobId: string;
  /**
   * What the job has spent and what it is allowed, from `GET /jobs/:job_id`.
   *
   * **Required, and no default.** A dialog that could not name the cap in force
   * would be asking for a number against nothing, and the caller draws this
   * only on a job whose detail has arrived.
   */
  spend: JobSpend;
  disabled: boolean;
  /**
   * Whether the dialog is up. **Held by the screen and not here**, for
   * `ReportControl`'s reason: `B` opens it too, and the keyboard is bound one
   * level up — see `detail-keys.ts`.
   */
  open: boolean;
  onOpen: (up: boolean) => void;
  /** The new ceiling, in millionths of a dollar — the unit `spend` reads in. */
  onRaise: (jobId: string, costCapMicros: number) => void;
}) {
  // Dollars, as typed, held as the string rather than a number: parsing on
  // every keystroke turns "1." into 1 and moves the cursor's meaning somewhere
  // the person did not.
  const [typed, setTyped] = useState("");

  const suggested = ((spend.cost_cap_micros * OFFERED) / MICROS).toFixed(2);
  // **The offered figure stands in for an untouched field**, rather than a
  // press seeding it: `B` opens this without going through the button, so there
  // is no press to seed on — and a dialog asking for a number against a blank
  // field asks a person to do arithmetic the screen has both halves of.
  const field = typed === "" ? suggested : typed;
  const asked = Number(field);
  const askedMicros = Math.round(asked * MICROS);
  const raises = Number.isFinite(asked) && askedMicros > spend.cost_cap_micros;

  function close() {
    onOpen(false);
    setTyped("");
  }

  return (
    <>
      <Button variant="secondary" disabled={disabled} onClick={() => onOpen(true)}>
        {RAISE_CAP_LABEL}
      </Button>
      <Dialog
        open={open}
        tone="neutral"
        title="Raise this job's cost cap?"
        confirmLabel={RAISE_CAP_LABEL}
        confirmDisabled={!raises}
        // Pinned outside the region that scrolls, for `Overrule.tsx`'s reason:
        // the figure is what the confirm control waits on, so it stays
        // reachable at any window height.
        field={
          // No `autoFocus`: the dialog's own contract puts initial focus on
          // Cancel, and a second claim on it here would only lose to it.
          <Input
            label="New cap, in dollars"
            mono
            inputMode="decimal"
            value={field}
            invalid={!raises}
            message={`A new cap has to be above ${cap(spend.cost_cap_micros)}, which is what this job is held to now.`}
            onChange={(event) => setTyped(event.target.value)}
          />
        }
        onCancel={close}
        onConfirm={() => {
          const sent = askedMicros;
          close();
          onRaise(jobId, sent);
        }}
      >
        {/* The two figures the new one is decided against, said here rather
            than left on the header behind the dialog. The spend is hedged and
            the cap is not, which is the distinction those two draw everywhere
            else: one is what a run would have cost at list price, and the other
            is a number somebody set. */}
        <p>
          {`This job has spent about ${cap(spend.cost_micros)} of ${cap(spend.cost_cap_micros)}, across ` +
            `${spend.drones} ${spend.drones === 1 ? "drone" : "drones"}. Raising the cap lets the next drone ` +
            "start; nothing already done is thrown away and the job stays exactly where it is."}
        </p>
        {/* What it does not touch. This paragraph argued until Sept 2026
            that the turn cap wanted no lever, because a job over it was going
            in circles and a bigger number bought more circles. Job
            `01M22TYSAE0023MADDP5ZQEYGW` falsified that: it passed every Check
            and then stopped at 393 turns against 300 with a cheap summarise
            never run. The turn cap has a control of its own now, and the
            sentence sends a person to it rather than arguing against it. */}
        <p>
          This job only. The cap every other job is held to does not move, and neither does the turn
          cap — that ceiling has a control of its own on this header.
        </p>
      </Dialog>
    </>
  );
}
