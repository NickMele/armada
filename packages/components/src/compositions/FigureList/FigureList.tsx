/**
 * Labels on the left, their figures justified to the list's right edge where
 * the Stats panel's counts sit — the key/value rows Pulse draws under a Job's
 * run, and the Fleet panel draws under its state.
 *
 * **One treatment, not two.** This was `JobHoldsSummary`'s own `Figure`, and a
 * second key/value style would drift from it the day either changed. So the
 * right edge is both callers' or neither's, at the accepted cost that *Where
 * things are* — not a `FigureList` — keeps its left-aligned values.
 *
 * **A figure with no value is not a row.** The caller leaves it out of the
 * list, because a label beside a blank reads the same whether the value is
 * nothing or never arrived.
 */

export type Figure = {
  /** Sans, `--text-label` — `Processes`, `pid`. */
  label: string;
  /** Mono, clipped from the right — `None`, `4242`, `171h 00m`. */
  value: string;
  /** Whether the value is a fault. Draws it in `--error`. */
  wrong?: boolean;
};

/**
 * How wide the label column is.
 *
 * Since the values went to the right edge this sets where a value *starts* —
 * how much room it has before it clips — not where it sits.
 *
 * `wide` is two `--space-12`, the column *Where things are* draws, so its
 * labels and Pulse's start on one line. `fit` sizes the column to the longest
 * label, leaving the value every pixel left over: the left column's Fleet
 * panel is 160px at its narrowest, and `171h 55m` needs them.
 */
export type FigureColumn = "wide" | "fit";

export type FigureListProps = {
  figures: Figure[];
  column?: FigureColumn;
};

export function FigureList({ figures, column = "wide" }: FigureListProps) {
  return (
    <dl className="armada-figures" data-column={column}>
      {figures.map((figure) => (
        <div key={figure.label} className="armada-figures__row" data-wrong={figure.wrong || undefined}>
          <dt className="armada-figures__label">{figure.label}</dt>
          <dd className="armada-figures__value">{figure.value}</dd>
        </div>
      ))}
    </dl>
  );
}
