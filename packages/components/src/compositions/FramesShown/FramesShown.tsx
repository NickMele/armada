import type { ReactNode } from "react";

import "./FramesShown.css";

/**
 * What a fetched frame draws as, once the caller knows what it is.
 *
 * **The kind and the payload travel together**, so a component reading `kind`
 * can narrow to the field that kind actually carries rather than trusting a
 * `src` that a text frame never set. `image` and `video` mint a `blob:` URL —
 * the caller's job, since it needs a live Fleet — and `text` and `json` carry
 * the decoded string itself: neither is drawn through an element that would
 * treat it as markup.
 *
 * **No `svg` or `html`.** Fleet answers both as `application/octet-stream` on
 * purpose — a browser executes either as a document — so neither reaches this
 * type. A caller that got bytes back for one of them is in the `unshowable`
 * case below, same as any kind Bridge does not know.
 */
export type FrameContent =
  | { kind: "image"; src: string }
  | { kind: "video"; src: string }
  | { kind: "text"; text: string }
  | { kind: "json"; text: string };

/**
 * One frame, as this component is handed it.
 *
 * **The bytes are the caller's problem and the drawing is this one's.** A frame
 * reaches the renderer as an array over the preload and becomes a `blob:` URL
 * or a decoded string; that is a screen's job and needs a live Fleet, so what
 * arrives here is a resolved [`FrameContent`] or a reason there is not one. It
 * is what lets this component be drawn in Storybook, and what keeps one fetch
 * out of a component that may be mounted twice.
 */
export type ShownFrame = {
  /** What a caller names it by. Stable, and the key. */
  kept: string;
  /**
   * What the harness called it — the spec's own words.
   *
   * **The only words there are.** A harness that names a frame
   * `job-detail-refused.png` has said something and one that names it `1.png`
   * has not, and Armada neither renames nor supplies a default.
   */
  name: string;
  /** Which run of the step produced it, counted from one. */
  attempt: number;
  /**
   * Which checkout it is a photograph of, in the words a reviewer thinks in.
   *
   * **`before` and `after`, not `base` and `branch`.** The wire's words name
   * the two checkouts, which is what Fleet had to know; what a person reading
   * a step wants is which of these is the screen as it was. The mapping is the
   * caller's, one layer up, so this component never learns what a base branch
   * is.
   *
   * **Absent is a step with one set of frames**, which is a repository with no
   * base, a base run that would not start, or a Fleet older than 9.5. A label
   * on a frame with nothing to compare it to would be noise.
   */
  side?: "before" | "after";
  /** What the file weighs. Drawn as read, so a slow one says why it is slow. */
  weight: string;
  /**
   * What to draw, once the caller has it.
   *
   * **Absent is not an error.** It is the ordinary state for the moment before
   * a fetch answers, and `why` is what tells that apart from a read that
   * settled with nothing to draw — a failed read, a kind Bridge does not know,
   * or a file held back for its size. All three are the frame's own business,
   * never the whole step's, so the plate says it rather than the list.
   */
  content?: FrameContent;
  /**
   * Why there is no `content`, where the reason is worth saying. Absent
   * alongside an absent `content` is a frame still being read.
   */
  why?: ReactNode;
};

export type FramesShownProps = {
  /** Oldest run first, exactly as the record answered. **No sort here.** */
  frames: ShownFrame[];
  /**
   * What to say where there are none.
   *
   * **A step with no frames is the ordinary case**, not a failure: most steps
   * declare no `shown` evidence at all. A caller that knows which silence this
   * is says so; this draws whatever it is given.
   */
  emptyNote?: ReactNode;
  /**
   * Opened, where a caller can open one. A frame at panel width is a thumbnail
   * of a screen, and a screen shrunk to 602px is not evidence of anything.
   */
  onOpen?: (kept: string) => void;
};

