// How a failure is drawn, and what its actions do.
//
// The three failures are built in `failures.ts` and every one of them arrives
// here, because the shape is shared and the sentences are not — the same
// discipline as six Job states through one row shape. A generic error screen is
// three failures given one *sentence*, which is what this repairs.

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
 * **This is what `c` calls.** The contextual key map binds `c` to copy debug
 * info on the focused row or the open job, and the key handler is not this
 * file's — so the act is exported as one function rather than left inside the
 * control's `onClick`, where a binding could only have reimplemented it.
 *
 * It stamps the instant here rather than at render. The payload is rebuilt on
 * every draw and `taken` is a fact about the press: a banner is a standing
 * condition somebody copies long after it appeared, which is the whole reason
 * the tail labels it.
 *
 * The write itself is `@armada/components`' — one implementation of the act,
 * shared with the control the error treatment draws.
 */
export function copyDebugInfoFor(failure: Failure, onCopied: (what: string) => void): void {
  copyDebugInfo({ ...failure.payload, at: new Date().toISOString() }, onCopied);
}

/**
 * What a failure offers to an issue tracker, composed when somebody opens the
 * review.
 *
 * **The envelope and nothing else, and the transcript's absence is said out
 * loud.** The four other items the drawing named are not reachable from a
 * failure surface: doctor is not built, a judge response and a diff belong to a
 * Job read whole that none of these five failures holds, and whether an
 * observed transcript may leave the machine is `[observe-transcript-sharing]`,
 * which is open. Only the transcript is named on screen, because it is the only
 * one somebody would look for and find missing.
 *
 * It stamps the instant here for the reason `copyDebugInfoFor` does — `taken`
 * is a fact about the press, and this is the press. A banner is a standing
 * condition redrawn every second, and a payload rebuilt on each render would
 * tick under the person reading it.
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
 * Ghost controls, because none of them is a decision Armada participates in:
 * reloading redraws a window, and copying puts the machine's own record of the
 * failure onto the clipboard so nobody retypes a stack.
 *
 * **Copying and filing are two acts and the second one has a review.** Copying
 * stays on the machine; an issue is public and permanent, so `File an issue`
 * opens a dialog showing exactly what would go and never sends on one press.
 * Nothing sends at all — see `@armada/components`' `issue.ts`.
 *
 * **The label is the key map's verb, not a second name for one act.** `c` is
 * bound to copy debug info, the palette displays that wording beside the
 * binding, and this control says the same. It also happens to be the better
 * label on its own: "Copy report" named an act somebody might be about to
 * perform and left the artifact unnamed, and what a person is deciding is
 * whether to paste a machine record into a public issue.
 */
export function FailureBlock({
  failure,
  onCopied,
  reloadable = true,
  onDismiss,
}: FailureBlockProps) {
  return (
    <FailureNotice
      headline={failure.headline}
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
