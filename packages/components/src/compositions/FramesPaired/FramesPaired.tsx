import type { ReactNode } from "react";
import { useCallback, useId, useState } from "react";

import type { ShownFrame } from "../FramesShown/FramesShown";
import "./FramesPaired.css";

/**
 * One screen, photographed on both sides of the change.
 *
 * **What decides whether it is drawn is `same`, and that is the caller's.**
 * Which digests agree is a fact about the record, so `packages/screens` settles
 * it and this draws what it is told — a component that compared anything would
 * be the second place the rule is written.
 */
export type PairedFrame = {
  name: string;
  attempt: number;
  /** The base's frame. Absent where the change added this screen. */
  before?: ShownFrame;
  /** The branch's frame. Absent where the change removed it. */
  after?: ShownFrame;
  /** Whether the two are the same picture. Folded when they are. */
  same: boolean;
};

export type FramesPairedProps = {
  pairs: PairedFrame[];
  /** What to say where there are none. */
  emptyNote?: ReactNode;
};

/**
 * What the change did to the screen — the pairs that moved, drawn; the ones
 * that did not, folded to a line.
 *
 * **Two screenshots of one app are about the same picture, and finding the
 * difference is the whole job.** A spec that photographs ten screens
 * photographs ten of which the change touched one, so drawing all twenty side
 * by side asks a person to do the comparison this surface exists to do for
 * them.
 *
 * **The pair that moved flips in place rather than sitting side by side.** At
 * the panel's width two plates are half a column each — smaller than a phone,
 * and unreadable for a picture of a screen. One plate at full width, swapping
 * under the eye, is also simply the better instrument: a difference nobody can
 * find by comparing two images is obvious the moment one replaces the other,
 * because the changed region is the only thing that moves.
 *
 * **It opens on the after.** That is the state a person came to see; the before
 * is what they reach for. Hover, click, or hold the key — the label always says
 * which is on screen, so the reading is never ambiguous even mid-flip.
 */
export function FramesPaired({ pairs, emptyNote }: FramesPairedProps) {
  if (pairs.length === 0) {
    return emptyNote === undefined ? null : <p className="armada-pairs__empty">{emptyNote}</p>;
  }
  return (
    <ul className="armada-pairs">
      {pairs.map((pair) => (
        <li className="armada-pairs__pair" key={`${pair.attempt} ${pair.name}`}>
          {pair.same ? <Unmoved pair={pair} /> : <Moved pair={pair} />}
        </li>
      ))}
    </ul>
  );
}

/**
 * A pair the change did not touch, folded to one line.
 *
 * **Folded and not dropped.** A screen that did not move is a fact worth
 * having — it is what says the change stayed where it was asked to — and a
 * reader who wants to check the claim can open it. That is `RunTree`'s rule for
 * a spent attempt, one surface over: the outcome stays, the working folds.
 */
function Unmoved({ pair }: { pair: PairedFrame }) {
  const [open, setOpen] = useState(false);
  const bodyId = useId();
  return (
    <div className="armada-pairs__unmoved" data-open={open || undefined}>
      <button
        className="armada-pairs__fold"
        type="button"
        aria-expanded={open}
        aria-controls={bodyId}
        onClick={() => setOpen((was) => !was)}
      >
        <span className="armada-pairs__chevron" aria-hidden>
          {open ? "⌄" : "›"}
        </span>
        <span className="armada-pairs__name">{pair.name}</span>
        <span className="armada-pairs__said">unchanged</span>
      </button>
      {/* Kept in the document while folded so `aria-controls` names something
          real, which is `Chapter`'s arrangement and for its reason. */}
      <div className="armada-pairs__body" id={bodyId} hidden={!open}>
        <Flip pair={pair} />
      </div>
    </div>
  );
}

/** A pair that differs. Drawn, always, and never behind a control. */
function Moved({ pair }: { pair: PairedFrame }) {
  return (
    <div className="armada-pairs__moved">
      <Flip pair={pair} />
    </div>
  );
}

/**
 * One plate, showing one side at a time.
 *
 * **Where a side is missing there is nothing to flip to**, and the plate says
 * which of the two happened rather than drawing a control that does nothing. A
 * frame the change added has no before; one it removed has no after. Neither is
 * a fault, and the new-screen case is the commonest thing `#209` is for.
 */
function Flip({ pair }: { pair: PairedFrame }) {
  const [showingBefore, setShowingBefore] = useState(false);
  const both = pair.before !== undefined && pair.after !== undefined;
  // With one side missing the plate shows whichever exists, whatever the state
  // says — so a stale `true` from a previous render can never blank it.
  const drawn = both ? (showingBefore ? pair.before : pair.after) : (pair.after ?? pair.before);
  const side = both ? (showingBefore ? "before" : "after") : pair.after ? "after" : "before";

  const flip = useCallback((to: boolean) => setShowingBefore(to), []);

  return (
    <div className="armada-pairs__flip">
      <div
        className="armada-pairs__plate"
        data-pressable={both || undefined}
        {...(both
          ? {
              role: "button",
              tabIndex: 0,
              "aria-label": `Show the ${showingBefore ? "after" : "before"} of ${pair.name}`,
              onClick: () => flip(!showingBefore),
              onMouseEnter: () => flip(true),
              onMouseLeave: () => flip(false),
              onKeyDown: (event: React.KeyboardEvent) => {
                if (event.key === " " || event.key === "Enter") {
                  event.preventDefault();
                  flip(true);
                }
              },
              onKeyUp: (event: React.KeyboardEvent) => {
                if (event.key === " " || event.key === "Enter") {
                  event.preventDefault();
                  flip(false);
                }
              },
            }
          : {})}
      >
        <span className="armada-pairs__tag" data-side={side}>
          {side}
        </span>
        <Content frame={drawn} name={pair.name} side={side} />
      </div>
      <p className="armada-pairs__caption">
        <span className="armada-pairs__name">{pair.name}</span>
        {both ? (
          <span className="armada-pairs__hint">hold to compare</span>
        ) : (
          <span className="armada-pairs__only" data-kind={pair.after ? "added" : "removed"}>
            {pair.after ? "nothing was here before" : "this screen is gone"}
          </span>
        )}
      </p>
    </div>
  );
}

/**
 * One side's plate content, drawn by kind — or the reason there is none.
 *
 * **A pair is `#209`'s reading of two screenshots, and stays sized for one.**
 * Video and text can be paired in principle — the same spec run against two
 * checkouts — but nothing produces a paired video or a paired log today, since
 * the base run this draws against is off (`#602`). Handling every kind here
 * rather than assuming `src` keeps that true if the base run comes back before
 * this component is looked at again.
 */
function Content({
  frame,
  name,
  side,
}: {
  frame: ShownFrame | undefined;
  name: string;
  side: "before" | "after";
}) {
  if (frame?.content === undefined) {
    return <span className="armada-pairs__why">{frame?.why ?? "reading…"}</span>;
  }
  if (frame.content.kind === "image") {
    // **The name and never a description.** Alt text saying what the frame
    // shows would be the caption this surface refuses.
    return (
      <img className="armada-pairs__image" src={frame.content.src} alt={`${name}, ${side}`} />
    );
  }
  if (frame.content.kind === "video") {
    return (
      <video
        className="armada-pairs__video"
        src={frame.content.src}
        controls
        preload="metadata"
        aria-label={`${name}, ${side}`}
      />
    );
  }
  return <pre className="armada-pairs__text">{frame.content.text}</pre>;
}
