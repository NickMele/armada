import { useEffect, useId, useState, type ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";

/**
 * Fleet's four limits, changeable from Bridge — how many drones run at once,
 * the memory and the disk Fleet keeps free before starting another, and how
 * many of a job's checks run at once.
 *
 * **Fleet-wide, and its own section rather than a section on Job settings.**
 * A Job's own settings answer for one Job; these answer for every drone Fleet
 * will ever start. **A plain section, not a layer of its own** — until #1089
 * this drew inside `Sheet`; Settings is a rail screen now, and `Card` on
 * `packages/screens/src/BridgeSettings.tsx` draws the chrome instead.
 */
export type FleetSettingsProps = {
  concurrency: FleetSettingsRow;
  memorySparePercent: FleetSettingsRow;
  diskFloorGib: FleetSettingsRow;
  checksAtOnce: FleetSettingsRow;
  /** Every row is off — the reading is not live, or a save is already out. */
  disabled?: boolean;
  /** Why, said once under the lead rather than left for each row to repeat. */
  disabledNote?: ReactNode;
};

/** One limit: the value in force, what Armada ships, and its own bound. */
export type FleetSettingsRow = {
  /** The value in force, before an unsent edit. */
  value: number;
  /** What Armada ships. The row's own way back to it. */
  shipped: number;
  /** The floor an edit may not cross. */
  min: number;
  /** The ceiling an edit may not cross. */
  max: number;
  /** Trailing the field and the shipped figure — `%`, `GiB`, absent for a count. */
  unit?: string;
  /** Sent on Save, or on the row's own way back to the shipped value. */
  onSave: (value: number) => void;
  /** This row's own press is out. The panel's `disabled` covers every other cause. */
  saving?: boolean;
  /** The line under the row once a save took. */
  said?: ReactNode;
  /** Fleet's own refusal, read exactly — the API's wire message. */
  refused?: string;
};

export function FleetSettings({
  concurrency,
  memorySparePercent,
  diskFloorGib,
  checksAtOnce,
  disabled = false,
  disabledNote,
}: FleetSettingsProps) {
  const group = useId();
  return (
    <div className="armada-fleet-settings">
      <div className="armada-fleet-settings__opening">
        <p className="armada-fleet-settings__lead">
          These apply to every job. A change counts from the next time a job is ready to start,
          and nothing already running stops.
        </p>
        {disabled && disabledNote !== undefined ? (
          <p className="armada-fleet-settings__means">{disabledNote}</p>
        ) : null}
      </div>

      <Row
        id={`${group}-concurrency`}
        label="Drones at once"
        means="How many drones Fleet runs at the same time. A job past this waits, queued, for one to finish."
        row={concurrency}
        disabled={disabled}
      />
      <Row
        id={`${group}-memory`}
        label="Memory to keep free"
        means="The share of memory Fleet leaves free before it starts another drone."
        row={memorySparePercent}
        disabled={disabled}
      />
      <Row
        id={`${group}-disk`}
        label="Disk to keep free"
        means="The disk space Fleet leaves free before it starts another drone."
        row={diskFloorGib}
        disabled={disabled}
      />
      <Row
        id={`${group}-checks`}
        label="Checks at once"
        means="How many of a job's checks Fleet runs at the same time. Some checks take more than one place — a heavy one waits until enough are free. When memory or disk runs short, the next check waits for one to finish."
        row={checksAtOnce}
        disabled={disabled}
      />
    </div>
  );
}

/** `2`, `15%`, `10 GiB` — a count carries no unit, a percentage takes none of its own space. */
function figure(value: number, unit: string | undefined): string {
  if (unit === undefined) return String(value);
  if (unit === "%") return `${value}%`;
  return `${value} ${unit}`;
}

/**
 * One limit's own field. Edited and sent are two different numbers until Save
 * is pressed — `typed` holds what has not gone anywhere yet, so a field does
 * not fire a request a digit at a time.
 */
function Row({
  id,
  label,
  means,
  row,
  disabled,
}: {
  id: string;
  label: string;
  means: string;
  row: FleetSettingsRow;
  disabled: boolean;
}) {
  const [typed, setTyped] = useState<string | null>(null);
  // Which of this row's two sends is out — Save and "Use the shipped value"
  // call the same `onSave`, so this is what tells the pressed control from
  // its sibling once `row.saving` alone cannot. #1117.
  const [pressed, setPressed] = useState<"save" | "shipped" | null>(null);
  useEffect(() => {
    if (row.saving !== true) setPressed(null);
  }, [row.saving]);
  const field = typed ?? String(row.value);
  const asked = Number(field);
  const dirty = field !== String(row.value);
  const inRange = Number.isInteger(asked) && asked >= row.min && asked <= row.max;
  const rangeInvalid = dirty && field !== "" && !inRange;
  const off = disabled || row.saving === true;
  const meansId = `${id}-means`;

  function send(value: number, which: "save" | "shipped"): void {
    setTyped(null);
    setPressed(which);
    row.onSave(value);
  }

  return (
    <div className="armada-fleet-settings__field">
      <label className="armada-fleet-settings__label" htmlFor={id}>
        {label}
      </label>
      <p className="armada-fleet-settings__means" id={meansId}>
        {means}
      </p>
      <div className="armada-fleet-settings__row">
        <Input
          id={id}
          mono
          inputMode="numeric"
          aria-describedby={meansId}
          trailing={row.unit}
          value={field}
          disabled={off}
          invalid={rangeInvalid || row.refused !== undefined}
          message={
            rangeInvalid
              ? `Between ${row.min} and ${row.max}.`
              : row.refused !== undefined && !dirty
                ? row.refused
                : undefined
          }
          onChange={(event) => setTyped(event.target.value)}
        />
        <Button
          variant="secondary"
          size="sm"
          ground="sunken"
          pending={pressed === "save"}
          disabled={off || !dirty || !inRange}
          aria-label={`Save ${label}`}
          onClick={() => send(asked, "save")}
        >
          {pressed === "save" ? "Saving…" : "Save"}
        </Button>
      </div>
      <p className="armada-fleet-settings__ships">
        {`Ships at ${figure(row.shipped, row.unit)}.`}
        {row.value !== row.shipped ? (
          <Button
            variant="ghost"
            size="sm"
            pending={pressed === "shipped"}
            disabled={off}
            aria-label={`Use the shipped value for ${label}`}
            onClick={() => send(row.shipped, "shipped")}
          >
            {pressed === "shipped" ? "Using the shipped value…" : "Use the shipped value"}
          </Button>
        ) : null}
      </p>
      {row.said !== undefined ? (
        <p className="armada-fleet-settings__said" role="status">
          {row.said}
        </p>
      ) : null}
    </div>
  );
}
