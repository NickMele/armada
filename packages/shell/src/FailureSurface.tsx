// How a failure is drawn, and what its actions do.
//
// The six failures are built under `failures/` and every one of them arrives
// here, because the shape is shared and the sentences are not — the same
// discipline as six Job states through one row shape. A generic error screen is
// six failures given one *sentence*, which is what this repairs.

import {
  Button,
  COPY_DEBUG_INFO,
  copyDebugInfo,
  envelopeOf,
  FailureNotice,
  FileAnIssue,
  NOT_OFFERED,
} from "@armada/components";
import type { Filing } from "@armada/components";

import type { BridgeIdentity } from "@armada/protocol";
import { CopiedToast, useCopied } from "./CopiedToast";
import type { Caught, Failure } from "./failures";
import { rendererFailure } from "./failures";

/**
 * Copy debug info, for one of Bridge's failures.
 *
 * **This is what `c` calls** — the key map binds `c` on the focused row or
 * open job, and the handler lives outside this file, so the act is exported
 * rather than left inside a control's `onClick`.
 *
 * Stamps the instant here, not at render: the payload rebuilds on every draw
 * and `taken` is a fact about the press — a banner is a standing condition
 * somebody copies long after it appeared.
 *
 * The write itself is `@armada/components`', shared with the control the
 * error treatment draws.
 */
export function copyDebugInfoFor(failure: Failure, onCopied: (what: string) => void): void {
  copyDebugInfo({ ...failure.payload, at: new Date().toISOString() }, onCopied);
}

/**
 * What a failure offers to an issue tracker, composed when somebody opens
 * the review.
 *
 * **The envelope and nothing else — the transcript's absence is said out
 * loud.** The other four items the drawing named are unreachable here:
 * doctor is not built, a judge response and a diff belong to a Job read
 * whole that none of these six failures holds, and whether an observed
 * transcript may leave the machine is `[observe-transcript-sharing]`, open.
 * Only the transcript is named, since it is the one somebody would look for
 * and find missing.
 *
 * Stamps the instant here for `copyDebugInfoFor`'s reason: a banner redrawn
 * every second would tick under the reader if rebuilt on each render.
 */
export function filingFor(failure: Failure): Filing {
  return {
    title: failure.payload.message,
    attached: [envelopeOf({ ...failure.payload, at: new Date().toISOString() })],
    withheld: NOT_OFFERED,
  };
}

export type FailureBlockProps = {
  failure: Failure;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied: (value: string) => void;
  /**
   * Whether reloading is one of the acts. It is on every failure that a redraw
   * re-runs — a reconnect, a re-render, a re-read of the board — and off on a
   * refusal, where the command was answered and reloading answers nothing.
   */
  reloadable?: boolean;
  /** Where the failure is a standing answer a person clears rather than fixes. */
  onDismiss?: () => void;
};

/**
 * One failure, with something to do about it.
 *
 * Ghost controls — none is a decision Armada participates in: reloading
 * redraws a window, copying puts the record on the clipboard.
 *
 * **Copying and filing are two acts; only filing has a review** — an issue
 * is public and permanent, so `File an issue` opens a dialog showing exactly
 * what would go and never sends on one press — `issue.ts`.
 *
 * **The label is the key map's verb** — `c` is bound to copy debug info, the
 * palette shows that wording, and this control says the same: "Copy report"
 * left the artifact unnamed, when the decision is pasting a machine record
 * into a public issue.
 */
export function FailureBlock({
  failure,
  onCopied,
  reloadable = true,
  onDismiss,
}: FailureBlockProps) {
  return (
    <FailureNotice
      // The class comes off the failure, which is the only thing that knows
      // whether Fleet is alive. It was a literal inside the notice, so all six
      // drew red and the two where Fleet is up and working said "restart me".
      kind={failure.kind}
      headline={failure.headline}
      // Read off the payload rather than off a second field beside it. The
      // chip on screen and the `code` row in the copied artifact are then the
      // same value by construction, which is the rule the payload's single
      // formatter already follows: two renderings that agree on the day they
      // were written are two artifacts by the day either one changes.
      code={failure.payload.code}
      next={failure.next}
      detailsLabel={failure.detailsLabel}
      details={failure.details}
      values={failure.values}
      note={failure.note}
      onCopied={onCopied}
      actions={
        <>
          {reloadable ? (
            <Button
              variant="ghost"
              size="sm"
              ground="sunken"
              // Bridge and Fleet have independent lifetimes: this drops a
              // window, not a daemon, and the running jobs never notice.
              onClick={() => window.location.reload()}
            >
              Reload Bridge
            </Button>
          ) : null}
          {/* No glyph and no kbd on the control. The error treatment carries
              no glyph at all, and a binding is discovered in the palette and
              the tooltip — the two surfaces the contract gives a kbd to. */}
          <Button
            variant="ghost"
            size="sm"
            ground="sunken"
            onClick={() => copyDebugInfoFor(failure, onCopied)}
          >
            {COPY_DEBUG_INFO}
          </Button>
          {/* Beside copying, never instead of it. **Copying stays on the
              machine and filing leaves it**, so the two are different acts and
              only one of them takes a review — and the review is what makes
              send never one press from the error. */}
          <FileAnIssue compose={() => filingFor(failure)} onCopied={onCopied} />
          {onDismiss === undefined ? null : (
            <Button variant="ghost" size="sm" ground="sunken" onClick={onDismiss}>
              Dismiss
            </Button>
          )}
        </>
      }
    />
  );
}

export type FailureSurfaceProps = {
  caught: Caught;
  region: string;
  /** False only for the root boundary, where nothing else survived. */
  usable: boolean;
  bridge: BridgeIdentity;
  /**
   * The app's toast layer. Absent at the root, where the app that owned it is
   * the thing that threw — so the fallback raises its own rather than copying
   * in silence.
   */
  onCopied?: (value: string) => void;
};

/**
 * What a boundary renders instead of the region it lost.
 *
 * The root boundary paints its own ground, because a fallback rendered over
 * nothing is the blank window this exists to stop — the window's background is
 * drawn by the tree that just threw.
 */
export function FailureSurface({ caught, region, usable, bridge, onCopied }: FailureSurfaceProps) {
  // The root fallback raises its own, because the app that owned the toast is
  // the thing that just threw. An inner boundary uses the app's.
  const [copied, setCopied] = useCopied();
  const failure = rendererFailure(caught, region, bridge, usable);
  const block = <FailureBlock failure={failure} onCopied={onCopied ?? setCopied} />;

  if (usable && onCopied !== undefined) return block;
  return (
    // The root fallback paints its own ground, because a fallback rendered over
    // nothing is the blank window this exists to stop — the window's background
    // was drawn by the tree that just threw.
    <div className={usable ? "" : "flex h-full flex-col overflow-y-auto bg-bg-base p-6 text-fg-default"}>
      {block}
      <CopiedToast copied={copied} />
    </div>
  );
}
