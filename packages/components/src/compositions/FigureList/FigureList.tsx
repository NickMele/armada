/**
 * Labels on the left, their figures justified to the list's right edge where
 * the Stats panel's counts sit — the key/value rows Pulse draws under a Job's
 * run, and the Fleet panel draws under its state.
 *
 * **One treatment, not two.** This was `JobHoldsSummary`'s own `Figure`, and a
 * second key/value style would drift from it the day either changed. So the
 * right edge is both callers' or neither's — Pulse's top line included, which
 * is the whole of what `words` and `detail` are for — at the accepted cost
 * that *Where things are*, not a `FigureList`, keeps its left-aligned values.
 *
 * **A figure with no value is not a row.** The caller leaves it out of the
 * list, because a label beside a blank reads the same whether the value is
 * nothing or never arrived.
 */

export type Figure = {
  /** Sans, `--text-label` — `Processes`, `pid`, `Drone`. */
  label: string;
  /** Mono, clipped from the right — `None`, `4242`, `171h 00m`. */
  value: string;
  /**
   * A second line under the value, ending on the same right edge — the instant
   * Pulse's top line was said. Mono `--text-2xs` `--fg-subtle`, the Fleet
   * panel's detail line at the width of one row.
   *
   * **It belongs to the value, so it goes where the value goes.** A time left
   * under a right-aligned phrase reads as a third column that lines up with
   * nothing.
   */
  detail?: string;
  /**
   * Whether the value is words somebody said rather than a figure — sans, and
   * held to two lines instead of clipped to one.
   *
   * **Pulse's top line is a sentence, and a sentence is not a reading.** It
   * carries a Drone's `Edit packages/settings/src/selectors.ts` as readily as
   * `thinking`, which is a turn to read in the log; mono and one line would
   * make it a figure that happens to be long.
   */
  words?: boolean;
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
 *
 * **`strip` is not a column at all** — the label sits over its figure and the
 * figures run across, wrapping as the width allows. A panel column is 160 to
 * 380px wide and a destination is the width of the window: the same rows
 * pushed to a 1000px right edge put a label and its figure two feet apart,
 * which is a pair nobody reads as a pair. #1538.
 */
export type FigureColumn = "wide" | "fit" | "strip";

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
          {/* The value is a box of its own lines rather than the text itself,
              because a detail under it has to clip on its own terms — and a
              block dropped into a clipping `dd` takes none of its clipping. */}
          <dd className="armada-figures__value" data-words={figure.words || undefined}>
            <span className="armada-figures__reading">{figure.value}</span>
            {figure.detail === undefined ? null : <span className="armada-figures__detail">{figure.detail}</span>}
          </dd>
        </div>
      ))}
    </dl>
  );
}
