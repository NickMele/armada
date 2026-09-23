// Fleet's four limits, and this machine's own settings, on one screen. #1089
// — the sheet these limits used to live behind opened from the status bar,
// gone since #1088, and had no rail row to open it from once it was.
//
// **`BridgeSettings`, not `Settings`.** `./settings.tsx` is a Job's own —
// `SettingsSheet`, its caps and its choices — and the two would collide by
// name and, on a case-insensitive filesystem, by file.
//
// A limit changed here takes the same way it always has: `FleetSettings`
// beneath is unchanged but for the layer, which is `Card` now instead of
// `Sheet`.

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  FleetSettings,
  GuidesSetting,
  MachineSettings,
  type FleetSettingsRow,
} from "@armada/components";
import type { FleetLimits, HelmActionAuthority, Outcome, SaveLimits } from "@armada/protocol";
import { useState } from "react";
import type { HealthRead } from "./overview-reads";

/** One of the four fields a row may send, by its wire name. */
type Field = keyof SaveLimits;

const ROWS: readonly { field: Field; min: number; max: number; unit?: string }[] = [
  { field: "concurrency", min: 1, max: 8 },
  { field: "memory_spare_percent", min: 0, max: 50, unit: "%" },
  { field: "disk_floor_gib", min: 0, max: 100, unit: "GiB" },
  { field: "checks_at_once", min: 1, max: 8 },
];

/** Said under a row once its own save takes. */
const TOOK = "Changed. Applies the next time a job is ready to start.";

export type BridgeSettingsProps = {
  /** `null` where Fleet has not answered yet — every row is off until it has. */
  limits: FleetLimits | null;
  /** A live connection. Off with nothing to send to. */
  live: boolean;
  /** `GET /health`, `#1127`'s reason: what Fleet resolved `this machine`'s own settings to. */
  health: HealthRead;
  onSave: (values: SaveLimits) => Promise<Outcome>;
  /** Opens the guide catalogue. Navigation is the window's, so it arrives as a prop. */
  onReadGuides?: () => void;
};

/** "Acting"/"Read-only", as `this machine` reads it — or absent, before `health` has answered. */
export function helmActionAuthorityValue(health: HealthRead): string | undefined {
  if (health.state !== "read") return undefined;
  return WORDS[health.health.helm_action_authority];
}

const WORDS: Record<HelmActionAuthority, string> = { acting: "Acting", read_only: "Read-only" };

export function BridgeSettings({ limits, live, health, onSave, onReadGuides }: BridgeSettingsProps) {
  return (
    <div className="armada-screen__pane">
      <Card>
        <CardHeader>
          <CardTitle>Fleet</CardTitle>
        </CardHeader>
        <CardContent>
          {limits === null ? (
            <p>Fleet has not answered yet.</p>
          ) : (
            <FleetLimitsFields limits={limits} live={live} onSave={onSave} />
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>This machine</CardTitle>
        </CardHeader>
        <CardContent>
          <MachineSettings
            rows={[
              {
                label: "Helm action authority",
                value: helmActionAuthorityValue(health),
                means:
                  "Whether Helm may act on a Tier 1 Redirect on this machine, or only read one and suggest it. Fleet resolves this once, when it starts, and nothing here changes it while Fleet runs.",
              },
            ]}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Guides</CardTitle>
        </CardHeader>
        <CardContent>
          <GuidesSetting {...(onReadGuides === undefined ? {} : { onReadGuides })} />
        </CardContent>
      </Card>
    </div>
  );
}

/**
 * The four fields, once Fleet has answered. **One save in flight for the
 * whole section**, on Job settings' terms: the four rows are one act's worth
 * of controls and a second set here could only ever refuse a press this never
 * sends.
 */
function FleetLimitsFields({
  limits,
  live,
  onSave,
}: {
  limits: FleetLimits;
  live: boolean;
  onSave: (values: SaveLimits) => Promise<Outcome>;
}) {
  const [saving, setSaving] = useState<Field | null>(null);
  const [said, setSaid] = useState<Partial<Record<Field, boolean>>>({});
  const [refused, setRefused] = useState<Partial<Record<Field, string>>>({});

  function send(field: Field, value: number): void {
    setSaving(field);
    setSaid((was) => ({ ...was, [field]: false }));
    setRefused((was) => ({ ...was, [field]: undefined }));
    void onSave({ [field]: value }).then((outcome) => {
      setSaving(null);
      if (outcome.ok) setSaid((was) => ({ ...was, [field]: true }));
      else if (!outcome.ok && outcome.why === "refused") {
        setRefused((was) => ({ ...was, [field]: outcome.error.message }));
      }
    });
  }

  const off = !live || saving !== null;
  const rowOf = (field: Field, unit: string | undefined, min: number, max: number): FleetSettingsRow => ({
    value: limits[field],
    shipped: limits.shipped[field],
    min,
    max,
    unit,
    saving: saving === field,
    said: said[field] === true ? TOOK : undefined,
    refused: refused[field],
    onSave: (value) => send(field, value),
  });

  const [concurrency, memorySparePercent, diskFloorGib, checksAtOnce] = ROWS.map((row) =>
    rowOf(row.field, row.unit, row.min, row.max),
  ) as [FleetSettingsRow, FleetSettingsRow, FleetSettingsRow, FleetSettingsRow];

  return (
    <FleetSettings
      disabled={off}
      disabledNote={
        !live
          ? "Fleet is not connected, so nothing can be changed."
          : saving !== null
            ? "Something sent to Fleet is still on its way."
            : undefined
      }
      concurrency={concurrency}
      memorySparePercent={memorySparePercent}
      diskFloorGib={diskFloorGib}
      checksAtOnce={checksAtOnce}
    />
  );
}