/**
 * The frames a step's harness produced — what a change whose point is not
 * the code is reviewed by. Images and their provenance, nothing that
 * reads as a claim: each frame carries the name the spec gave it, the run
 * it came from and what it weighs. There is no caption and no field for
 * one — a screenshot of the wrong state looks exactly like the right
 * state, so only the spec that produced it, in the diff beside the
 * change, makes a frame checkable. A sentence here would be the Drone
 * attesting to its own work in a field nothing can check.
 */

/**
 * The run is on every frame, not implied by grouping: a step worked three
 * times captured three different screens, and a reader counting down a
 * list to work out which run they were looking at would be guessing at
 * the one thing that decides whether a frame is current.
 *
 * A frame that is still being read draws its own space — the box is
 * sized before the bytes arrive, so a chapter does not jump as three
 * images land, which is the reflow that makes a person lose the one they
 * were reading.
 */
export function FramesShown({ frames, emptyNote, onOpen }: FramesShownProps) {
  if (frames.length === 0) {
    return emptyNote === undefined ? null : (
      <p className="armada-frames__empty">{emptyNote}</p>
    );
  }
  return (
    <ul className="armada-frames">
      {frames.map((frame) => (
        <li className="armada-frames__frame" key={frame.kept}>
          <Plate frame={frame} onOpen={onOpen} />
          <p className="armada-frames__said">
            <span className="armada-frames__name">{frame.name}</span>
            <span className="armada-frames__from">
              {said(frame)}
            </span>
          </p>
        </li>
      ))}
    </ul>
  );
}

/**
 * The line under a frame: which run, which side, what it weighs.
 *
 * **The side sits between the run and the weight** because that is the order a
 * reader asks in — is this current, is this the before, how long will it take
 * to open. Absent where a step has only one set, for `ShownFrame.side`'s
 * reason.
 */
function said(frame: ShownFrame): string {
  const parts = [`attempt ${frame.attempt}`];
  if (frame.side !== undefined) parts.push(frame.side);
  parts.push(frame.weight);
  return parts.join(" · ");
}

/**
 * The frame, drawn as its own kind, or the box where it will be.
 *
 * **A button only where pressing does something, and only for an image.** A
 * frame nobody can open is a figure and not a control — `onOpen` absent is a
 * record being read, which is the same rule the file rail follows one
 * composition over. A frame with no content yet is never a control either:
 * there is nothing behind it to open. Video keeps its own controls rather than
 * sitting inside a button that would steal their clicks; text and JSON have
 * nothing an *open* would show that is not already on the plate.
 */
function Plate({ frame, onOpen }: { frame: ShownFrame; onOpen?: (kept: string) => void }) {
  if (frame.content === undefined) {
    return (
      <span className="armada-frames__plate" data-empty>
        {/* Absent with no reason is a read in flight, and it says so rather
            than drawing a blank that reads as a frame of a blank page — which
            is the one thing this surface must never be mistaken for. A kind
            Bridge cannot draw and a file held back for its size land here
            too, each with its own sentence — the name is already on the line
            below, so the plate need only say why there is nothing on it. */}
        <span className="armada-frames__why">{frame.why ?? "reading…"}</span>
      </span>
    );
  }
  if (frame.content.kind === "image") {
    const image = (
      <img
        className="armada-frames__image"
        src={frame.content.src}
        // **The name and never a description.** Alt text saying what the frame
        // shows would be the caption this surface refuses, written by whoever
        // wired it rather than by the spec.
        alt={frame.name}
      />
    );
    if (onOpen === undefined) return <span className="armada-frames__plate">{image}</span>;
    return (
      <button
        className="armada-frames__plate"
        type="button"
        onClick={() => onOpen(frame.kept)}
        aria-label={`Open ${frame.name}`}
      >
        {image}
      </button>
    );
  }
  if (frame.content.kind === "video") {
    return (
      <span className="armada-frames__plate">
        <video
          className="armada-frames__video"
          src={frame.content.src}
          controls
          preload="metadata"
          aria-label={frame.name}
        />
      </span>
    );
  }
  // text or json — the record's own words, never rendered as markup.
  return (
    <span className="armada-frames__plate armada-frames__plate--text">
      <pre className="armada-frames__text">{frame.content.text}</pre>
    </span>
  );
}
