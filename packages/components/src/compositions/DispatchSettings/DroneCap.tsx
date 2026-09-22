import { Input } from "../../primitives/Input/Input";

import { AUTO } from "./TierModels";

/**
 * How many Drones **this Job** may run at once, beside how many the machine
 * runs across every Job.
 *
 * **Two numbers that mean different things, and the screen says which is
 * which** (#1550). One is a Job's own share; the other is
 * `LimitValues.concurrency`, which is the machine's and is set in Settings.
 * Merging them into one figure is how a person ends up capping the wrong
 * thing.
 */
export type DroneCapProps = {
  /** This Job's cap. Absent is the machine's own cap holding. */
  cap?: number;
  onCap: (cap: number | undefined) => void;
  /** How many this machine runs at once. `null` before Fleet has said. */
  machineCap: number | null;
  disabled?: boolean;
};

export function DroneCap({ cap, onCap, machineCap, disabled = false }: DroneCapProps) {
  return (
    <div className="armada-dispatch-settings__cap">
      <Input
        label="Drones at once"
        type="number"
        min={1}
        {...(machineCap === null ? {} : { max: machineCap })}
        value={cap === undefined ? "" : String(cap)}
        placeholder={AUTO}
        disabled={disabled}
        onChange={(event) =>
          onCap(event.target.value === "" ? undefined : Number(event.target.value))
        }
      />
      {/* The machine's own cap, beside the Job's and never merged into it: one
          is how many Drones this Job may run, the other is how many run here
          at all. */}
      <p className="armada-dispatch-settings__said">
        {machineCap === null
          ? "Fleet has not said how many this machine runs at once."
          : `This machine runs ${machineCap} at once, across every Job.`}
      </p>
    </div>
  );
}
