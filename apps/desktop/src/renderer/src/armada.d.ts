import type { BridgeApi } from "../../shared/bridge";
import type { Proposed } from "@armada/screens";

// What the preload put on the window, and the whole of what the renderer can
// reach. Declared rather than imported at runtime: the preload is a wire, not
// an import path.
//
// # The proposer, declared here until `BridgeApi` carries it
//
// The two halves of the Job proposer were built either side of the seam at
// once, and `apps/desktop/src/shared/bridge.ts` belongs to the half that owns
// the preload. This is the surface half naming the one entry it calls, so the
// renderer typechecks against the wire it was written for rather than against a
// cast — a cast would still compile on the day the real signature turns out to
// disagree, and this does not.
//
// **Delete the intersection when `BridgeApi` declares `proposeFromRequest`.**
// Two declarations of one entry is exactly the drift this file's own comment
// warns about; it is legal for as long as it takes the two halves to meet.
type Proposer = {
  /**
   * Read a request — a prompt, or a link to a ticket — and draft every Job it
   * became, each at `awaiting_approval`. One model call, answered once.
   *
   * **No in-flight guard here.** Two calls are two plans, so the form is what
   * stops a second press — see `DispatchJob` in `@armada/screens`.
   */
  proposeFromRequest: (request: string) => Promise<Proposed>;
};

declare global {
  interface Window {
    armada: BridgeApi & Proposer;
  }
}

export {};
