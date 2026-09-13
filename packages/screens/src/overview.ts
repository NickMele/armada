// What each of Overview's tiles reads, from what Fleet already serves. `OverviewTiles.tsx` draws
// them; this decides them, so the arithmetic is tested as arithmetic.
//
// **Fleet, Doctor and Drones read the machine; Queued and drift read the scope.** A pick narrows
// the last two and never the first three — `docs/concepts/fleet.md` names what stays Fleet-wide.

import { ADMISSION_HOLD } from "@armada/components";
import type { OverviewTileProps, OverviewTileTone } from "@armada/components";
import type { Connection, FleetCapacity, JobSummary, ManifestDrift, RepositorySummary } from "@armada/protocol";
import { statementOf } from "@armada/shell/src/fleet";
import { manifestLabel, repositoryLabel } from "@armada/shell/src/repository-label";
import { ofPicked, tabOf } from "./board";
import { said } from "./copy";
import type { DriftsRead, FleetHealth, HealthRead, RepositoryDrift } from "./overview-reads";

/** A tile's reading. Where it opens is the band's, not the reading's. */
export type TileReading = Omit<OverviewTileProps, "opens" | "onOpen">;

/** The status bar's three hues, and nothing for the states that are none of them. */
const FLEET_TONE: Partial<Record<Connection["state"], OverviewTileTone>> = {
  connected: "completed-success",
  not_running: "completed-failed",
  unreachable: "awaiting-review",
};

/** The connection's own statement — the status bar's sentence, not a second one. */
export function fleetReading(connection: Connection, now: number, readAt: number | null): TileReading {
  const statement = statementOf(connection, now, readAt);
  const tone = FLEET_TONE[connection.state];
  return {
    label: "Fleet",
    value: statement.headline,
    ...(tone === undefined ? {} : { tone }),
    ...(statement.detail === "" ? {} : { detail: statement.detail, detailFace: "mono" as const }),
  };
}

/** Doctor's words, worst first. A word this build does not know ranks below all three. */
const OUTCOMES = ["fail", "warn", "pass"] as const;
const DOCTOR_TONE: Record<(typeof OUTCOMES)[number], OverviewTileTone> = {
  fail: "completed-failed",
  warn: "awaiting-review",
  pass: "completed-success",
};

export function doctorReading(read: HealthRead): TileReading {
  switch (read.state) {
    case "none":
    case "reading":
      return { label: "Doctor" };
    case "failed":
      return { label: "Doctor", value: "Not read", detail: said(read.outcome) };
    case "read":
      return doctorOf(read.health);
  }
}

/**
 * The worst word, and every module that did not pass named beside its own. **No blended score**:
 * a failing module is named rather than folded into a count a passing one could hide it in.
 */
function doctorOf(health: FleetHealth): TileReading {
  const first = health.probes[0];
  if (first === undefined) return { label: "Doctor", value: "Nothing probed" };
  const worst = OUTCOMES.find((word) => health.probes.some((probe) => probe.outcome === word));
  const flagged = health.probes.filter((probe) => probe.outcome !== "pass");
  const detail =
    flagged.length > 0
      ? flagged.map((probe) => `${probe.module}: ${probe.outcome}`).join(" · ")
      : health.probes.map((probe) => probe.module).join(", ");
  return {
    label: "Doctor",
    value: worst ?? first.outcome,
    valueFace: "mono",
    ...(worst === undefined ? {} : { tone: DOCTOR_TONE[worst] }),
    detail,
  };
}

/**
 * How full the fleet is. **What holds the next Drone back appears only while something is
 * queued**, the status bar's rule: a hold with nothing waiting on it answers no question.
 */
export function dronesReading(connection: Connection, capacity: FleetCapacity | null, queued: number): TileReading {
  if (capacity === null) {
    return connection.state === "connected" ? { label: "Drones" } : { label: "Drones", value: "Not read" };
  }
  const free = Math.max(0, capacity.bound - capacity.occupied);
  const hold = capacity.held_by;
  const detail =
    hold !== undefined && queued > 0 ? (ADMISSION_HOLD[hold]?.verb ?? hold) : free === 0 ? "None free" : `${free} free`;
  return { label: "Drones", value: `${capacity.occupied} of ${capacity.bound}`, valueFace: "mono", detail };
}

