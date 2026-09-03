// Dispatch a job: describe the work, or fill the form in by hand.
//
// **One surface with two ways through it, and they are not equal.** Describing
// the work is the path — the Job proposer reads the request and answers the
// title, the workflow and, where the work is several jobs, the order between
// them. `docs/concepts/job-proposer.md` calls hand entry the override, and the
// composer this swaps to is what hand entry became.
//
// # The proposal is this screen's state, not the app's
//
// Nothing outside this surface reads it, it dies when the surface closes, and
// the app holding it made the guard below depend on the app re-rendering in
// time. What does cross back is the half the app draws: a refusal with no
// drawing here is an `Outcome`, and `answeredAs` is what decides which of the
// two an answer is.
//
// # The double-press guard, and why it is two things
//
// There is no in-flight guard on the preload call, so two presses are two model
// calls and two drafted plans — two of everything at the gate, and somebody
// deleting one by hand. **The form is what stops it.**
//
// The control being off while a call is out is the first half and the one a
// person sees. The second half is the ref: a press that arrives anyway sends
// nothing, because one request is outstanding until its promise settles. A
// disabled attribute is a rendering, and a rendering is not a guarantee — a key
// repeat and a synthetic click both reach the handler with the button drawn
// live.

import { useRef, useState } from "react";
import type { ReactNode } from "react";

import { Button, DispatchRequest } from "@armada/components";
import type { Proposal, ProposalWatch } from "@armada/components";

import type { Answered } from "./proposal";
import { PROPOSAL_IS_SLOW } from "./proposal";

/** What the control that goes back to describing is called. */
const BACK = "Describe the work instead";

export type DispatchJobProps = {
  /**
   * Send the request, and answer with what came back.
   *
   * **A promise rather than a callback**, for the reason `onReadCall` is one:
   * the answer belongs to the press that asked for it and to nothing else, so
   * publishing it as app state would make one person's gesture part of what
   * every surface re-renders on.
   *
   * Called at most once per answer, and never with a blank request.
   */
  onPropose: (request: string) => Promise<Answered>;
  /** Open one of the jobs that came back, where its own gate is drawn. */
  onOpen: (jobId: string) => void;
  /** Hand entry, built by the caller. The composer, and the override. */
  byHand: ReactNode;
  /**
   * What Fleet says the call in flight is doing, or `null`.
   *
   * **The app's, where the proposal is this screen's.** The two are not the
   * same fact and cannot come from the same place: the proposal is what the
   * press asked for and belongs to the press, and this arrives on the event
   * stream between the asking and the answer. A screen holding its own copy
   * would have nothing to fill it from.
   */
  watching: ProposalWatch | null;
  /**
   * Stop the call. **Kills it rather than stopping the wait** — a wait
   * abandoned leaves the proposer running inside Fleet and spending.
   */
  onStop: () => void;
  /** Nothing may be dispatched while the connection is not live. */
  disabled: boolean;
  /** Why the controls are off, where they are. */
  disabledNote?: ReactNode;
  /** What the surface is told after a clipboard write, so it can raise a toast. */
  onCopied?: (what: string) => void;
};

export function DispatchJob({
  onPropose,
  onOpen,
  byHand,
  watching,
  onStop,
  disabled,
  disabledNote,
  onCopied,
}: DispatchJobProps) {
  const [request, setRequest] = useState("");
  const [proposal, setProposal] = useState<Proposal>({ at: "unasked" });
  // Which of the two ways through this surface is open. Describing is the
  // path, so it is what the surface opens on.
  const [hand, setHand] = useState(false);
  // One request outstanding at a time. A ref rather than state because nothing
  // renders from it: it is the guard, not a reading, and a re-render between
  // the press and the answer would be a second chance to fire.
  const outstanding = useRef(false);

  async function dispatch(): Promise<void> {
    const asked = request.trim();
    if (outstanding.current || disabled || asked === "") return;
    outstanding.current = true;
    setProposal({ at: "reading" });
    try {
      const read = await onPropose(asked);
      setProposal(read.proposal);
      // Fleet's echo, put back in the field. A refused request comes back
      // unchanged, and this is what makes that true rather than only stated.
      if (read.request !== null) setRequest(read.request);
    } catch (thrown) {
      // A surface left on `reading` is worse than the throw: nothing on it says
      // the call is dead and the control never comes back. The throw carries on
      // to the app's own handler for a rejection nothing caught.
      setProposal({ at: "unasked" });
      throw thrown;
    } finally {
      outstanding.current = false;
    }
  }

  function reset(): void {
    setRequest("");
    setProposal({ at: "unasked" });
  }

  if (hand) {
    return (
      <div className="flex flex-col gap-4">
        {/* The way back, above the form. Hand entry is the exception, so
            leaving it is one press and never a dead end. */}
        <div>
          <Button variant="secondary" size="sm" onClick={() => setHand(false)}>
            {BACK}
          </Button>
        </div>
        {byHand}
      </div>
    );
  }

  return (
    <DispatchRequest
      request={request}
      onRequest={setRequest}
      onDispatch={() => void dispatch()}
      onEnterByHand={() => setHand(true)}
      onReset={reset}
      onOpen={onOpen}
      // The two facts are joined here and nowhere else: the proposal is this
      // screen's and the watch is the app's, and the component takes one value.
      // A screen still `reading` with nothing published yet draws the wait
      // without a reading, which is the sentence that was always there.
      proposal={
        proposal.at === "reading" && watching !== null
          ? { at: "reading", watch: watching }
          : proposal
      }
      onStop={onStop}
      slowAfterMs={PROPOSAL_IS_SLOW}
      disabled={disabled}
      disabledNote={disabledNote}
      onCopied={onCopied}
    />
  );
}
