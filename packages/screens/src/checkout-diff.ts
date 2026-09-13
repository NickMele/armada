// What `get_checkout_run_diff` came back as, held apart from `checkout-runs.ts`
// so main can name it without compiling a React hook.
//
// **Bridge-only, not on the wire** — `CheckoutRunListRead`'s kind, and here
// rather than beside it only because `packages/protocol` is Fleet's half of
// #782 and was closed when this half was built. It belongs there, and moves
// the next time that file is open.

import type { CheckoutRunDiff, Outcome } from "@armada/protocol";

/**
 * One run's patch, or why there is none. **Answered to the caller rather than
 * held as state**, `CheckoutRunListRead`'s reason: a finished run's diff does
 * not move, and only the person who pressed *Open the diff* wants it.
 */
export type CheckoutRunDiffRead =
  | { ok: true; diff: CheckoutRunDiff }
  | { ok: false; outcome: Outcome };
