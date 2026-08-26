// One screen: what Fleet is, a form to propose a Job, and every Job with the
// one decision a person can make from here.
//
// Everything drawn comes from the state the main process publishes over the one
// connection. Nothing here fetches, and nothing here holds a copy of a Job that
// Fleet has not confirmed — a Job whose real state is not what the screen says
// is the failure that matters.

import { useEffect, useMemo, useState } from "react";
import { Alert, Button, Separator } from "@armada/components";

import type { BridgeState, Draft, Outcome } from "../../shared/bridge";
import { FleetBar, statementOf } from "./FleetBar";
import { Composer } from "./Composer";
import { Jobs } from "./Jobs";

/** How often the elapsed figures are redrawn. They are read, so they must move. */
const TICK_MS = 1000;

const WAITING: BridgeState = {
  connection: { state: "reading" },
  jobs: [],
  unreadable: [],
  missed: 0,
  readAt: null,
  approving: [],
};

export function App() {
  const [state, setState] = useState<BridgeState>(WAITING);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // What has been read and acknowledged. The count itself belongs to the
  // connection and is never reset from here — a drop that happened, happened.
  const [acknowledged, setAcknowledged] = useState(0);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    void window.armada.state().then(setState);
    return window.armada.subscribe(setState);
  }, []);

  useEffect(() => {
    const tick = setInterval(() => setNow(Date.now()), TICK_MS);
    return () => clearInterval(tick);
  }, []);

  const live = state.connection.state === "connected";
  const statement = statementOf(state.connection, now, state.readAt);

  // The ids Fleet has already minted, which are the only ones Bridge can offer.
  const workflows = useMemo(() => unique(state.jobs.map((job) => job.workflow_id)), [state.jobs]);
  const manifests = useMemo(
    () => unique(state.jobs.map((job) => job.owner_manifest_id)),
    [state.jobs],
  );

  async function propose(draft: Draft): Promise<void> {
    setOutcome(await window.armada.proposeJob(draft));
  }

  async function approve(jobId: string): Promise<void> {
    setOutcome(await window.armada.approveDispatch(jobId));
  }

  return (
    <div className="flex h-full flex-col bg-bg-base text-fg-default">
      <header className="flex shrink-0 items-baseline gap-3 border-b border-border-subtle bg-bg-raised p-4">
        <h1 className="text-lg">Armada</h1>
        <span className="text-fg-muted">{statement.headline}</span>
      </header>

      <main className="armada-app__scroll flex flex-1 flex-col gap-6 overflow-y-auto p-6">
        {statement.next === null ? null : (
          <Alert tone="escalated" title={statement.headline}>
            <span className="mono">{statement.detail}</span> {statement.next}
          </Alert>
        )}

        {state.missed <= acknowledged ? null : (
          <Alert
            tone="escalated"
            title="Events were dropped before Bridge saw them"
            action={
              <Button variant="ghost" size="sm" onClick={() => setAcknowledged(state.missed)}>
                Noted
              </Button>
            }
          >
            {`${state.missed} events will never arrive. Fleet resynced current state after each drop, so the list below is repaired.`}
          </Alert>
        )}

        {outcome === null || outcome.ok ? null : (
          <Alert
            tone="escalated"
            action={
              <Button variant="ghost" size="sm" onClick={() => setOutcome(null)}>
                Dismiss
              </Button>
            }
          >
            {said(outcome)}
          </Alert>
        )}

        <Composer
          workflows={workflows}
          manifests={manifests}
          disabled={!live}
          onPropose={(draft) => void propose(draft)}
        />

        <Separator />

        <Jobs
          jobs={state.jobs}
          unreadable={state.unreadable}
          approving={state.approving}
          stale={!live}
          onApprove={(jobId) => void approve(jobId)}
        />
      </main>

      <FleetBar statement={statement} />
    </div>
  );
}

/** What a refusal says. Every one names what happened and what to do. */
function said(outcome: Outcome): string {
  if (outcome.ok) return "";
  switch (outcome.why) {
    case "empty_brief":
      return "A job needs a brief. Nothing was created.";
    case "not_connected":
      return "Fleet is not connected. Nothing was sent.";
    case "already_approving":
      return "That approval is already in flight. It was not sent twice.";
    case "refused":
      return `${outcome.error.message} (${outcome.error.code})`;
    case "transport":
      return `Fleet did not answer: ${outcome.detail}`;
  }
}

function unique(values: readonly string[]): string[] {
  return [...new Set(values)].sort();
}
