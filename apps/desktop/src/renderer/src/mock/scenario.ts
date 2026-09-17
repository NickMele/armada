// A named moment for the whole app: the state main would publish, and every
// per-Job read behind it. Built from `@armada/screens`' fixtures, never beside
// them — `docs/practices/running-locally.md`, *Bridge on a mock Fleet*.

import { connectedTo, PROTOCOL_VERSION } from "@armada/protocol";
import type {
  Connection,
  JobSummary,
  ManifestSummary,
  ModelChoices,
  Outcome,
  RepositorySummary,
  WorkflowSummary,
} from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import * as build from "@armada/screens/src/fixtures/build/index";
import { repository, workflow } from "@armada/screens/src/fixtures/build/base";
import { recorded, RECORDED_SLUGS } from "@armada/screens/src/fixtures/recorded";
import realBoard from "@armada/screens/src/fixtures/boards/real-board.json";

import { NOTHING_YET } from "../../../shared/bridge";
import type { BridgeState } from "../../../shared/bridge";

/** One moment: what is published before anything is opened, and the reads behind each Job. */
export type Scenario = {
  name: string;
  /** What it shows, as a sentence. The picker lists it. */
  says: string;
  state: BridgeState;
  /** Every Job's own reads, by Job id. A Job absent here opens onto `unanswered`. */
  reads: Record<string, JobFixture>;
  /** A Job to open on start, the way a pressed notification opens one. */
  opens?: string;
};

/**
 * What a read the scenario holds nothing for comes back as. **A transport
 * failure, not a refusal**: a refusal carries a code, and nothing outside Fleet
 * may mint one (`refusedWith` in `@armada/protocol` says why).
 */
export function unanswered(path: string): Outcome {
  return {
    ok: false,
    why: "transport",
    detail: "Not in this mock scenario — no Fleet is behind this window",
    fault: { method: "GET", path, why: "unreachable" },
  };
}

/** A Fleet that answered. Invented: no process has this pid or this port. */
const CONNECTED: Connection = connectedTo(
  { protocolVersion: PROTOCOL_VERSION, pid: 4242, port: 7878, startedAt: "2026-09-10T14:00:00Z" },
  1,
);

/** What `list_models` answers — `props.ts`' own guess, so a story and the app agree. */
const MODELS: ModelChoices = { models: ["haiku", "sonnet", "opus"], default: "sonnet" };

/** The state a connected Fleet publishes, holding these Jobs. */
function connected(
  jobs: JobSummary[],
  workflows: WorkflowSummary[],
  repositories: RepositorySummary[],
): BridgeState {
  return {
    ...NOTHING_YET,
    connection: CONNECTED,
    jobs,
    readAt: Date.now(),
    holds: {
      workflows,
      manifests: repositories.flatMap((one) => (one.manifest === undefined ? [] : [one.manifest])),
      models: MODELS,
      repositories,
    },
  };
}

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

/** Every builder `fixtures/build/index.ts` exports, by its export name. */
const BUILT: [string, JobFixture][] = Object.entries(build)
  .filter((entry): entry is [string, () => JobFixture] => typeof entry[1] === "function")
  .map(([name, make]) => [name, make()]);

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
 * A builder added to `fixtures/build/index.ts`, or a recording added under
 * `fixtures/recorded/`, is a scenario here and a row on `every-state` with no
 * edit to this file.
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
  ...BUILT.map(([name, fixture]) => holding(`job/${name}`, fixture.name, [fixture], fixture.job.id)),
  ...RECORDED.map(([slug, fixture]) => holding(`recorded/${slug}`, fixture.name, [fixture], fixture.job.id)),
];

/** The scenario by name, or `undefined` for a name nothing here holds. */
export function scenarioNamed(name: string): Scenario | undefined {
  return SCENARIOS.find((one) => one.name === name);
}
