import type { CSSProperties, ReactNode } from "react";
import { Panel } from "../Panel/Panel";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Stats — the left column's second panel, Bridge/1088. What used to be the
 * status bar's two counts, plus what Overview's own tiles already read for
 * drones and Manifest drift, gathered where the rail's crowded top used to
 * carry them.
 *
 * **No Queued.** Overview's own Queued panel already lists those Jobs; a
 * count here would be a second place to check the same fact.
 */
export type StatRow = {
  id: string;
  label: string;
  /** A number, or `"2 of 4"` — Drones and Manifest read as pairs, not counts. */
  value: ReactNode;
  tone?: "warn" | "hot";
  /**
   * The row's dot, as the stem of the token it draws in: `status-escalated`
   * draws `--status-escalated`. **The caller picks it**, so this component
   * holds no row-to-status table. design-system.md → Stats panel.
   */
  hue: StatHue;
  /** Nothing counted or nothing running: the dot mixes into the ground at `--dot-idle`. */
  idle: boolean;
  hint?: string;
};

/** Every token a Stats row's dot may take, and no other. */
export type StatHue =
  | "status-awaiting-review"
  | "status-escalated"
  | "status-not-started"
  | "stat-drones"
  | "stat-manifest-current"
  | "notice-caution";

export type StatsPanelProps = {
  rows: StatRow[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  narrow?: boolean;
};

export function StatsPanel({ rows, open, onOpenChange, narrow }: StatsPanelProps) {
  const dotTone = rows.some((row) => row.tone === "hot")
    ? "escalated"
    : rows.some((row) => row.tone === "warn")
      ? "warn"
      : "muted";

  return (
    <Panel label="Stats" open={open} onOpenChange={onOpenChange} narrow={narrow} dotTone={dotTone}>
      <dl className="armada-stats-panel">
        {rows.map((row) => (
          <StatRowLine key={row.id} row={row} />
        ))}
      </dl>
    </Panel>
  );
}

function StatRowLine({ row }: { row: StatRow }) {
  const value = (
    <dd className="armada-stats-panel__value" data-tone={row.tone}>
      {row.value}
    </dd>
  );
  // A mark, not a glyph: Iconography keeps this panel text-only, and the value
  // beside it already says in words what the dot's strength says in colour.
  const hue = { "--armada-stat-hue": `var(--${row.hue})` } as CSSProperties;
  return (
    <>
      <dt className="armada-stats-panel__label">
        <span className="armada-stats-panel__dot" aria-hidden="true" style={hue} data-idle={row.idle || undefined} />
        {row.label}
      </dt>
      {row.hint === undefined ? (
        value
      ) : (
        <Tooltip asChild label={row.hint}>
          {value}
        </Tooltip>
      )}
    </>
  );
}
