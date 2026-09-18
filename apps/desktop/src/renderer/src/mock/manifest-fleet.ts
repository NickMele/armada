// A Fleet holding this repository's own Manifest: the file on a disk, a watch
// that re-reads it after a save, the forms' edits applied as Fleet's writer
// would, the run sheet, runs and their diffs, drift, and repository-wide
// allows. Moved here from the `Screens/Manifest` story's wrapper, so the
// Manifest surface runs through `App` — #1224.

import type {
  AllowedCommandRow,
  CheckoutRunDiff,
  CheckoutRunFollowed,
  CheckoutRunList,
  CheckoutRunRecord,
  CheckoutRunSheet,
  CheckoutRunSheetRead,
  CheckoutRunUnderway,
  CheckoutVerify,
  EditManifest,
  KitServerRow,
  ManifestDeclared,
  ManifestDriftRead,
  ManifestSpend,
  Outcome,
  RunEntry,
  ServerEntry,
  StartCheckoutRun,
  VerifyStep,
} from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import type { BridgeApi } from "../../../shared/api";
import { appliedTo } from "./manifest-applied";
import { onBoard } from "./moment";
import type { FleetHandle, Scenario } from "./moment";


/** Where Fleet resolved the Manifest, in the shape `root.join("armada.yml")` gives. */
export const MANIFEST_PATH = "/Users/user/armada/armada.yml";

/** A cut of this repository's own `armada.yml`, as `GET /manifest/file` answers it. */
export const MANIFEST_TEXT = `version: 1
id: armada
base: main

drone:
  poke_limit: 3

checks:
  build:
    run: cargo build --workspace --locked
  typecheck:
    run: pnpm typecheck
`;

/**
 * What the forms draw from: this repository's own Checks and a few of its
 * Commands, as `GET /manifest/file` carries them beside the text.
 */
export const DECLARED: ManifestDeclared = {
  id: "armada",
  version: 1,
  checks: [
    { name: "build", check: { run: "cargo build --workspace --locked" } },
    { name: "typecheck", check: { run: "pnpm typecheck", requires: ["bootstrap"] } },
  ],
  commands: [
    { name: "bootstrap", command: { run: "pnpm install --frozen-lockfile", destructive: false } },
    { name: "fmt", command: { run: "cargo fmt --all", destructive: false } },
    { name: "gate", command: { run: "cargo xtask verify-foundations", destructive: false } },
  ],
  ports: [],
  freeze: false,
  auto_merge: { written: "never", offered: ["never", "checks-pass", "always"] },
  review_gate: { written: "human_always", offered: ["human_always", "auto_if_judge_passes"] },
  base: "main",
  evidence: {
    serve: "pnpm -C packages/components exec storybook dev -p ${port.storybook} --no-open --ci",
    ready: "curl -sf http://localhost:${port.storybook}",
    run: "pnpm -C packages/components exec playwright test {}",
    frames: ".armada/frames",
    never: ["/__notes"],
  },
  setup_requires: ["bootstrap"],
  poke_limit: 3,
};

/** No Job has spent anything here, so the budget section warns about nothing. */
const NO_SPEND: ManifestSpend = { jobs: 0, most_cost_micros: 0, most_turns: 0 };

/** What pressing Save on the forms comes to: written, or refused for a result that would not load. */
export type EditGoesTo = "took" | "refused";

/** Drift on this repository's own lines, every one still there. */
export const DRIFT_CURRENT: ManifestDriftRead = {
  state: "read",
  drift: {
    path: MANIFEST_PATH,
    checkout: "/Users/user/armada",
    declarations: [
      { section: "checks", name: "build", key: "run", run: "cargo build --workspace --locked", drift: { verdict: "current", checked: 1 }, unfollowed: [] },
      { section: "checks", name: "typecheck", key: "run", run: "pnpm typecheck", drift: { verdict: "current", checked: 1 }, unfollowed: [] },
      { section: "checks", name: "bridge_test", key: "run", run: "pnpm bridge-test", drift: { verdict: "current", checked: 1 }, unfollowed: [] },
    ],
  },
};

