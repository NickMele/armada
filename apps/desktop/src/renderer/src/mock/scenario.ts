// A named moment for the whole app: the state main would publish, and every
// per-Job read behind it. Built from `@armada/screens`' fixtures, never beside
// them — `docs/practices/running-locally.md`, *Bridge on a mock Fleet*.

import type {
  JobSummary,
  ManifestSummary,
  RepositorySummary,
  WorkflowSummary,
} from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import {
  running,
  runningWaitingOnACommand,
  review,
  escalatedGateFailure,
  escalatedEvidenceSuspect,
  reviewAtDelivery,
  queued,
  awaitingApproval,
  awaitingRepair,
  awaitingAttestation,
  piloted,
  escalatedBlockedByPolicy,
  escalatedInterrupted,
  escalatedSilent,
  escalatedLoopCap,
  escalatedNoReport,
  completedSuccess,
  completedFailed,
  rejected,
  killed,
  superseded,
  preparing,
  reading,
  unreadable,
  retryingCheckFailure,
  runningAtGate,
  gateChecksStreaming,
} from "@armada/screens/src/fixtures/build/index";
import { repository, workflow } from "@armada/screens/src/fixtures/build/base";
import { recorded, RECORDED_SLUGS } from "@armada/screens/src/fixtures/recorded";
import realBoard from "@armada/screens/src/fixtures/boards/real-board.json";

import { NOTHING_YET } from "../../../shared/bridge";
import { connected } from "./moment";
import type { Scenario } from "./moment";
import { DRIFT_GONE, GH_ISSUE_VIEW, manifesting } from "./manifest-fleet";
import { SCRATCH, SHEET_READ, settingUp } from "./setup-fleet";
import { studying } from "./studio-fleet";

export { connected, onBoard, unanswered } from "./moment";
export type { FleetHandle, Scenario } from "./moment";

/** One of each, by manifest and id — two fixtures on one workflow list it once. */
function distinct<T>(items: T[], key: (item: T) => string): T[] {
  return [...new Map(items.map((item) => [key(item), item])).values()];
}

/**
 * The repository a Manifest is read from. **Invented where a fixture names only
 * the Manifest**: the root is made up, and nothing a screen draws reads it
 * except the rail's label, which is the Manifest's own `repository`.
 */
function servedFrom(manifest: ManifestSummary): RepositorySummary {
  return manifest.id === repository().manifest?.id
    ? repository()
    : { root: `/Users/user/${manifest.id}`, records_root: manifest.records_root, manifest };
}

/** A connected Fleet holding these fixtures' Jobs, each with its own reads. */
function holding(name: string, says: string, fixtures: JobFixture[], opens?: string): Scenario {
  const manifests = distinct(fixtures.flatMap((one) => one.manifests), (one) => one.id);
  return {
    name,
    says,
    state: connected(
      fixtures.map((one) => one.job),
      distinct(fixtures.flatMap((one) => one.workflows), (one) => `${one.manifest_id}/${one.id}`),
      manifests.map(servedFrom),
    ),
    reads: Object.fromEntries(fixtures.map((one) => [one.job.id, one])),
    opens,
  };
}

/**
 * One Job, already open, the way a pressed notification opens it — for a test
 * that moved a fixture to a moment no builder names. `whereOpen` is the
 * person's own preference for the Where things are section.
 */
export function onJob(fixture: JobFixture, { whereOpen = false }: { whereOpen?: boolean } = {}): Scenario {
  const scenario = holding("job", fixture.name, [fixture], fixture.job.id);
  return {
    ...scenario,
    state: { ...scenario.state, preferences: { ...scenario.state.preferences, where_things_are_open: whereOpen } },
  };
}

/**
 * The fixture, moved onto another id, handle and title.
 *
 * **Every `build/` fixture is the same Job** — one narrative at many
 * moments, `base.ts` says why — so on one Board they would be one row. Each
 * read that names its Job is moved with it, or the detail would draw a
 * different Job's reads as "not this one's".
 */
function asRow(fixture: JobFixture, at: number, slug: string): JobFixture {
  const id = `01M2C1TJ8G00${String(at).padStart(2, "0")}EVERYSTATE00`;
  const renamed = { id, handle: `${at}-${slug}`, title: fixture.name };
  const job = { ...fixture.job, ...renamed };
  const moved = <Read extends { state: string }>(read: Read): Read =>
    "jobId" in read ? { ...read, jobId: id } : read;
  const watched = moved(fixture.watched);
  return {
    ...fixture,
    job,
    watched:
      watched.state === "read"
        ? { ...watched, detail: { ...watched.detail, job: { ...watched.detail.job, ...renamed } } }
        : watched,
    observed: moved(fixture.observed),
    journalled: moved(fixture.journalled),
    resources: moved(fixture.resources),
    history: fixture.history === undefined ? undefined : moved(fixture.history),
    recorded: {
      footprint: moved(fixture.recorded.footprint),
      handed: moved(fixture.recorded.handed),
      evidence: moved(fixture.recorded.evidence),
      diff: moved(fixture.recorded.diff),
      remarks: moved(fixture.recorded.remarks),
    },
  };
}

