import type { ReactNode } from "react";
import { useCallback, useEffect, useId, useRef, useState } from "react";

/**
 * A passage held to a few lines, with the rest one press away.
 *
 * **The brief and the Drone's instructions are the reason.** Both are written
 * by a person or a model at whatever length the work needed, and both sit above
 * things a reader is trying to get to. A four-hundred-word brief pushes the run,
 * the phases and the whole story off the screen — so the passage is clamped and
 * the reader decides when to spend the room.
 *
 * **The control only exists when the text overflows.** A *View more* under three
 * lines that were never truncated is a control that does nothing, and a reader
 * who presses it once learns the surface lies. So the overflow is measured
 * rather than guessed from a character count: the same words are two lines wide
 * on one panel and five at `--window-floor`, and a length threshold cannot know
 * which.
 *
 * **It re-measures on resize.** A sheet opening beside the panel narrows it, and
 * a passage that fitted at the old width does not at the new one. Without this
 * the control disappears exactly when it starts being needed.
 *
 * **Clamping is `-webkit-line-clamp`, and the lines are a count rather than a
 * height.** A `max-height` in pixels has to be recomputed against every type
 * scale it lands in and silently cuts a line in half when it is wrong.
 */

export type ClampedProps = {
  children: ReactNode;
  /**
   * How many lines to hold it to. **A count of lines, never a height** — the
   * scale is the surface's, and a passage clamped to 64px shows three lines in
   * one place and two and a half in another.
   */
  lines?: number;
  /** The control that opens it. */
  moreLabel?: ReactNode;
  /** The control that closes it again. */
  lessLabel?: ReactNode;
  /**
   * Open on mount. A caller that knows the passage is the reason the reader is
   * here — a redirect just written, a refusal's grounds — starts it open.
   */
  defaultOpen?: boolean;
};

export function Clamped({
  children,
  lines = 3,
  moreLabel = "View more",
  lessLabel = "View less",
  defaultOpen = false,
}: ClampedProps) {
  const [open, setOpen] = useState(defaultOpen);
  // Whether the text is actually longer than the clamp. Not a guess from its
  // length: the same words wrap differently at every width this panel has.
  const [overflows, setOverflows] = useState(false);
  const held = useRef<HTMLDivElement | null>(null);
  const bodyId = useId();

  const measure = useCallback(() => {
    const node = held.current;
    if (node === null) return;
    // Read against the clamped box, so the answer is "is there more than the
    // clamp shows" rather than "is there more than the expanded box shows",
    // which is always no.
    setOverflows(node.scrollHeight > node.clientHeight + 1);
  }, []);

  useEffect(() => {
    const node = held.current;
    if (node === null) return;
    measure();
    // Width is the whole of it — a sheet opening beside the panel re-wraps the
    // passage, and a control that vanished then would go exactly when it
    // started being needed.
    const watching = new ResizeObserver(measure);
    watching.observe(node);
    return () => watching.disconnect();
  }, [measure, children]);

  return (
    <div className="armada-clamped">
      <div
        className="armada-clamped__body"
        id={bodyId}
        ref={held}
        // The clamp is off while open, and the measurement above only means
        // anything against the clamped box — so it is re-read on every close.
        data-open={open ? "true" : undefined}
        style={{ ["--armada-clamped-lines" as string]: String(lines) }}
      >
        {children}
      </div>
      {/* Only where there is more to see. A control under three lines that were
          never truncated is one that does nothing. */}
      {overflows || open ? (
        <button
          type="button"
          className="armada-clamped__more"
          aria-expanded={open}
          aria-controls={bodyId}
          onClick={() => setOpen((was) => !was)}
        >
          {open ? lessLabel : moreLabel}
        </button>
      ) : null}
    </div>
  );
}
