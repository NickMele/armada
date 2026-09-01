// Bridge's frame: the rail, the panel the journeys mount in, and the status
// bar. The drawn shell, wired to what Fleet actually serves.
//
// # One surface, because one surface exists
//
// The rail carries the Job Board and nothing else. Alerts, Doctor, Manifest
// and Helm are named in the concept page and none of them is built — four
// disabled rows would be a promise Armada does not keep, so the roster is
// what exists.
//
// Active Jobs, Reviews and the Activity Feed used to be named here too. They
// are not surfaces any more: the Board holds every Job with state as a filter,
// and each of the three is that list under one filter. See
// `docs/concepts/bridge.md`.
//
// # One region of the drawing has no source, and is left out
//
// `today ~$4.80`: nothing measures spend, and it is not drawn as a labelled
// blank — a label with nothing under it reads as a value that failed to load.
//
// `1 drone` used to be here for the same reason: `assigned_drone` has no event
// that sets it, so no count could be taken from the board. `get_capacity`
// answers it now, from the roster admission itself counts against, so the
// contract's `pid, port, drone count` is finally all three.
//
// # The bar names Fleet's state; the panel head carries the controls
//
// `Refresh` used to live in the bar. The design contract is explicit that the
// bar carries no icons and that its only colour is the dot and the two counts,
// so the control moved up beside `New job` and the bar is what it is specified
// to be.

import { useEffect, useState, type ReactNode } from "react";
import { ClipboardList } from "lucide-react";
import { plural, Select, TheShell, type FleetState, type StatusBarProps } from "@armada/components";

import type { Connection } from "@armada/protocol";
import type { FleetCapacity, JobSummary } from "@armada/protocol";
import type { ManifestSummary } from "@armada/protocol";
import type { Statement } from "./fleet";
import { ADMISSION_HOLD } from "@armada/components";

/** The one surface in the rail. Its glyph is the registry's for Job Board. */
const JOB_BOARD = "board";

export type ShellProps = {
  connection: Connection;
  /** Fleet's state in the words the design contract settles. */
  statement: Statement;
  /** What Fleet holds. One entry is why the drawing shows the control. */
  manifests: readonly ManifestSummary[];
  /** The Manifest the rail names. What a new Job starts pointed at. */
  scope: string;
  onScope: (manifestId: string) => void;
  /** Every Job, for the rail's count and the bar's two. */
  jobs: readonly JobSummary[];
  /** How full the fleet is. `null` is a Fleet that has not answered yet. */
  capacity: FleetCapacity | null;
  title: string;
  summary?: string;
  /** The head's trailing controls. `New job` is the one primary. */
  actions?: ReactNode;
  /** Selecting the rail's surface returns to it. The rail is the one thing
   *  present on every view, so it is where a person looks to get back —
   *  Escape and Cancel both work and neither is what they reach for. */
  onSurface?: () => void;
  children: ReactNode;
};

export function Shell({
  connection,
  statement,
  manifests,
  scope,
  onScope,
  jobs,
  capacity,
  title,
  summary,
  actions,
  onSurface,
  children,
}: ShellProps) {
  const collapsed = useNarrow();

  return (
    <TheShell
      surfaces={[
        { id: JOB_BOARD, label: "Job Board", icon: ClipboardList, count: jobs.length },
      ]}
      activeId={JOB_BOARD}
      onSelect={onSurface === undefined ? undefined : () => onSurface()}
      collapsed={collapsed}
      railHeader={
        <Select
          aria-label="Project"
          value={scope}
          onChange={(event) => onScope(event.target.value)}
          // A picker over what Fleet holds, like every other id field in
          // Bridge. Empty until the connection answers, and the option says so
          // rather than showing a blank control.
          disabled={manifests.length === 0}
        >
          {manifests.length === 0 ? <option value="">No manifest read yet</option> : null}
          {manifests.map((manifest) => (
            <option key={manifest.id} value={manifest.id}>
              {/* The repository, because `armada.yml` declares no name. */}
              {manifest.repository}
            </option>
          ))}
        </Select>
      }
      title={title}
      summary={summary}
      actions={actions}
      status={statusOf(connection, statement, jobs, capacity)}
    >
      {children}
    </TheShell>
  );
}

