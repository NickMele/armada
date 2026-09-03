// The Job proposer's one call, and the only place this app makes it.
//
// **Its own file for the reason `palette.ts` is one**: it is a subject rather
// than a line of wiring, and `App.tsx` is already over the length the gate warns
// at. What is here is everything the call needs that is not a screen's — the
// preload entry, the two facts a fault's payload is quoted with, and the instant
// it was taken.
//
// # It is not one of the bound calls at the top of `App.tsx`
//
// Those are stable at module scope because effects depend on them and a lambda
// rebuilt every render would feed itself. This one cannot be: it reads the
// workflow roster and Bridge's identity out of published state, so it is a
// function of the render rather than a constant of the window.
//
// # Why the instant is stamped here
//
// A payload built at render carries a timestamp that moves every second, which
// is the one thing a quotable artifact may not do. Stamped once, when the answer
// arrived, so what a person copies an hour later says when it was taken.

import { answeredAs } from "@armada/screens";
import type { Answered } from "@armada/screens";
import type { BridgeIdentity, WorkflowSummary } from "@armada/protocol";

/** What the reading needs from published state, and nothing else. */
export type Proposing = {
  /** The workflows Fleet holds, so a proposal names one rather than an id. */
  workflows: readonly WorkflowSummary[];
  /** Both protocol versions, for the payload a fault is quoted from. */
  bridge: BridgeIdentity;
};

/**
 * Read a request, and answer with what the surface draws and what the app has
 * to say elsewhere.
 *
 * **No guard here.** Nothing about this call is idempotent, and the form is
 * what stops a second press — see `DispatchJob` in `@armada/screens`. A guard
 * in two places is two answers about whether a request went out.
 */
export async function proposeRequest(request: string, seen: Proposing): Promise<Answered> {
  return answeredAs(await window.armada.proposeFromRequest(request), {
    sent: request,
    workflows: seen.workflows,
    bridge: seen.bridge,
    at: new Date().toISOString(),
  });
}
