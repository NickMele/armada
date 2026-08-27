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
 * 168px 72px 108px 100px 128px from the drawing, composed off the spacing scale
 * rather than written as literals. Exported because the surface drawing real
 * Jobs needs the same tracks, and a second copy is how the two start to differ.
 */
export const APPROVAL_TRACKS = [
  "calc(var(--space-12) * 3 + var(--space-6))",
  "calc(var(--space-12) + var(--space-6))",
  "calc(var(--space-12) * 2 + var(--space-3))",
  "calc(var(--space-12) * 2 + var(--space-1))",
  "calc(var(--space-12) * 2 + var(--space-8))",
].join(" ");

export function TheListSixStatesOneRowShape({
  heading,
  summary,
  action,
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
