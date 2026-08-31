import type { ReactNode } from "react";
import { ActiveJobsList } from "../../compositions/ActiveJobsList/ActiveJobsList";
import {
  JobRowStacked,
  type JobRowStackedProps,
} from "../../compositions/JobRowStacked/JobRowStacked";

/**
 * Journey · Monitor Active Work. The list, in the order Fleet supplies: the
 * rows that need a person first, the rest newest work first.
 *
 * **The screen renders the order it is handed.** Sorting is Fleet's, and a
 * component that re-sorted would be a second definition of the rule — which is
 * `ActiveJobsList`'s own position, restated here only because a screen is where
 * somebody would be tempted to sort.
 *
 * **One row shape at every state.** A row's status, glyph and verb arrive from
 * the generated vocabulary; nothing here writes a status label.
 */
export type TheListSixStatesOneRowShapeProps = {
  /** The surface's name. Lowercase anything countable: "Active jobs". */
  heading?: ReactNode;
  /** The count sentence: "6 jobs. 1 awaiting approval." */
  summary?: ReactNode;
  /** The surface's one primary action, outside the frame. */
  action?: ReactNode;
  /** Where `Board controls` mounts: under the heading, over the frame. */
  controls?: ReactNode;
  /** The rows, in the order they are to be drawn. */
  rows: JobRowStackedProps[];
  /** Where `Board empty state` mounts when there are none. */
  empty?: ReactNode;
  /** True where every row opens a Job. The frame becomes a listbox. */
  selectable?: boolean;
  /** The listbox's name, where it is one. */
  label?: string;
  onCopied?: (value: string) => void;
};

/**
 * The track list a Job at the gate takes: it has no branch, no step and no
 * elapsed yet, so the run is a different field set and carries its own tracks.
 * 168px 72px 108px 100px 128px from the drawing — the workflow, the empty bar,
 * the step, the timestamp it has instead of elapsed, and origin. Exported
 * because the surface drawing real Jobs needs the same tracks, and a second
 * copy is how the two start to differ.
 *
 * **Named properties, not five `calc()` literals.** It was the literals, which
 * is precisely how it missed #218: that change gave `Job row (stacked)` a sixth
 * track and had to call it `--armada-track-provenance`, because
 * `--armada-track-origin` was already spent on the branch-or-workflow track —
 * and nothing here referred to either name, so the fifth width here went on
 * meaning origin by coincidence. The stories' own `GATE_TRACKS`, written in the
 * same change to stand in for this list, had already drifted 24px on the fourth
 * track. Both now read the same properties, declared once in
 * `JobRowStacked.css`.
 */
export const APPROVAL_TRACKS = [
  "var(--armada-track-origin)",
  "var(--armada-track-bar)",
  "var(--armada-track-step)",
  "var(--armada-track-created)",
  "var(--armada-track-provenance)",
].join(" ");

export function TheListSixStatesOneRowShape({
  heading,
  summary,
  action,
  controls,
  rows,
  empty,
  selectable,
  label,
  onCopied,
}: TheListSixStatesOneRowShapeProps) {
  return (
    <ActiveJobsList
      heading={heading}
      summary={summary}
      action={action}
      controls={controls}
      empty={empty}
      selectable={selectable}
      label={label}
    >
      {rows.map((row, index) => (
        <JobRowStacked
          key={row.jobId ?? index}
          {...row}
          onCopied={row.onCopied ?? onCopied}
        />
      ))}
    </ActiveJobsList>
  );
}
