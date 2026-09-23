import { WorkflowStepCard, type WorkflowStepCardProps } from "../WorkflowStepCard/WorkflowStepCard";

/**
 * The same run, stacked — one card per step, top to bottom, a step's groups
 * indented under it and their tasks indented again. The other half of the
 * workflow canvas's toggle (`#1530`, 22 Sep: canvas by default, stacked
 * available, at every width).
 *
 * **The same card as the canvas draws.** A toggle that changed what a step
 * says about itself would be two screens rather than two arrangements.
 *
 * **A second edge is a line of words here**, since a column has nothing to
 * draw an edge with: a group a later step worked says which step that was, and
 * an unworked group says nothing because it has nothing to say yet.
 *
 * **A loop is a line, not an arc**, for the same reason.
 */

export type WorkflowStackedRow = {
  id: string;
  card: WorkflowStepCardProps;
  /** The row this one hangs under — its step, or its group. */
  under?: string;
  /** How deep it is: a group under a step, a task under a group. */
  depth?: 1 | 2;
  /** The step that worked this group, where one has. Absent until one does. */
  worked?: string;
  /** Where this step loops back to, in words, and its cap. */
  returns?: { toName: string; label: string };
};

export type WorkflowStackedProps = {
  /** What the run is, read to somebody who cannot see it. */
  label: string;
  rows: readonly WorkflowStackedRow[];
};

export function WorkflowStacked({ label, rows }: WorkflowStackedProps) {
  return (
    <div className="armada-workflow-stacked">
      <ol className="armada-workflow-stacked__run" aria-label={label}>
        {rows.map((row) => (
          <li className="armada-workflow-stacked__row" data-depth={row.depth} key={row.id}>
            <WorkflowStepCard {...row.card} />
            {row.worked === undefined ? null : (
              <p className="armada-workflow-stacked__worked">worked at {row.worked}</p>
            )}
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
