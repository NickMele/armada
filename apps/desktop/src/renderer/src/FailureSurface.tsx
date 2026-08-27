// How a failure is drawn, and what its actions do.
//
// The three failures are built in `failures.ts` and every one of them arrives
// here, because the shape is shared and the sentences are not — the same
// discipline as six Job states through one row shape. A generic error screen is
// three failures given one *sentence*, which is what this repairs.

import { Button, FailureNotice } from "@armada/components";

import type { BridgeIdentity } from "../../shared/bridge";
import { CopiedToast, useCopied } from "./CopiedToast";
import type { Caught, Failure } from "./failures";
import { rendererFailure, reportOf } from "./failures";

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
 * reloading redraws a window, and reporting puts what is on screen onto the
 * clipboard so nobody retypes a stack.
 */
export function FailureBlock({
  failure,
  onCopied,
  reloadable = true,
  onDismiss,
}: FailureBlockProps) {
  function report(): void {
    const text = reportOf(failure, new Date().toISOString());
    void navigator.clipboard.writeText(text).then(
      () => onCopied("The report"),
      // A failed clipboard write is otherwise indistinguishable from a dead
      // control, so the surface is told either way.
      () => onCopied("The report"),
    );
  }

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
          <Button variant="ghost" size="sm" ground="sunken" onClick={report}>
            Copy report
          </Button>
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
