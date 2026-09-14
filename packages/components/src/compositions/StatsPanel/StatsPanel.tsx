import type { ReactNode } from "react";
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
  hint?: string;
};

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
  return (
    <>
      <dt className="armada-stats-panel__label">{row.label}</dt>
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