/** Drift with `bridge_test` gone: after `run`, a missing script is `gone` rather than unfollowed. */
export const DRIFT_GONE: ManifestDriftRead = {
  state: "read",
  drift: {
    path: MANIFEST_PATH,
    checkout: "/Users/user/armada",
    declarations: [
      { section: "checks", name: "build", key: "run", run: "cargo build --workspace --locked", drift: { verdict: "current", checked: 1 }, unfollowed: [] },
      { section: "checks", name: "typecheck", key: "run", run: "pnpm typecheck", drift: { verdict: "current", checked: 1 }, unfollowed: [] },
      { section: "checks", name: "bridge_test", key: "run", run: "pnpm run bridge-test", drift: { verdict: "gone", missing: ["package.json: scripts.bridge-test"] }, unfollowed: [] },
    ],
  },
};

/** A Verify step that ran, with as much of its record as the panel reads. */
function ran(group: string, name: string, run: string, exit: number, ms: number): VerifyStep {
  const record = {
    id: `crun_${name}`, name, command: run, required: [],
    started_at: "2026-09-12T14:11:00Z", ended_at: "2026-09-12T14:11:09Z", duration_ms: ms,
    exit_code: exit, expect_exit_code: 0, ended: `exited ${exit}`, stopped: false,
    changed: [], undoable: false, log: `runs/crun_${name}/output.log`,
  };
  return { group, name, run, state: "ran", record };
}

/** `build`, out as Verify's third step, and what it has printed. */
export const BUILD_OUT: CheckoutRunUnderway = {
  id: "crun_build",
  name: "build",
  command: "cargo build --workspace --locked",
  started_at: "2026-09-12T14:19:18Z",
};
export const BUILD_FOLLOWED: CheckoutRunFollowed = {
  state: "following",
  runId: BUILD_OUT.id,
  name: "build",
  path: ".armada/runs/crun_build/output.log",
  fromLine: 1,
  lines: ["   Compiling ipc v0.0.0", "   Compiling api v0.0.0", "   Compiling fleet v0.0.0"],
};

/** Setup ran, `build` is out, and the rest are not reached yet. */
export const VERIFY_UNDERWAY: CheckoutVerify = {
  id: "01VERIFY",
  started_at: "2026-09-12T14:19:02Z",
  steps: [
    ran("setup", "bootstrap", "pnpm install --frozen-lockfile", 0, 8200),
    ran("setup", "browsers", "pnpm -C packages/components exec playwright install chromium --only-shell", 0, 1400),
    { group: "checks", name: "build", run: BUILD_OUT.command, state: "running", run_id: BUILD_OUT.id },
    { group: "checks", name: "test", run: "cargo nextest run --workspace --exclude acceptance", state: "waiting" },
    { group: "checks", name: "typecheck", run: "pnpm typecheck", state: "waiting" },
  ],
};

/** Ended: `typecheck` exited 2 where 0 is expected, and the Check after it still ran. */
export const VERIFY_ENDED: CheckoutVerify = {
  id: "01VERIFY",
  started_at: "2026-09-12T14:11:00Z",
  ended_at: "2026-09-12T14:12:30Z",
  steps: [
    ran("setup", "bootstrap", "pnpm install --frozen-lockfile", 0, 8200),
    ran("checks", "build", "cargo build --workspace --locked", 0, 41000),
    ran("checks", "typecheck", "pnpm typecheck", 2, 9400),
    ran("checks", "format", "cargo fmt --all --check", 0, 1800),
  ],
};

/** One finished run in this checkout, as `list_checkout_runs` rows it. */
function record(
  name: string,
  command: string,
  at: string,
  exit: number,
  ms: number,
): CheckoutRunRecord {
  return {
    id: `crun_${name}_${at.slice(11, 16).replace(":", "")}`,
    name,
    command,
    required: [],
    started_at: at,
    ended_at: at,
    duration_ms: ms,
    exit_code: exit,
    expect_exit_code: 0,
    ended: `exited ${exit}`,
    stopped: false,
    changed: [],
    undoable: false,
    log: `runs/crun_${name}/output.log`,
  };
}

