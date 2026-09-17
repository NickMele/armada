// The left column's Stats and Fleet panels, built here rather than in
// `@armada/shell` — that package is what `@armada/screens` imports
// `statementOf` from, so the reverse import would be a dependency cycle.
// Reuses Overview's own tile arithmetic rather than re-deriving it, so the
// two readings cannot drift apart. Bridge/1088.

import type { StatRow, FleetPanelProps } from "@armada/components";
import type { Connection, FleetCapacity, JobSummary, RepositorySummary } from "@armada/protocol";
import { spoken } from "@armada/protocol";
import { fleetStateOf, shortLabelOf, type Statement } from "@armada/shell";
import { doctorReading, driftReading, dronesReading } from "@armada/screens/src/overview";
import type { DriftsRead, HealthRead } from "@armada/screens/src/overview-reads";
// `instant` and `lasting` are the Job elapsed-time figure's own parse-and-format
// pair (`elapsedSince` above them). Reused rather than re-derived so a job's
// "1h 30m" and Fleet's "up 1h 30m" cannot drift into two spellings of one span.
import { instant, lasting } from "@armada/screens/src/duration";

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
  // Only `driftReading`'s behind reading carries `notice-caution`; Current,
  // Not read and Not set up all keep the current hue, dim.
  const behind = drift.tone === "notice-caution";
  return [
    countRow("approval", "Awaiting approval", jobs.filter((job) => job.status === "awaiting_approval").length),
    countRow("review", "Needs review", jobs.filter((job) => job.status === "awaiting_review").length),
    countRow("escalated", "Escalated", jobs.filter((job) => job.status === "escalated").length, "hot"),
    { id: "jobs", label: "Jobs", value: jobs.length, hue: "status-not-started", idle: jobs.length === 0 },
    {
      id: "drones",
      label: "Drones",
      value: drone.value ?? "—",
      hint: asString(drone.detail),
      hue: "stat-drones",
      idle: (capacity?.occupied ?? 0) === 0,
    },
    {
      id: "manifest",
      label: "Manifest",
      value: drift.value ?? "—",
      tone: toneOf(drift.tone),
      hint: asString(drift.detail),
      hue: behind ? "notice-caution" : "stat-manifest-current",
      idle: !behind,
    },
  ];
}

/**
 * A count row, amber past zero — Awaiting approval, Needs review and Escalated
 * share the treatment. The dot's hue is the value's own loud colour, dim at zero.
 */
function countRow(id: string, label: string, value: number, tone: "warn" | "hot" = "warn"): StatRow {
  const hue = tone === "hot" ? "status-escalated" : "status-awaiting-review";
  return { id, label, value, hue, idle: value === 0, ...(value > 0 ? { tone } : {}) };
}

/** `ReadingTone` collapsed to the Stats panel's warn/hot pair. */
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
  now: number,
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
    meta: metaOf(connection, now),
    doctor,
  };
}

/**
 * `protocol 13.49 · up 2h 14m` — only once a connection has one to read.
 *
 * **Ticks off the app's one `now`** (`App.tsx`'s `setInterval`, already
 * running for every other elapsed figure on screen) rather than a second
 * clock. The uptime is relative to `FleetIdentity.startedAt` — the instant
 * `ps -o lstart=` gave Fleet's own process, the spelling `crates/fleet`
 * chose specifically so Bridge could parse it too — not a static read taken
 * once at connect time.
 */
function metaOf(connection: Connection, now: number): string | undefined {
  if (connection.state !== "connected") return undefined;
  const protocol = `protocol ${spoken(connection.fleet.protocolVersion)}`;
  const startedMs = instant(connection.fleet.startedAt);
  return startedMs === null ? protocol : `${protocol} · up ${lasting(now - startedMs)}`;
}
