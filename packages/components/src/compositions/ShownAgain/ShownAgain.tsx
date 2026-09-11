import type { ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { FramesShown, type ShownFrame } from "../FramesShown/FramesShown";

import "./ShownAgain.css";

/**
 * Whether the press can be made, as the control is handed it.
 *
 * **The reason is the caller's words**, because which of Fleet's facts stands
 * in the way is a reading of a Job, and this component draws one sentence
 * without knowing what a worktree is.
 */
export type ShowAgainOffer =
  /** The press can run. `spec` is what it will run, in the Drone's words. */
  | { state: "ready"; spec: string }
  /** A press is out. Nothing more can be sent until it answers. */
  | { state: "showing"; spec: string }
  /** It cannot run, and `why` says what is missing and what to do. */
  | { state: "cannot"; why: ReactNode };

/**
 * One press's frames, with the line a person tells it apart by.
 *
 * **`heading` is when it ran**, which is the owner's decision on #603: a press
 * adds a set and never replaces one, so the moment is what separates two of
 * them — and it is drawn above the frames rather than on every one of them.
 */
export type ShownAgainSet = {
  /** Stable, and the key. */
  key: string;
  heading: ReactNode;
  frames: ShownFrame[];
};

export type ShownAgainProps = {
  /**
   * Whether the press can be made. **Absent draws no control**, only the sets
   * — a step whose spec is not the one a press would rerun still shows what
   * earlier presses kept on it.
   */
  offer?: ShowAgainOffer;
  /** Every press this Job kept for the step, oldest first. **No sort here.** */
  sets: ShownAgainSet[];
  /**
   * What the last press from this window came to, where it was not a new set
   * — a spec that captured nothing, or a refusal Fleet answered with.
   * **Said beside the control that was pressed**, because a press that
   * produced nothing on screen and nothing in words is a dead click.
   */
  said?: ReactNode;
  onShow: () => void;
  /** Passed to each set's frames unchanged. See `FramesShown`. */
  onOpen?: (kept: string) => void;
};

/**
 * Asking a Job to show its work again, and every time somebody did.
 *
 * **The control and its reason are one line**, because a disabled button with
 * nothing beside it asks a person to guess why. What it will run is said while
 * it can run, and what stands in the way is said while it cannot.
 *
 * **Each set is an unchanged `FramesShown`.** A press's frames are frames like
 * the step's own, and drawing them any other way would be a second answer to
 * what a frame looks like. What this adds is the line above each set.
 */
export function ShownAgain({ offer, sets, said, onShow, onOpen }: ShownAgainProps) {
  return (
    <div className="armada-again">
      {offer === undefined ? null : (
        <div className="armada-again__control">
          <Button
            variant="secondary"
            onClick={onShow}
            disabled={offer.state !== "ready"}
            aria-busy={offer.state === "showing" || undefined}
          >
            {offer.state === "showing" ? "Showing…" : "Show again"}
          </Button>
          <p className="armada-again__note">
            {offer.state === "cannot" ? (
              offer.why
            ) : (
              <>
                {offer.state === "showing" ? "Running " : "Runs "}
                <code className="armada-again__spec">{offer.spec}</code> in this Job's worktree
              </>
            )}
          </p>
        </div>
      )}
      {said === undefined ? null : (
        <p className="armada-again__said" role="status">
          {said}
        </p>
      )}
      {sets.map((set) => (
        <section key={set.key} className="armada-again__set" aria-label={labelOf(set.heading)}>
          <h4 className="armada-again__heading">{set.heading}</h4>
          <FramesShown frames={set.frames} onOpen={onOpen} />
        </section>
      ))}
    </div>
  );
}

/** A set's accessible name, where its heading is plain text. */
function labelOf(heading: ReactNode): string | undefined {
  return typeof heading === "string" ? heading : undefined;
}
