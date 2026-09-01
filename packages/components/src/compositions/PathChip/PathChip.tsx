import type { ReactNode } from "react";

/**
 * A path chip — the one value in the tree that keeps its basename at every
 * width.
 *
 * **The directory truncates and the filename never does.** A run of six
 * produced paths clipped from the right all read `packages/settings/src/…`,
 * which is six rows saying nothing; clipped from the left they read
 * `…/selectors.ts`, `…/reducer.ts`, `…/index.ts`, which is what a person came
 * to the tree to learn. The drawing splits the chip into two spans for exactly
 * this, and the split is the component.
 *
 * **How the left clip is done, and why it is not a substring.** The directory
 * is laid out `direction: rtl` with an ordinary ellipsis, so the browser drops
 * characters from the *start* of the run and keeps the end — the segment
 * nearest the filename, which is the informative one. Cutting the string in
 * JavaScript would need a width measurement the component does not have and
 * would re-cut on every resize.
 *
 * **The directory recedes and the basename does not.** `--fg-subtle` against
 * `--fg-muted`: two values on one line, and the one that survives truncation
 * is the one that reads first.
 */
export type PathChipProps = {
  /**
   * Everything up to and including the final separator —
   * `packages/settings/src/`. May be empty, for a path at the repository root.
   *
   * **Keep the trailing separator.** It belongs to the directory: dropping it
   * makes `src` and `selectors.ts` read as two names rather than one path,
   * and the separator is the first thing the ellipsis should eat into.
   */
  directory?: string;
  /** The filename. Never truncated, never abbreviated. */
  basename: string;
  /**
   * What the path is, where the path alone does not say — `+61 −4`,
   * `work product`. Sans, outside the chip's mono, because it is a note about
   * the value rather than part of it.
   */
  note?: ReactNode;
  /**
   * What the whole path is, for the title. Assembled from the two halves when
   * the caller supplies neither — a truncated value needs somewhere to be read
   * in full, and hovering it is the cheapest place.
   */
  title?: string;
  /**
   * Copy the whole path. **What goes on the clipboard is the whole value, never
   * what fits** — a copy truncated with the display would be worse than the
   * overflow it fixed.
   *
   * Absent draws the chip as a value rather than as a control, which is what a
   * path in a read-only record is.
   */
  onCopy?: (path: string) => void;
};

export function PathChip({ directory, basename, note, title, onCopy }: PathChipProps) {
  const whole = title ?? `${directory ?? ""}${basename}`;

  const body = (
    <>
      {directory === undefined || directory === "" ? null : (
        // `dir="ltr"` inside an `rtl` box: the characters stay in reading
        // order and only the overflow end moves. Without it a path renders
        // its separators on the wrong side.
        <span className="armada-path__dir" dir="ltr">
          {directory}
        </span>
      )}
      <span className="armada-path__base">{basename}</span>
      {note === undefined ? null : <span className="armada-path__note">{note}</span>}
    </>
  );

  if (onCopy === undefined) {
    return (
      <span className="armada-path" title={whole}>
        {body}
      </span>
    );
  }

  return (
    <button
      type="button"
      className="armada-path"
      title={whole}
      // A tree row selects a step and the chip inside it copies a path. Without
      // this the copy would select the step as well, which is the one gesture
      // a person doing the other one did not want.
      onClick={(event) => {
        event.stopPropagation();
        void navigator.clipboard.writeText(whole).then(
          // A failed clipboard write is otherwise indistinguishable from a
          // dead element, so the surface is told either way.
          () => onCopy(whole),
          () => onCopy(whole),
        );
      }}
    >
      {body}
    </button>
  );
}