/**
 * The bar, from what Bridge holds.
 *
 * **No spend**, for the reason at the top of this file. The job count is the
 * contract's own middle segment — "Fleet running · 3 jobs" — and the two
 * waiting counts appear only when non-zero.
 */
function statusOf(
  connection: Connection,
  statement: Statement,
  jobs: readonly JobSummary[],
  capacity: FleetCapacity | null,
): StatusBarProps {
  const detail = [statement.detail, drones(capacity)].filter((part) => part !== "").join(" · ");
  return {
    fleet: fleetOf(connection),
    fleetLabel: statement.headline,
    detail: detail === "" ? undefined : detail,
    advice: statement.next ?? undefined,
    items: [plural(jobs.length), ...held(jobs, capacity)],
    escalations: jobs.filter((job) => job.status === "escalated").length,
    approvals: jobs.filter((job) => job.status === "awaiting_approval").length,
  };
}

/**
 * "2 of 2 drones", the contract's third mono value, and empty where Fleet has
 * not answered.
 *
 * **Empty rather than "0 of 0"**, because a Fleet that has not been asked and a
 * Fleet with nothing running are different facts and one of them is not known.
 */
function drones(capacity: FleetCapacity | null): string {
  if (capacity === null) return "";
  return `${capacity.occupied} of ${capacity.bound} ${capacity.bound === 1 ? "drone" : "drones"}`;
}

/**
 * Which of the four things is holding the next drone back, **and only while
 * something is waiting on it**.
 *
 * Fleet answers this whenever admission would refuse, which includes a fleet
 * with nothing queued at all. Drawn then it is a warning about a situation
 * nobody is in, and the contract is explicit that the bar must not become a
 * second alert surface. So it appears when there is a Job it is an answer for.
 *
 * **The word is the registry's.** An unknown key is a newer Fleet naming a
 * reason this build has never heard of — additive by design — and it renders as
 * its own wire spelling, which is the fallback every other surface takes rather
 * than inventing a second vocabulary.
 */
function held(jobs: readonly JobSummary[], capacity: FleetCapacity | null): string[] {
  const hold = capacity?.held_by;
  if (hold === undefined) return [];
  if (!jobs.some((job) => job.status === "queued")) return [];
  return [ADMISSION_HOLD[hold]?.verb ?? hold];
}

/**
 * Which of the dot's hues this reading takes.
 *
 * Four of Bridge's seven connection states are none of the contract's three —
 * reading, connecting, a refused runtime file and a protocol Bridge does not
 * speak. They keep the neutral dot rather than borrowing a fourth hue; the
 * sentence beside it names each one, and the failure notice on the board
 * carries the whole reading.
 */
function fleetOf(connection: Connection): FleetState {
  switch (connection.state) {
    case "connected":
      return "running";
    case "not_running":
      return "not-running";
    case "unreachable":
      return "unreachable";
    default:
      return "unknown";
  }
}

/**
 * Whether the window is below the layout breakpoint, so the rail collapses to
 * its 48px form. **The bound is read from the token**, never retyped: the
 * theme calls `--layout-breakpoint` a media query bound and this is the media
 * query. A layout designed only for the size it was built at is the v1 failure
 * this app exists to escape.
 */
function useNarrow(): boolean {
  const [narrow, setNarrow] = useState(false);

  useEffect(() => {
    const bound = getComputedStyle(document.documentElement)
      .getPropertyValue("--layout-breakpoint")
      .trim();
    if (bound === "") return undefined;
    const query = window.matchMedia(`(max-width: ${bound})`);
    const read = (): void => setNarrow(query.matches);
    read();
    query.addEventListener("change", read);
    return () => query.removeEventListener("change", read);
  }, []);

  return narrow;
}
