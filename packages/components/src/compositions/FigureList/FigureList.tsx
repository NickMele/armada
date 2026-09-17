/**
 * Labels on the left, their figures in one aligned column on the right — the
 * key/value rows Pulse draws under a Job's run, and the Fleet panel draws under
 * its state.
 *
 * **One treatment, not two.** This was `JobHoldsSummary`'s own `Figure`. The
 * owner settled the Fleet panel on 2026-09-17 as "like Pulse's Processes /
 * Worktree rows", and a second key/value style written for it would drift from
 * the first the day either changed.
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
 * `wide` is two `--space-12`, the column *Where things are* draws, so Pulse
 * and the rows under it read as one list. `fit` sizes the column to the
 * longest label, for a list with no neighbour to line up with and no width to
 * spare — the left column's Fleet panel is 160px at its narrowest.
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
