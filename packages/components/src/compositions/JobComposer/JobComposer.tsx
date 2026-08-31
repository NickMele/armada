import type { ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { Card, CardFooter } from "../../primitives/Card/Card";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Textarea } from "../../primitives/Textarea/Textarea";

/**
 * Job composer — what M1 renders where the approval card goes.
 *
 * **Same card, same order, same button, one field set smaller.** The full
 * approval card exists in the design and is not built at this milestone: it
 * turns on a diff size, a job type and an estimated cost, none of which
 * Armada can measure before a drone has run. The composer keeps the card's
 * shape and drops what M1 cannot fill.
 *
 * **The glance strip survives with the two values M1 can measure.** How long
 * the workflow is, and which Checks gate it. A card whose whole design is a
 * forced glance cannot ship with nothing to glance at, so the strip narrows
 * from three fields to two rather than disappearing.
 *
 * **`Approve and dispatch` is the one accent fill in the milestone.** It is
 * the only place a person commits Armada to spend, and every other action in
 * Bridge is secondary or ghost so that this one reads as the decision.
 *
 * **`Approve` lands the job in `queued`, not `running`** — a drone spawning is
 * what starts it, and Fleet runs a bounded number of them at once, so an
 * approved job waits where the bound is reached. **`Cancel` writes `killed`**: a
 * job you never dispatched was not stopped, it was abandoned, and `rejected` is
 * a verdict exit that M1 does not have. The copy names what the person is
 * doing; the record names what happened.
 *
 * The field labels are the drawing's: `Title`, `Brief`, `Workflow`, `Project`.
 * None opens with a Wh- word.
 */
export type JobComposerGlance = {
  /** The field name, in sans and set back. */
  label: ReactNode;
  /** The value, in mono: every one of them is measured off the workflow. */
  value: ReactNode;
};

export type JobComposerProps = {
  /** The Job's title. A person's own words, so it is sans. */
  title?: string;
  /** The brief. Prose written at length, which is why the field is a textarea. */
  brief?: string;
  /**
   * The workflow options, as `option` elements. The label carries the step
   * count — `bug — 4 steps` — because the choice is between workflows of
   * different lengths.
   */
  workflows: ReactNode;
  /**
   * The project, read-only. M1 dispatches into the workspace Bridge is already
   * pointed at, so this is a fact rather than a choice — and a disabled select
   * would be a control that looks choosable and is not.
   */
  project: ReactNode;
  /** The glance strip: what can be known about the run before it starts. */
  glance: JobComposerGlance[];
  /** Where the job came from. Sans and set back — provenance is not a status. */
  provenance?: ReactNode;
  onCancel?: () => void;
  onDispatch?: () => void;
};

export function JobComposer({
  title,
  brief,
  workflows,
  project,
  glance,
  provenance,
  onCancel,
  onDispatch,
}: JobComposerProps) {
  return (
    <Card className="armada-job-composer">
      <Input label="Title" defaultValue={title} />
      <Textarea label="Brief" defaultValue={brief} />

      <div className="armada-job-composer__pair">
        <Select label="Workflow">{workflows}</Select>
        {/* Read-only, so it is a labelled value rather than a field. It keeps
            the control height so its baseline lines up with the select
            beside it. */}
        <div className="armada-job-composer__readonly">
          <span className="armada-job-composer__label">Project</span>
          <span className="armada-job-composer__static">{project}</span>
        </div>
      </div>

      <div className="armada-job-composer__glance">
        {glance.map((field, i) => (
          <div className="armada-job-composer__field" key={i}>
            <span className="armada-job-composer__field-label">{field.label}</span>
            <span className="armada-job-composer__field-value">{field.value}</span>
          </div>
        ))}
      </div>

      <CardFooter className="armada-job-composer__foot">
        {provenance ? (
          <span className="armada-job-composer__provenance">{provenance}</span>
        ) : null}
        <div className="armada-job-composer__actions">
          <Button onClick={onCancel}>Cancel</Button>
          <Button variant="primary" onClick={onDispatch}>
            Approve and dispatch
          </Button>
        </div>
      </CardFooter>
    </Card>
  );
}
