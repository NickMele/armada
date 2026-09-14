import type { ReactNode } from "react";
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
  /** `pid 61372 · port 40000`, mono. */
  detail?: ReactNode;
  /** `protocol 13.49 · up 2h 14m`, mono. */
  meta?: ReactNode;
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

export function FleetPanel({ state, label, detail, meta, doctor, open, onOpenChange, narrow }: FleetPanelProps) {
  return (
    <Panel label="Fleet" open={open} onOpenChange={onOpenChange} narrow={narrow} dotTone={DOT_TONE[state]}>
      <div className="armada-fleet-panel">
        <div className="armada-fleet-panel__state">
          <span className="armada-fleet-panel__dot" data-tone={DOT_TONE[state]} aria-hidden />
          {label}
        </div>
        {detail === undefined ? null : <div className="armada-fleet-panel__mono">{detail}</div>}
        {meta === undefined ? null : <div className="armada-fleet-panel__meta">{meta}</div>}
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
