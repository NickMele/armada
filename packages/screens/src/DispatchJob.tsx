// Dispatch a job: describe the work, or fill the form in by hand.
//
// **One surface with two ways through it, and they are not equal.** Describing
// the work is the path — the Job proposer reads the request and answers the
// title, the workflow and, where the work is several jobs, the order between
// them. `docs/concepts/job-proposer.md` calls hand entry the override, and the
// composer this swaps to is what hand entry became.
//
// # The double-press guard is here, and it is two things
//
// There is no in-flight guard on the preload call, so two presses are two model
// calls and two drafted plans — two of everything at the gate, and somebody
// deleting one by hand. **The form is what stops it.**
//
// The control being off while a call is out is the first half and the one a
// person sees. The second half is this: a press that arrives anyway sends
// nothing, because one request is outstanding until an answer moves the
// proposal off `reading`. A disabled attribute is a rendering, and a rendering
// is not a guarantee — a key repeat, a synthetic click or a caller that has not
// re-rendered yet all reach the handler with the button still drawn live.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import { Button, DispatchRequest } from "@armada/components";
import type { Proposal } from "@armada/components";

/** What the control that goes back to describing is called. */
const BACK = "Describe the work instead";

export type DispatchJobProps = {
  /** What the proposer answered, or that it has not been asked. */
  proposal: Proposal;
  /**
   * Send the request. **Called at most once per answer** — see the guard
   * above — and never with a blank one.
   */
  onDispatch: (request: string) => void;
  /** Drop what came back, so the field is a fresh request again. */
  onReset: () => void;
  /** Open one of the jobs that came back, where its own gate is drawn. */
  onOpen: (jobId: string) => void;
  /**
   * What the field should hold, where Fleet echoed a request back. Applied
   * when it changes and never otherwise, so nothing overwrites typing.
   */
  echoed: string | null;
  /** Hand entry, built by the caller. The composer, and the override. */
  byHand: ReactNode;
  /** Nothing may be dispatched while the connection is not live. */
  disabled: boolean;
  /** Why the controls are off, where they are. */
  disabledNote?: ReactNode;
  /** What the surface is told after a clipboard write, so it can raise a toast. */
  onCopied?: (what: string) => void;
};

export function DispatchJob({
  proposal,
  onDispatch,
  onReset,
  onOpen,
  echoed,
  byHand,
  disabled,
  disabledNote,
  onCopied,
}: DispatchJobProps) {
  const [request, setRequest] = useState("");
  // Which of the two ways through this surface is open. Describing is the
  // path, so it is what the surface opens on.
  const [hand, setHand] = useState(false);
  // One request outstanding at a time. A ref rather than state because nothing
  // renders from it: it is the guard, not a reading, and a re-render between
  // the press and the answer would be a second chance to fire.
  const outstanding = useRef(false);

  // The answer released it, whatever the answer was. `reading` is the only
  // state a call is out in, so anything else means this surface is free.
  useEffect(() => {
    if (proposal.at !== "reading") outstanding.current = false;
  }, [proposal]);

  // Fleet's echo, put back in the field. A refused request comes back unchanged
  // and this is what makes that true rather than merely stated.
  useEffect(() => {
    if (echoed !== null) setRequest(echoed);
  }, [echoed]);

  function dispatch(): void {
    const asked = request.trim();
    if (outstanding.current || proposal.at === "reading" || disabled || asked === "") return;
    outstanding.current = true;
    onDispatch(asked);
  }

  function reset(): void {
    setRequest("");
    onReset();
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
      onDispatch={dispatch}
      onEnterByHand={() => setHand(true)}
      onReset={reset}
      onOpen={onOpen}
      proposal={proposal}
      disabled={disabled}
      disabledNote={disabledNote}
      onCopied={onCopied}
    />
  );
}