/**
 * What has run in this checkout, newest first — so the list on the left says
 * what each entry last came to, which is what #1383 asked it to carry.
 * `typecheck` is the one that came back with a code it did not expect.
 */
export const RUNS: CheckoutRunList = {
  runs: [
    record("typecheck", "pnpm typecheck", "2026-09-12T14:11:30Z", 2, 9420),
    record("build", "cargo build --workspace --locked", "2026-09-12T14:09:02Z", 0, 41_300),
    record("bootstrap", "pnpm install --frozen-lockfile", "2026-09-12T13:58:11Z", 0, 8200),
  ],
  unreadable: [],
};

/** What a pull brought in while the edit was open. */
export const PULLED_TEXT = MANIFEST_TEXT.replace("base: main", "base: main\n\nauto_merge: never");

/**
 * A repository-wide always-allow, as `get_repository_allowed_commands`
 * answers it — #836's own case: `gh issue view`, always-allowed from Job 7
 * while filing #834.
 */
export const GH_ISSUE_VIEW: AllowedCommandRow = {
  run: "gh issue view",
  reach: "repository",
  allowed_at: "2026-09-13T09:41:00Z",
  by: "person",
};

/**
 * A kit with both tiers already saying something, so the mock shows a Drone
 * here getting one server and withheld from another. #1275.
 */
export const KIT_SERVERS: KitServerRow[] = [
  {
    name: "tracker",
    address: { transport: "stdio", command: "npx", args: ["-y", "@scope/server-tracker"] },
    drones: "no",
    manifest: "extended",
    resolves: true,
    added_at: "2026-09-16T11:02:00Z",
    by: "human",
  },
  {
    name: "nexus",
    address: { transport: "http", url: "https://nexus.example.com/mcp" },
    drones: "yes",
    manifest: "restricted",
    resolves: false,
    added_at: "2026-09-16T11:04:00Z",
    by: "human",
  },
  {
    name: "sentry",
    address: { transport: "stdio", command: "sentry-mcp", args: [] },
    drones: "no",
    resolves: false,
    added_at: "2026-09-17T08:30:00Z",
    by: "human",
  },
];

/**
 * What a save comes to, played the way Fleet plays it: the write answers,
 * then the watch re-reads and publishes. `refused` is a correction Fleet would
 * not adopt; `moved` is a pull that landed under the edit.
 */
export type SaveGoesTo = "took" | "refused" | "moved";

/** The instant a save in a story lands, and the reading that follows it. */
const WROTE_AT = "2026-09-12T14:20:03.120Z";
const READ_AT = "2026-09-12T14:20:04.700Z";

/** One row, in the shape `RunEntry` has with nothing narrowed against it. */
function entry(
  name: string,
  run: string,
  extra: Partial<RunEntry> = {},
): RunEntry {
  return {
    name,
    run,
    narrows: false,
    requires: [],
    expect_exit_code: 0,
    destructive: false,
    // Nothing in the main checkout is frozen: this is the file Fleet holds.
    frozen: false,
    ...extra,
  };
}

const STORYBOOK: ServerEntry = {
  name: "storybook_dev",
  serve: "pnpm -C packages/components exec storybook dev -p 41207 --no-open --ci",
  ready: "curl -sf http://localhost:41207",
  links: [{ url: "http://localhost:41207", name: "Storybook" }],
  destructive: false,
};

