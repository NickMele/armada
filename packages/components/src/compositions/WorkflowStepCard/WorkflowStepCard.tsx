import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * One node of a Job's workflow — a step, a group inside one, or a task inside
 * a group. **The same card on the canvas and in the stacked run**, so a toggle
 * between the two changes the arrangement and never what a step says about
 * itself.
 *
 * **Status colour is not chosen here.** The mark is `StepActivityMark`, whose
 * hue maps onto the step machine, and a chip takes a verdict's hue only where
 * the caller names one. A group's own state (`joining`, `checking`) has no
 * mark of its own, so the caller maps it to the nearest activity and hands the
 * group's word through `said` — nothing is lost and nothing is invented.
 */

/** One short fact under the name. A value, never a sentence. */
export type WorkflowStepFact = {
  value: string;
  named?: FactChipNamed;
};

export type WorkflowStepCardProps = {
  /** A step of the workflow, a group inside one, or a task inside a group. */
  kind: "step" | "group" | "task";
  /** The step's label, the group's name, or the task's title. */
  name: string;
  /** Whether `name` is a `step_id` rather than a label, so it renders in mono. */
  nameIsAnIdentifier?: boolean;
  activity: StepActivity;
  /**
   * The state in words — the registry's verb for a step, the group machine's
   * own value for a group. Read to somebody who cannot see the mark.
   */
  said: string;
  /** Its position in the run, counted from one. Stands in for a glyph on a step nothing entered. */
  ordinal?: number;
  facts?: readonly WorkflowStepFact[];
  /** The step the Job is on. Its mark pulses and the card takes the accent edge. */
  current?: boolean;
  /** Open in the inspector. */
  selected?: boolean;
  /** The gate's own word, where the step waits for a person. Absent on `auto`. */
  gate?: string;
  /** Opens it in the inspector. Absent draws a card that is not a control. */
  onOpen?: () => void;
};

export function WorkflowStepCard({
  kind,
  name,
  nameIsAnIdentifier,
  activity,
  said,
  ordinal,
  facts = [],
  current = false,
  selected = false,
  gate,
  onOpen,
}: WorkflowStepCardProps) {
  const body = (
    <>
      <span className="armada-wf-card__head">
        <StepActivityMark
          activity={activity}
          label={said}
          ordinal={ordinal}
          pulsing={current}
          says={`${name}, ${said}`}
        />
        <span className="armada-wf-card__name" data-identifier={nameIsAnIdentifier || undefined}>
          {name}
        </span>
      </span>
      {facts.length === 0 ? null : (
        <span className="armada-wf-card__facts">
          {facts.map((fact) => (
            <FactChip key={fact.value} named={fact.named}>
              {fact.value}
            </FactChip>
          ))}
        </span>
      )}
      {gate === undefined ? null : <span className="armada-wf-card__gate">{gate}</span>}
    </>
  );

  const attributes = {
    className: "armada-wf-card",
    "data-kind": kind,
    "data-current": current || undefined,
  };

  return onOpen === undefined ? (
    <span {...attributes} role="group" aria-label={`${name}, ${said}`}>
      {body}
    </span>
  ) : (
    <button
      {...attributes}
      type="button"
      aria-current={selected ? "true" : undefined}
      aria-label={`${name}, ${said}`}
      onClick={onOpen}
    >
      {body}
    </button>
  );
}