/**
 * Every builder `fixtures/build/index.ts` exports, by its export name. **Named
 * one by one**, because the vocabulary gate refuses a wholesale import;
 * `scenario.test.ts` fails where this and `FIXTURES` disagree.
 */
export const BUILDERS: Record<string, () => JobFixture> = {
  running,
  runningWaitingOnACommand,
  review,
  escalatedGateFailure,
  escalatedEvidenceSuspect,
  reviewAtDelivery,
  queued,
  awaitingApproval,
  awaitingRepair,
  awaitingAttestation,
  piloted,
  escalatedBlockedByPolicy,
  escalatedInterrupted,
  escalatedSilent,
  escalatedLoopCap,
  escalatedNoReport,
  completedSuccess,
  completedFailed,
  rejected,
  killed,
  superseded,
  preparing,
  reading,
  unreadable,
  retryingCheckFailure,
  runningAtGate,
  gateChecksStreaming,
};

const BUILT: [string, JobFixture][] = Object.entries(BUILDERS).map(([name, make]) => [name, make()]);

/** Every recording, by its directory. */
const RECORDED: [string, JobFixture][] = RECORDED_SLUGS.map((slug) => [slug, recorded(slug)]);

/** A Fleet that is not running: no runtime file, so nothing was ever connected. */
const NOT_RUNNING: Scenario = {
  name: "fleet-not-running",
  says: "Fleet is not running — no runtime file",
  state: {
    ...NOTHING_YET,
    connection: {
      state: "not_running",
      absence: {
        why: "no_runtime_file",
        path: "/Users/user/Library/Application Support/Armada/fleet.json",
      },
    },
  },
  reads: {},
};

/** The Board Fleet served on 11 Sep 2026, recorded with `scripts/record-job.mjs --board`. */
function recordedBoard(): Scenario {
  const board = realBoard as unknown as {
    jobs: JobSummary[];
    workflows: WorkflowSummary[];
    manifests: ManifestSummary[];
  };
  const reads = Object.fromEntries(
    RECORDED.map(([, fixture]) => fixture).filter((one) => board.jobs.some((job) => job.id === one.job.id))
      .map((one) => [one.job.id, one]),
  );
  return {
    name: "recorded-board",
    says: "The Board as Fleet served it on 11 Sep 2026",
    state: connected(board.jobs, board.workflows, board.manifests.map(servedFrom)),
    reads,
  };
}

/**
 * Every scenario, by name. **The first is where the mock opens.**
 *
 * A recording added under `fixtures/recorded/` is a scenario and an `every-state`
 * row with no edit here; a builder needs its line in `BUILDERS`.
 */
export const SCENARIOS: readonly Scenario[] = [
  holding(
    "every-state",
    "One Job in every state, each openable",
    [
      ...BUILT.map(([name, fixture], at) => asRow(fixture, at + 1, name)),
      // A recording keeps its own id, which no built row shares. The one
      // recorded today is the Board's Cleared tab, which no builder reaches.
      ...RECORDED.map(([, fixture]) => fixture),
    ],
  ),
  NOT_RUNNING,
  {
    name: "first-launch",
    says: "Fleet running and serving no repository",
    state: connected([], [], []),
    reads: {},
  },
  {
    name: "empty-store",
    says: "One repository, and no Job yet",
    state: connected([], [workflow()], [repository()]),
    reads: {},
  },
  recordedBoard(),
  settingUp({ repositories: [repository(), SCRATCH], sheet: SHEET_READ }),
  manifesting({ alwaysAllowed: [GH_ISSUE_VIEW], drift: DRIFT_GONE }),
  studying().scenario,
  ...BUILT.map(([name, fixture]) => holding(`job/${name}`, fixture.name, [fixture], fixture.job.id)),
  ...RECORDED.map(([slug, fixture]) => holding(`recorded/${slug}`, fixture.name, [fixture], fixture.job.id)),
];

/** The scenario by name, or `undefined` for a name nothing here holds. */
export function scenarioNamed(name: string): Scenario | undefined {
  return SCENARIOS.find((one) => one.name === name);
}
