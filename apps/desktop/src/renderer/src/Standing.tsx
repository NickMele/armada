// The standing conditions, above whatever surface is open.
//
// **A band, not a screen.** Everything here is true regardless of which surface
// a person is on, stays on screen until they put it away, and leaves the
// surface working beneath it — which is the design contract's own definition of
// a banner, and the reason none of it belongs in the status bar.
//
// # Why it is its own file
//
// `App.tsx` reached the 900 lines the gate refuses. This band is the one seam
// inside it that is already a seam: `App` decides what is true and this decides
// how a standing truth is drawn, so the props below are exactly the conditions
// and nothing about the surface underneath. The same remedy `protocol.ts` has
// taken three times.
//
// **Nothing here dismisses on a timer.** A person who has just saved a file, or
// gone to look at a worktree, is not looking at this window; a notice that
// expired while they were is one they cannot get back.

import { Alert, Button, ManifestNotice } from "@armada/components";
import type { ManifestReading, Outcome, WorktreeReclaimed } from "@armada/protocol";
import type { BridgeIdentity } from "@armada/protocol";
import type { Failure, Uncaught } from "@armada/shell";
import { FailureBlock, uncaughtFailure } from "@armada/shell";
import { reclaimed, said } from "@armada/screens";

export type StandingProps = {
  /** Fleet, where the one connection is not one. */
  fleet: Failure | null;
  /** What Fleet's last read of `armada.yml` came to. */
  manifestReading: ManifestReading | null;
  /** The reading already put away, by the instant Fleet read the file. */
  readingSeen: string | null;
  onReadingSeen: (at: string | null) => void;
  /** What no error boundary saw. */
  uncaught: Uncaught | null;
  onUncaught: (caught: Uncaught | null) => void;
  bridge: BridgeIdentity;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied: (value: string) => void;
  /** Events Fleet dropped, and how many of them have been read. */
  missed: number;
  acknowledged: number;
  onAcknowledged: (missed: number) => void;
  /** What a reclaim gave back. */
  givenBack: WorktreeReclaimed | null;
  onGivenBack: (given: WorktreeReclaimed | null) => void;
  /** The last command's answer, and the failure half of it where there is one. */
  commandFailure: Failure | null;
  outcome: Outcome | null;
  onOutcome: (outcome: Outcome | null) => void;
};

/** Everything true above the surface, in the order it is met. */
export function Standing({
  fleet,
  manifestReading,
  readingSeen,
  onReadingSeen,
  uncaught,
  onUncaught,
  bridge,
  onCopied,
  missed,
  acknowledged,
  onAcknowledged,
  givenBack,
  onGivenBack,
  commandFailure,
  outcome,
  onOutcome,
}: StandingProps) {
  return (
    <>
      {/* Fleet, when the one connection is not one. The status bar keeps the
          single line; this is the same reading with the four runtime-file
          answers and the log under it. */}
      {fleet === null ? null : <FailureBlock failure={fleet} onCopied={onCopied} />}

      {/* Fleet's own Manifest read. **Here and not in the status bar**, which
          the contract holds to three states and one colour and warns must not
          become a second alert surface. Put away by hand: whoever saved the
          file is looking at their editor. Nothing draws for a read with no
          news in it — `ManifestNotice` asks that itself. */}
      {manifestReading === null || readingSeen === manifestReading.at ? null : (
        <ManifestNotice
          reading={manifestReading}
          onDismiss={() => onReadingSeen(manifestReading.at)}
        />
      )}

      {/* What no boundary sees. A click that threw and a rejected preload
          call both look like a button that did nothing. */}
      {uncaught === null ? null : (
        <FailureBlock
          failure={uncaughtFailure(uncaught, bridge)}
          onCopied={onCopied}
          // Cleared by hand rather than on a timer: a failure that vanishes
          // while nobody is looking is the silence being repaired here.
          onDismiss={() => onUncaught(null)}
        />
      )}

      {missed <= acknowledged ? null : (
        <Alert
          tone="escalated"
          title="Events were dropped before Bridge saw them"
          action={
            <Button variant="ghost" size="sm" onClick={() => onAcknowledged(missed)}>
              Noted
            </Button>
          }
        >
          {`${missed} events will never arrive. Fleet resynced current state after each drop, so the list below is repaired.`}
        </Alert>
      )}

      {/* What a reclaim gave back. **Neutral, because nothing is wrong** — a
          branch kept for holding work nothing has taken is the safe setting
          working, and drawing it in the escalation hue would tell somebody the
          act failed when it did exactly what it promised. Dismissed by hand: a
          directory and a branch are what a person goes and looks at, and a
          notice that vanished while they did is one they cannot get back. */}
      {givenBack === null ? null : (
        <Alert
          tone="neutral"
          title="Worktree reclaimed"
          action={
            <Button variant="ghost" size="sm" onClick={() => onGivenBack(null)}>
              Dismiss
            </Button>
          }
        >
          {reclaimed(givenBack)}
        </Alert>
      )}

      {/* A refusal Fleet named carries a `run_id`, its `fields` and its
          `chain`, so it is drawn whole rather than as one line of copy — its
          `message` names one problem even where several exist. A command Fleet
          did not answer carries no envelope and is drawn whole for the same
          reason: the code, the route and the wait are the whole of what a
          person has to hand on. Neither is reloadable, because a redraw re-runs
          no command. Everything else here is the form telling you what it will
          not send, which is guidance and not a failure. */}
      {commandFailure !== null ? (
        <FailureBlock
          failure={commandFailure}
          onCopied={onCopied}
          reloadable={false}
          onDismiss={() => onOutcome(null)}
        />
      ) : outcome === null || outcome.ok ? null : (
        <Alert
          tone="escalated"
          action={
            <Button variant="ghost" size="sm" onClick={() => onOutcome(null)}>
              Dismiss
            </Button>
          }
        >
          {said(outcome)}
        </Alert>
      )}
    </>
  );
}