/** The queued Jobs in the scope, by the Board's own tab rule. */
export function queuedIn(jobs: readonly JobSummary[], picked: RepositorySummary | null): JobSummary[] {
  return ofPicked(jobs, picked).filter((job) => tabOf(job) === "queued");
}

/** How many are queued, and where. A pick names itself; All says how many repositories hold them. */
export function queuedReading(
  queued: readonly JobSummary[],
  served: readonly RepositorySummary[],
  picked: RepositorySummary | null,
): TileReading {
  const reading = { label: "Queued", value: String(queued.length), valueFace: "mono" as const };
  if (picked !== null) return { ...reading, detail: repositoryLabel(picked, served) };
  if (served.length <= 1) return reading;
  const owners = [...new Set(queued.map((job) => job.owner_manifest_id))];
  const only = owners.length === 1 ? owners[0] : undefined;
  if (queued.length === 0) return { ...reading, detail: `Across ${repositories(served.length)}` };
  if (only !== undefined) return { ...reading, detail: manifestLabel(only, served) };
  return { ...reading, detail: `In ${owners.length} of ${repositories(served.length)}` };
}

/** Whether a repository's drift names that it is behind, current, or could not be read. */
export function driftReading(read: DriftsRead, picked: RepositorySummary | null): TileReading {
  if (read.state === "none") return { label: "Manifest drift" };
  if (picked === null) return acrossOf(read.repositories);
  return oneOf(read.repositories.find((one) => one.root === picked.root));
}

function goneIn(drift: ManifestDrift): number {
  return drift.declarations.filter((line) => line.drift.verdict === "gone").length;
}

function notSetUp(one: RepositoryDrift): boolean {
  return one.drift.state === "failed" && !one.drift.outcome.ok && one.drift.outcome.why === "not_set_up";
}

/** One repository: how many of the lines `armada.yml` names are behind. */
function oneOf(one: RepositoryDrift | undefined): TileReading {
  const label = "Manifest drift";
  if (one === undefined) return { label };
  const drift = one.drift;
  switch (drift.state) {
    case "none":
    case "reading":
      return { label };
    case "failed":
      return notSetUp(one)
        ? { label, value: "Not set up", detail: "No armada.yml to read" }
        : { label, value: "Not read", detail: said(drift.outcome) };
    case "read": {
      const gone = goneIn(drift.drift);
      const lines = lines_(drift.drift.declarations.length);
      // A clean list says only that nothing it names is gone, never that every line is right.
      return gone === 0
        ? { label, value: "Current", detail: `${lines}, nothing they name is gone` }
        : { label, value: `${gone} behind`, tone: "notice-caution", detail: `Of ${lines} armada.yml names` };
    }
  }
}

/** Every repository: how many are behind, and which could not be asked. */
function acrossOf(each: readonly RepositoryDrift[]): TileReading {
  const label = "Manifest drift";
  if (each.length === 0) return { label, value: "No repositories" };
  const read = each.flatMap((one) => (one.drift.state === "read" ? [one.drift.drift] : []));
  const waiting = each.some((one) => one.drift.state === "none" || one.drift.state === "reading");
  if (read.length === 0 && waiting) return { label };
  const unset = each.filter(notSetUp).length;
  const unread = each.filter((one) => one.drift.state === "failed").length - unset;
  const behind = read.filter((drift) => goneIn(drift) > 0).length;
  const said = [
    behind > 0 ? `${behind} of ${repositories(read.length)}` : `${repositories(read.length)}, none behind`,
    ...(unset > 0 ? [`${unset} not set up`] : []),
    ...(unread > 0 ? [`${unread} not read`] : []),
  ];
  if (read.length === 0) return { label, value: unread > 0 ? "Not read" : "Not set up", detail: said.slice(1).join(" · ") };
  return behind > 0
    ? { label, value: `${behind} behind`, tone: "notice-caution", detail: said.join(" · ") }
    : { label, value: "Current", detail: said.join(" · ") };
}

function repositories(n: number): string {
  return `${n} ${n === 1 ? "repository" : "repositories"}`;
}

function lines_(n: number): string {
  return `${n} ${n === 1 ? "line" : "lines"}`;
}