/** This repository's Manifest, as `get_checkout_run_sheet` answers it. */
export function sheet(over: Partial<CheckoutRunSheet> = {}): CheckoutRunSheet {
  return {
    setup: [
      entry("bootstrap", "pnpm install --frozen-lockfile"),
      entry(
        "browsers",
        "pnpm -C packages/components exec playwright install chromium --only-shell",
      ),
    ],
    checks: [
      entry("build", "cargo build --workspace --locked", { narrows: true }),
      entry("test", "cargo nextest run --workspace --exclude acceptance", { narrows: true }),
      entry("typecheck", "pnpm typecheck"),
      entry("bridge_build", "pnpm -C apps/desktop build"),
      entry("storybook", "pnpm -C packages/components build-storybook"),
      entry("bridge_test", "pnpm bridge-test"),
      entry("format", "cargo fmt --all --check", { narrows: true }),
    ],
    commands: [
      entry("fmt", "cargo fmt --all", { destructive: true }),
      entry("gate", "cargo xtask verify-foundations", { expect_exit_code: 0 }),
    ],
    manifest_edited_at: "2026-09-10T09:14:00Z",
    servers: [STORYBOOK],
    ...over,
  };
}

export type Manifesting = {
  /** `GET /manifest/run_sheet`. */
  sheet?: CheckoutRunSheetRead;
  followed?: CheckoutRunFollowed;
  /** What `list_checkout_runs` answers. */
  runs?: CheckoutRunList;
  alwaysAllowed?: AllowedCommandRow[];
  /** What Kit holds, and what this repository has said about each. #1275. */
  kitServers?: KitServerRow[];
  save?: SaveGoesTo;
  diff?: CheckoutRunDiff;
  drift?: ManifestDriftRead;
  edit?: EditGoesTo;
  spend?: ManifestSpend;
  /** Whether the file opens declaring `freeze: true`. */
  frozen?: boolean;
  /** `false` where the repository has no root `armada.yml`. */
  setUp?: boolean;
  onEdits?: (body: EditManifest) => void;
  onStartRun?: (body: StartCheckoutRun) => void;
  /** Pressed Verify. Opening its reading must not reach this — #1383. */
  onStartVerify?: () => void;
  /** `false` leaves the rail on All repositories, which every per-repository surface has a state for. */
  picked?: boolean;
};

/** A connected Fleet serving this repository, picked, with its Manifest as the options say. */
export function manifesting(options: Manifesting = {}): Scenario {
  const one = repository();
  const served = options.setUp === false ? { root: one.root, records_root: one.records_root } : one;
  const base = onBoard([], {
    repositories: [served],
    picked: options.picked === false ? null : served.root,
  });
  return {
    ...base,
    name: "manifest",
    says: "This repository's own Manifest, on a Fleet that saves, edits and runs it",
    behaves: (fleet) => behaviour(fleet, options),
  };
}

