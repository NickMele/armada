// The run tree's labels, marked so the keyboard can reach the control each one
// is drawn in.
//
// **A file of its own rather than a third job for `detail-keys.ts`.** That
// module owns the bindings and the attribute this writes, but it is a `.ts`
// file and this returns an element. Not `run.ts` for the mirror reason: that
// builds data and this builds elements.
//
// It exists because `j`/`k` move focus, and focus is the only cursor the tree
// can draw — so the keyboard has to reach the control the name sits in.
// Everything else done to the run goes through `openSteps`. It lived in
// `JobDetail.tsx` until that screen crossed the 900 lines the gate refuses at.

import { type RunTreeStep } from "@armada/components";

import { namesStep } from "./detail-keys";

/**
 * A step of the run, with its name marked so the keyboard can find the control
 * the name is drawn in.
 *
 * **The marker draws nothing.** It is `display: contents`, so the row lays out
 * exactly as it did with a bare string — which matters on this row, where the
 * name is the only column that flexes and the ellipsis it truncates with is the
 * whole reason the duration column never moves.
 */
export function named(step: RunTreeStep): RunTreeStep {
  return {
    ...step,
    label: (
      <span className="contents" {...namesStep(step.id)}>
        {step.label}
      </span>
    ),
  };
}
