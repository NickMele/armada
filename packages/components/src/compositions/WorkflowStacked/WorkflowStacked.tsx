import type { ReactNode } from "react";

import { WorkflowStepCard, type WorkflowStepCardProps } from "../WorkflowStepCard/WorkflowStepCard";

/**
 * The same run, stacked — one card per step, top to bottom, a step's groups
 * indented under it. The other half of the workflow canvas's toggle
 * (`#1530`, 22 Sep: canvas by default, stacked available, at every width).
 *
 * **The same card as the canvas draws.** A toggle that changed what a step
 * says about itself would be two screens rather than two arrangements.
 *
 * **A loop is a line, not an arc.** There is nothing to arc over in a column,
 * so the step that loops says where it goes back to and what its cap is —
 * which is `WorkflowDiagram`'s own answer to the same problem.
 */

export type WorkflowStackedRow = {
  id: string;
  card: WorkflowStepCardProps;
  /** The step this row hangs under, where it is a group. */
  under?: string;
  /** Where this step loops back to, in words, and its cap. */
  returns?: { toName: string; label: string };
};

export type WorkflowStackedProps = {
  /** What the run is, read to somebody who cannot see it. */
  label: string;
  rows: readonly WorkflowStackedRow[];
  /** Drawn above the column — the canvas/stacked toggle. */
  aside?: ReactNode;
};

export function WorkflowStacked({ label, rows, aside }: WorkflowStackedProps) {
  return (
    <div className="armada-workflow-stacked">
      {aside === undefined ? null : <div className="armada-workflow-stacked__aside">{aside}</div>}
      <ol className="armada-workflow-stacked__run" aria-label={label}>
        {rows.map((row) => (
          <li className="armada-workflow-stacked__row" data-under={row.under === undefined ? undefined : ""} key={row.id}>
            <WorkflowStepCard {...row.card} />
            {row.returns === undefined ? null : (
              <p className="armada-workflow-stacked__returns">
                back to {row.returns.toName} · {row.returns.label}
              </p>
            )}
          </li>
        ))}
      </ol>
    </div>
  );
}