function behaviour(fleet: FleetHandle, options: Manifesting): Partial<BridgeApi> {
  const { save = "took", edit = "took", drift = DRIFT_CURRENT, spend = NO_SPEND } = options;
  const runs = options.runs ?? { runs: [], unreadable: [] };
  let disk = MANIFEST_TEXT;
  let declared: ManifestDeclared = options.frozen === true ? { ...DECLARED, freeze: true } : DECLARED;
  let allowed = options.alwaysAllowed ?? [];
  // Kit's own table, and the resolution over both tiers — the mock answers
  // exactly as `fleet::kit` does, because the surface draws `resolves` rather
  // than working it out. #1275.
  let servers: KitServerRow[] = options.kitServers ?? [];
  const file = () => ({ ok: true as const, file: { path: MANIFEST_PATH, text: disk, declared } });
  return {
    readManifestFile: async () => file(),
    editManifest: async (body) => {
      options.onEdits?.(body);
      if (edit === "refused") {
        return {
          state: "refused",
          saying: "armada.yml would not load after these edits: 1 fault",
          faults: [{ key: "checks.typecheck.requires", fault: "names bootstrap, which no Command declares" }],
        };
      }
      declared = appliedTo(declared, body.edits);
      return { state: "edited", edited: { path: MANIFEST_PATH, at: WROTE_AT, text: disk, declared } };
    },
    saveManifestFile: async (body) => {
      if (save === "moved") {
        disk = PULLED_TEXT;
        return { state: "moved", onDisk: PULLED_TEXT };
      }
      disk = body.text;
      // The watch re-reads after the write answers, as Fleet's does.
      queueMicrotask(() =>
        fleet.publish({
          manifestReading:
            save === "refused"
              ? {
                  path: MANIFEST_PATH,
                  at: READ_AT,
                  refused: {
                    summary: "armada.yml could not be adopted: 1 fault",
                    faults: [{ key: "drone.poke_limit", fault: "invalid type: string, expected u32" }],
                  },
                }
              : { path: MANIFEST_PATH, at: READ_AT, moved: [{ key: "drone.poke_limit", before: "3", after: "5" }] },
        }),
      );
      return { state: "saved", saved: { path: MANIFEST_PATH, at: WROTE_AT } };
    },
    readManifestSpend: async () => ({ ok: true, spend }),
    listKitServers: async () => ({ ok: true, kit: { servers } }),
    addKitServer: async (adding) => {
      const added: KitServerRow = {
        name: adding.name,
        address: adding.address,
        // Reaching nobody, which is what `KitServer::added` guarantees.
        drones: "no",
        resolves: false,
        added_at: READ_AT,
        by: "human",
      };
      servers = [...servers.filter((row) => row.name !== adding.name), added].sort((one, two) =>
        one.name.localeCompare(two.name),
      );
      return { ok: true, kit: { servers } };
    },
    forgetKitServer: async (name) => {
      servers = servers.filter((row) => row.name !== name);
      return { ok: true, kit: { servers } };
    },
    setKitServerReach: async (name, drones) => {
      servers = servers.map((row) => (row.name === name ? resolvedRow({ ...row, drones }) : row));
      return { ok: true, kit: { servers } };
    },
    setManifestServerReach: async (name, reach) => {
      servers = servers.map((row) =>
        row.name === name
          ? resolvedRow({ ...row, manifest: reach ?? undefined })
          : row,
      );
      return { ok: true, kit: { servers } };
    },
    listRepositoryAllowedCommands: async () => ({ ok: true, commands: { commands: allowed } }),
    removeRepositoryAllowedCommand: async (run) => {
      allowed = allowed.filter((row) => row.run !== run);
      return { ok: true, commands: { commands: allowed } };
    },
    watchCheckoutRunSheet: async (want) =>
      fleet.publish({ checkoutRunSheet: want ? (options.sheet ?? { state: "read", sheet: sheet() }) : { state: "none" } }),
    observeCheckoutRun: async (runId) =>
      fleet.publish({ checkoutRunFollowed: runId === null ? { state: "none" } : (options.followed ?? { state: "none" }) }),
    watchManifestDrift: async (want) => fleet.publish({ manifestDrift: want ? drift : { state: "none" } }),
    listCheckoutRuns: async () => ({ ok: true, runs }),
    getCheckoutRunOutput: async (runId) => {
      const run = runs.runs.find((one) => one.id === runId);
      return run === undefined
        ? { ok: false, outcome: { ok: false, why: "not_connected" } }
        : {
            ok: true,
            output: { id: run.id, name: run.name, path: `.armada/${run.log}`, lines: [], from_line: 1, total_lines: 0, bytes: 0, whole: true },
          };
    },
    getCheckoutRunDiff: async () =>
      options.diff === undefined ? { ok: false, outcome: { ok: false, why: "not_connected" } } : { ok: true, diff: options.diff },
    startCheckoutRun: async (body): Promise<Outcome> => {
      options.onStartRun?.(body);
      return { ok: true };
    },
    startCheckoutVerify: async (): Promise<Outcome> => {
      options.onStartVerify?.();
      return { ok: true };
    },
  };
}

/**
 * The two tiers, as Fleet answers them. **The Manifest's word wins where it has
 * one**, in either direction; where it has none, Kit's default answers.
 */
function resolvedRow(row: KitServerRow): KitServerRow {
  const resolves =
    row.manifest === undefined ? row.drones === "yes" : row.manifest === "extended";
  return { ...row, resolves };
}
