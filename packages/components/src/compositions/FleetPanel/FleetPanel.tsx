import type { ReactNode } from "react";
import { FigureList, type Figure } from "../FigureList/FigureList";
import { Panel } from "../Panel/Panel";

/** Where Bridge's one connection is, collapsed to what a dot can carry. Was `StatusBar`'s. */
export type FleetState = "running" | "not-running" | "unreachable" | "unknown";

/**
 * A rollup of the probes Fleet itself can answer for — never Doctor's own
 * grid, which is unbuilt (`docs/concepts/doctor.md`, #99). Worst of the probes
 * `GET /health` returned, the same reading the status bar was heading toward
 * before this panel replaced it.
 */
export type DoctorLine = {
  outcome: "pass" | "warn" | "fail" | "reading";
  /** The module names it checked, comma-joined — "Fleet, SQLite, Manifest, system stats". */
  checked: string;
};

export type FleetPanelProps = {
  state: FleetState;
  /** "Running", "Not running", "Unreachable", "Reading" — the panel already says "Fleet". */
  label: ReactNode;
  /**
   * `pid`, `port`, `protocol`, `up` — the facts the runtime file and the
   * connection carry, one row each, labels left and values in one aligned
   * column. Settled 2026-09-17, replacing two `·`-joined mono lines.
   *
   * **Only the rows a state has a value for.** A Fleet that is not running has
   * no port to name, and a `port` row with nothing beside it would read as a
   * port that failed to arrive. The caller leaves it out.
   */
  rows?: Figure[];
  /**
   * The sentence a state carries beside or instead of its rows, mono — what
   * the runtime file says, how long an unreachable Fleet has been silent, or
   * the two protocol versions when Fleet is ahead.
   */
  detail?: ReactNode;
  doctor?: DoctorLine;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  narrow?: boolean;
};

const DOT_TONE: Record<FleetState, "success" | "escalated" | "warn" | "muted"> = {
  running: "success",
  "not-running": "escalated",
  unreachable: "warn",
  unknown: "muted",
};

export function FleetPanel({ state, label, rows, detail, doctor, open, onOpenChange, narrow }: FleetPanelProps) {
  return (
    <Panel label="Fleet" open={open} onOpenChange={onOpenChange} narrow={narrow} dotTone={DOT_TONE[state]}>
      <div className="armada-fleet-panel">
        <div className="armada-fleet-panel__state">
          <span className="armada-fleet-panel__dot" data-tone={DOT_TONE[state]} aria-hidden />
          {label}
        </div>
        {rows === undefined || rows.length === 0 ? null : <FigureList figures={rows} column="fit" />}
        {detail === undefined ? null : <div className="armada-fleet-panel__mono">{detail}</div>}
        {doctor === undefined ? null : (
          <div className="armada-fleet-panel__doctor">
            <span className="armada-fleet-panel__dot" data-tone={DOCTOR_TONE[doctor.outcome]} aria-hidden />
            <span>Doctor</span>
            <b className="armada-fleet-panel__doctor-outcome" data-tone={DOCTOR_TONE[doctor.outcome]}>
              {doctor.outcome}
            </b>
            <span className="armada-fleet-panel__meta">{doctor.checked}</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

const DOCTOR_TONE: Record<DoctorLine["outcome"], "success" | "warn" | "escalated" | "muted"> = {
  pass: "success",
  warn: "warn",
  fail: "escalated",
  reading: "muted",
};
