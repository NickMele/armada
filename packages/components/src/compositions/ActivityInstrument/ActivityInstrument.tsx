import { useId, type CSSProperties } from "react";

/**
 * Activity — tool calls per window on a running Job, one bar per window.
 * `docs/contracts/design-system.md`, under Instruments.
 *
 * **Every mark is a value from the wire.** The windows arrive counted; this
 * draws them against the busiest one, and an empty window still draws, so a
 * Drone gone quiet is a flat tail rather than a shorter chart.
 */
export type ActivityInstrumentProps = {
  /** Calls per window, oldest first. */
  windows: readonly number[];
  /** What the instrument measures, `Tool calls per 30 seconds`. Its accessible name. */
  label: string;
  /** The reading in words, `No tool calls for the last 12 minutes`. */
  description: string;
  /** The oldest edge in words, `12m ago`. */
  from: string;
  /** The newest edge in words, `now`. */
  to: string;
};

/** A bar's share of its window, so neighbours read as separate windows. */
const BAR_SHARE = 0.7;

export function ActivityInstrument({ windows, label, description, from, to }: ActivityInstrumentProps) {
  const labelId = useId();
  const descriptionId = useId();
  const peak = windows.reduce((most, count) => Math.max(most, count), 0);
  const slot = windows.length === 0 ? 0 : 100 / windows.length;
  return (
    <div
      className="armada-instrument-activity"
      role="img"
      aria-labelledby={labelId}
      aria-describedby={descriptionId}
    >
      <div className="armada-instrument-activity__head">
        <span id={labelId}>{label}</span>
        {peak === 0 ? null : <span>peak {peak}</span>}
      </div>
      <svg className="armada-instrument-activity__plot" aria-hidden>
        {windows.map((count, index) => (
          <rect
            // Windows never reorder; a new reading replaces the drawing.
            key={index}
            className={count === 0 ? "armada-instrument-activity__empty" : "armada-instrument-activity__bar"}
            x={`${slot * index + (slot * (1 - BAR_SHARE)) / 2}%`}
            width={`${slot * BAR_SHARE}%`}
            style={{ "--armada-activity-share": peak === 0 ? 0 : count / peak } as CSSProperties}
          />
        ))}
        <rect className="armada-instrument-activity__axis" x="0" width="100%" />
      </svg>
      <div className="armada-instrument-activity__foot">
        <span>{from}</span>
        <span>{to}</span>
      </div>
      <span id={descriptionId} className="armada-instrument-activity__description">
        {description}
      </span>
    </div>
  );
}
