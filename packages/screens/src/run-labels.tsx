// The run tree's labels, marked so the keyboard can reach the control each one
// is drawn in.
//
// **A file of its own rather than a third job for `detail-keys.ts`.** That
// module owns the bindings and the attribute this writes, and would be the
// obvious home — but it is a `.ts` file and this returns an element, so taking
// it would turn a module about keys into one that renders. It is not in
// `run.ts` for the same reason read the other way: that file builds data and
// this one builds elements.
//
// It exists at all because `j`/`k` move focus, and focus is the only cursor
// the tree can draw — so the keyboard has to be able to reach the control the
// name sits in. Everything else done to the run goes through `openSteps`.
//
// It lived in `JobDetail.tsx` until that screen crossed the 900 lines the gate
// refuses at, which is a size to answer by moving a piece out rather than by
// trimming prose somebody wrote on purpose.

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
