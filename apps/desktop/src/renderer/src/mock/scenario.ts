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
  workingAPlan,
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
import { DRIFT_GONE, GH_ISSUE_VIEW, RUNS, manifesting } from "./manifest-fleet";
import { SCRATCH, SHEET_READ, settingUp } from "./setup-fleet";
import { everyKind, studying, untitled } from "./studio-fleet";

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

/**
 * A connected Fleet holding these fixtures' Jobs, each with its own reads.
 *
 * `alsoServed` is served beside what the Manifests imply. **Every repository a
 * fixture reaches is set up by construction** — `servedFrom` builds one per
 * Manifest — so a repository nobody set up can only arrive this way.
 */
function holding(
  name: string,
  says: string,
  fixtures: JobFixture[],
  { opens, alsoServed = [] }: { opens?: string; alsoServed?: RepositorySummary[] } = {},
): Scenario {
  const manifests = distinct(fixtures.flatMap((one) => one.manifests), (one) => one.id);
  return {
    name,
    says,
    state: connected(
      fixtures.map((one) => one.job),
      distinct(fixtures.flatMap((one) => one.workflows), (one) => `${one.manifest_id}/${one.id}`),
      [...manifests.map(servedFrom), ...alsoServed],
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
  const scenario = holding("job", fixture.name, [fixture], { opens: fixture.job.id });
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
 *
 * **The title is given, never taken from the builder's `name`.** That name is
 * the state the fixture demonstrates, and a row built from it read `running —
 * the drone is waiting for a person to allow a command` where a Job's title
 * reads `Cache the manifest read`. `EVERY_STATE_TITLES` is where the row's own
 * title is written.
 */
function asRow(fixture: JobFixture, at: number, slug: string, title: string): JobFixture {
  const id = `01M2C1TJ8G00${String(at).padStart(2, "0")}EVERYSTATE00`;
  const renamed = { id, handle: `${at}-${slug}`, title };
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
export const BUILDERS = {
  running,
  workingAPlan,
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
} satisfies Record<string, () => JobFixture>;

/** One builder's export name. `satisfies` above is what keeps this a union. */
type Builder = keyof typeof BUILDERS;

/**
 * The Job title each builder's `every-state` row carries, one per builder.
 *
 * **A builder's `name` is not a title.** It names the scenario and the row's
 * handle, and it reads as the state being demonstrated — a status word, a gate
 * or a Check, joined by an em dash to what happened. Drawn as a title it said
 * the same thing the status badge beside it already says, and no row read like
 * work anybody asked for (owner note, 17 Sep 2026). So each row carries a
 * change somebody asked for instead, in the voice `fixtures/build/board.ts`
 * writes the Board's rows in.
 *
 * Typed against `BUILDERS`, so a builder added without a title is a compile
 * error rather than a row back in the old shape.
 */
const EVERY_STATE_TITLES: Record<Builder, string> = {
  running: "Cache the manifest read between dispatches",
  workingAPlan: "Extract the column order selector into its own module",
  runningWaitingOnACommand: "Reuse one HTTP client across every query",
  review: "Fold the two notification routes into one",
  escalatedGateFailure: "Shorten the reconnect backoff to two seconds",
  escalatedEvidenceSuspect: "Prune the evidence bundle before it is written",
  reviewAtDelivery: "Carry the branch name into the pull request body",
  queued: "Move the worktree prune off the startup path",
  awaitingApproval: "Widen the allowlist to cover read-only git commands",
  awaitingRepair: "Coalesce the journal writes into one flush",
  awaitingAttestation: "Give the runtime file a version field",
  piloted: "Make the diff pane remember its split",
  escalatedBlockedByPolicy: "Teach the dispatcher to read a repository alias",
  escalatedInterrupted: "Record the port Fleet claimed in its own log",
  escalatedSilent: "Stop the log pane scrolling on a background write",
  escalatedLoopCap: "Round the cost estimate to the nearest cent",
  escalatedNoReport: "Collapse repeated journal notes into one row",
  completedSuccess: "Debounce the Job Board's resize handler",
  completedFailed: "Drop the second clock from the elapsed figure",
  rejected: "Send the manifest digest with every dispatch",
  killed: "Read the workflow file once per dispatch",
  superseded: "Sort the Job Board by when a job last moved",
  preparing: "Name the drone in the resources panel",
  reading: "Give every sheet its own scroll position",
  unreadable: "Keep the Helm dock open across a restart",
  retryingCheckFailure: "Trim the brief to the files the step touched",
  runningAtGate: "Hold the composer's draft while a job is open",
  gateChecksStreaming: "Let the palette open on an empty Job Board",
};

const BUILT: [Builder, JobFixture][] = Object.entries(BUILDERS).map(([name, make]) => [
  name as Builder,
  make(),
]);

/** Every recording, by its directory. */
const RECORDED: [string, JobFixture][] = RECORDED_SLUGS.map((slug) => [slug, recorded(slug)]);

/** Every builder as an `every-state` row, each on its own id and title. Held, so a Studio can name one. */
const EVERY_STATE_ROWS: JobFixture[] = BUILT.map(([name, fixture], at) =>
  asRow(fixture, at + 1, name, EVERY_STATE_TITLES[name]),
);

/**
 * A second folder added by path and never set up, so that `NOTHING_SET_UP`
 * holds more than one — the surfaces that ask for a repository ask because
 * there are several, and one of them alone would not say so.
 */
const NOTES: RepositorySummary = {
  root: "/Users/user/notes",
  records_root: "/Users/user/Library/Application Support/Armada/records/notes",
};

/**
 * Repositories served, and not one of them with a Manifest. **The moment every
 * surface that needs a Manifest has nothing to offer**: the rail is on All
 * repositories, the ask lists only what is set up, and nothing is. Studios is
 * where it was found.
 */
const NOTHING_SET_UP: Scenario = {
  name: "nothing-set-up",
  says: "Two repositories served, neither set up",
  state: connected([], [], [SCRATCH, NOTES]),
  reads: {},
};

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
  {
    ...holding(
      "every-state",
      "One Job in every state, each openable",
      [
        ...EVERY_STATE_ROWS,
        // A recording keeps its own id, which no built row shares. The one
        // recorded today is the Board's Cleared tab, which no builder reaches.
        ...RECORDED.map(([, fixture]) => fixture),
      ],
      // The folder somebody added and never set up. **Without it nobody
      // browsing the mock ever sees the greyed-out half of a repository
      // picker**: every repository here comes from a Manifest, so New job's
      // ask and Studios' ask drew nothing under Not set up, and the one
      // scenario that shows an unset repository — `nothing-set-up` — has no
      // set-up one to show it beside. `scratch` rather than a new name: it is
      // the folder Setup is run against, and the same folder in every
      // scenario that needs one nobody set up.
      { alsoServed: [SCRATCH] },
    ),
    // Studios worth looking at, the way the rows are Jobs worth looking at: every node kind and
    // every edge kind on one, and a second nobody has named — #1341.
    studios: [everyKind(EVERY_STATE_ROWS[0]!.job.id), untitled()],
  },
  NOT_RUNNING,
  NOTHING_SET_UP,
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
  manifesting({ alwaysAllowed: [GH_ISSUE_VIEW], drift: DRIFT_GONE, runs: RUNS }),
  studying().scenario,
  ...BUILT.map(([name, fixture]) => holding(`job/${name}`, fixture.name, [fixture], { opens: fixture.job.id })),
  ...RECORDED.map(([slug, fixture]) =>
    holding(`recorded/${slug}`, fixture.name, [fixture], { opens: fixture.job.id }),
  ),
];

/** The scenario by name, or `undefined` for a name nothing here holds. */
export function scenarioNamed(name: string): Scenario | undefined {
  return SCENARIOS.find((one) => one.name === name);
}
