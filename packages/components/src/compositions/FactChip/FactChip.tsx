import type { ReactNode } from "react";

/**
 * A fact chip — one short machine-derived value, on a raised surface.
 *
 * **A fact is a value, never a sentence.** `not run`, `2 criteria`,
 * `3 files · +94 −31`, `refused · same criterion`. Anything that reads as
 * prose belongs in the panel beside the tree, where there is room for it; that
 * division is the whole reason the tree and the panel are two regions.
 *
 * **A chip, not a table cell.** The run tree carried its facts as plain mono
 * text in a two-column table, which drew a grid the drawing has none of and
 * made a one-word value the width of the column. A chip is the width of its
 * value, so a row of them reads as a row of values.
 *
 * **Mono, and neutral by default.** Everything a chip carries was measured or
 * reported, so it speaks flatly. `named` is the exception and it is per fact,
 * never summed: a refused attempt is red because the refusal is the fact, and
 * a passing Check is green for the same reason.
 *
 * **It truncates, and it keeps its whole value in the title.** A chip is a
 * single line and clips; where the value is a path, use `PathChip` instead —
 * clipping a path from the right is what leaves six rows all reading
 * `packages/settings/src/…`.
 */

/**
 * Which verdict the value is, for its hue. Spelled as the wire spells it, so
 * nothing here is a second vocabulary: `passed`, `failed`, `met`, `not_met`,
 * `advanced`, `refused`, `waiting`.
 *
 * Absent on every chip that is only a measurement — which is most of them, and
 * why neutral is the default rather than a variant.
 */
export type FactChipNamed =
  | "passed"
  | "failed"
  | "met"
  | "not_met"
  | "advanced"
  | "refused"
  | "waiting";

export type FactChipProps = {
  /** The value. One line, mono, clipped where the column is narrower. */
  children: ReactNode;
  named?: FactChipNamed;
  /**
   * The whole value, for the title, where `children` is a string the caller
   * has already shortened. A chip clips and a clipped value with nothing
   * behind it is a value that is gone.
   */
  title?: string;
};

export function FactChip({ children, named, title }: FactChipProps) {
  return (
    <span className="armada-chip" data-named={named} title={title}>
      {children}
    </span>
  );
}
