import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * Step bar — progress on a list row, as a bar rather than a fraction.
 *
 * "4 of 7" has to be read and converted; a bar is read at a glance, and read
 * down a column it shows which Jobs are near the end. One segment per step, so
 * the segment width itself says how long the workflow is. The exact count moves
 * to the tooltip.
 *
 * **The bar never pulses.** Its job is where the work got to, which is a static
 * fact. On a list row the badge carries the pulse, because it sits in the same
 * fixed column on every row and the motion then appears in one predictable
 * place rather than moving with the workflow's length.
 *
 * **A step behind the current one keeps its hue**, `--step-advanced`, the same
 * as in a rail. The bar used to draw them neutral on the argument that a list
 * of Jobs would be a wall of green; the owner ruled the other way on 11 Sep
 * 2026, because a grey run of finished steps read as steps that had not
 * happened.
 */
export type StepBarProps = {
  /** One segment per step. Segment width is what says how long a workflow is. */
  total: number;
  /**
   * The 1-based position of the current step. `0` for a Job that has not
   * started: every segment is remaining and none takes hue.
   */
  current: number;
  /**
   * The current step's activity, and the hue its segment takes. The steps
   * behind it take `advanced`'s. `killed` and `retrying` take none, as
   * everywhere else.
   */
  activity?: StepActivity;
  /**
   * The exact count, as the tooltip carries it. Written by the caller, because
   * "Step 4 of 7" is a sentence and this component composes none.
   */
  label?: string;
};

export function StepBar({ total, current, activity = "not_started", label }: StepBarProps) {
  const segments = Array.from({ length: total }, (_, i) => {
    const position = i + 1;
    if (position < current) return "past" as const;
    if (position === current) return "current" as const;
    return "remaining" as const;
  });

  const bar = (
    <span
      className="armada-step-bar"
      role="img"
      aria-label={label ?? `Step ${current} of ${total}`}
    >
      {segments.map((state, i) => (
        <span
          key={i}
          className="armada-step-bar__segment"
          data-state={state}
          data-activity={state === "current" ? activity : undefined}
        />
      ))}
    </span>
  );

  // The exact count lives in the tooltip, which is why the bar carries no
  // number of its own.
  return label ? <Tooltip label={label}>{bar}</Tooltip> : bar;
}
