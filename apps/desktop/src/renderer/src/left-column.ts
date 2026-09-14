// The left column's Stats and Fleet panels, built here rather than in
// `@armada/shell` — that package is what `@armada/screens` imports
// `statementOf` from, so the reverse import would be a dependency cycle.
// Reuses Overview's own tile arithmetic rather than re-deriving it, so the
// two readings cannot drift apart. Bridge/1088.

import type { StatRow, FleetPanelProps } from "@armada/components";
import type { Connection, FleetCapacity, JobSummary, RepositorySummary } from "@armada/protocol";
import { fleetStateOf, shortLabelOf, type Statement } from "@armada/shell";
import { doctorReading, driftReading, dronesReading } from "@armada/screens/src/overview";
import type { DriftsRead, HealthRead } from "@armada/screens/src/overview-reads";

/** Stats — the six rows Overview's own tiles already read. No Queued: Overview's own Queued panel lists those Jobs. */
export function statsOf(
  connection: Connection,
  jobs: readonly JobSummary[],
  capacity: FleetCapacity | null,
  repositories: readonly RepositorySummary[],
  scope: string | null,
  drifts: DriftsRead,
): StatRow[] {
  const picked = repositories.find((repository) => repository.root === scope) ?? null;
  const queued = jobs.filter((job) => job.status === "queued").length;
  const drone = dronesReading(connection, capacity, queued);
  const drift = driftReading(drifts, picked);
  return [
    countRow("approval", "Awaiting approval", jobs.filter((job) => job.status === "awaiting_approval").length),
    countRow("review", "Needs review", jobs.filter((job) => job.status === "awaiting_review").length),
    countRow("escalated", "Escalated", jobs.filter((job) => job.status === "escalated").length, "hot"),
    { id: "jobs", label: "Jobs", value: jobs.length },
    { id: "drones", label: "Drones", value: drone.value ?? "—", hint: asString(drone.detail) },
    {
      id: "manifest",
      label: "Manifest",
      value: drift.value ?? "—",
      tone: toneOf(drift.tone),
      hint: asString(drift.detail),
    },
  ];
}

/** A count row, amber past zero — Awaiting approval, Needs review and Escalated share the treatment. */
function countRow(id: string, label: string, value: number, tone: StatRow["tone"] = "warn"): StatRow {
  return { id, label, value, ...(value > 0 ? { tone } : {}) };
}

/** `OverviewTileTone` collapsed to the Stats panel's warn/hot pair. */
function toneOf(tone: string | undefined): StatRow["tone"] {
  if (tone === "completed-failed") return "hot";
  if (tone === "awaiting-review" || tone === "notice-caution") return "warn";
  return undefined;
}

/** Every reader `statsOf` calls returns a plain string `detail`; anything else is dropped rather than stringified blind. */
function asString(node: unknown): string | undefined {
  return typeof node === "string" ? node : undefined;
}

const DOCTOR_OUTCOMES = new Set(["pass", "warn", "fail"]);

/**
 * Fleet — Running, pid · port, and Doctor as a rollup of what `GET /health`
 * answered. **Not Doctor's own grid**, which is unbuilt (#99): the worst of
 * the probes Fleet itself can run, the same reading Overview's own Doctor
 * tile already computes.
 */
export function fleetPanelOf(
  connection: Connection,
  statement: Statement,
  health: HealthRead,
): Omit<FleetPanelProps, "open" | "onOpenChange"> {
  const reading = doctorReading(health);
  const outcome = asString(reading.value);
  const doctor =
    outcome === undefined || !DOCTOR_OUTCOMES.has(outcome)
      ? undefined
      : { outcome: outcome as "pass" | "warn" | "fail", checked: asString(reading.detail) ?? "" };
  return {
    state: fleetStateOf(connection),
    label: shortLabelOf(connection),
    detail: statement.detail === "" ? undefined : statement.detail,
    doctor,
  };
}
