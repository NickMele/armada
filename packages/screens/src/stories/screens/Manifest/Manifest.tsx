// The app's Manifest surface, in the window Bridge draws it in.
//
// **Not a drawing of the surface — the surface.** `Shell`, `headOf` and
// `Manifest` are the three the app renders, called the way `App` calls them,
// so the rail, the head and every row come out of the app's own code. What is
// made up is only the data, and it is typed against the wire.

import { useRef, useState } from "react";
import { connectedTo, PROTOCOL_VERSION, type Connection } from "@armada/protocol";
import type {
  CheckoutRunDiff,
  CheckoutRunDiffRead,
  AllowedCommandRow,
  CheckoutRunFollowed,
  CheckoutRunList,
  CheckoutRunSheet,
  CheckoutRunSheetRead,
  CheckoutRunUnderway,
  CheckoutVerify,
  EditManifest,
  ManifestDeclared,
  ManifestDriftRead,
  ManifestSpend,
  ManifestReading,
  Outcome,
  RunEntry,
  RunOutputRead,
  SaveManifestFile,
  StartCheckoutRun,
  ServerEntry,
  VerifyStep,
} from "@armada/protocol";
import { headOf, Shell, statementOf, SURFACE } from "@armada/shell";
import { Manifest } from "../../../Manifest";
import type { ManifestEditAnswer, ManifestSaveAnswer, ManifestView } from "../../../editing";
import type { RepositoryAllowedCommandsRead } from "../../../manifest-allows";
import { useManifestEditing } from "../../../manifest-file";
import { useManifestForm } from "../../../manifest-form";
import { appliedTo } from "../ManifestForms/applied";
import { CREATED_AT, repository } from "../../../fixtures/build/base";

/** The moment every elapsed figure on this page is read at, so it never moves. */
export const NOW = Date.parse("2026-09-12T14:20:00Z");

/** A Fleet that answered, as Bridge holds one once the socket is up. */
const CONNECTED: Connection = connectedTo(
  { protocolVersion: PROTOCOL_VERSION, pid: 4242, port: 7878, startedAt: CREATED_AT },
  1,
);

const noop = () => {};
const nothingHappens = (): Promise<Outcome> => Promise.resolve({ ok: true });

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
 * What a save comes to, played the way Fleet plays it: the write answers,
 * then the watch re-reads and publishes. `refused` is a correction Fleet would
 * not adopt; `moved` is a pull that landed under the edit.
 */
export type SaveGoesTo = "took" | "refused" | "moved";

/** The instant a save in a story lands, and the reading that follows it. */
const WROTE_AT = "2026-09-12T14:20:03.120Z";
const READ_AT = "2026-09-12T14:20:04.700Z";

export function ManifestFrom({
  sheet,
  followed = { state: "none" },
  runs = { runs: [], unreadable: [] },
  alwaysAllowed = [],
  now = NOW,
  view = "run",
  save = "took",
  diff,
  drift = DRIFT_CURRENT,
  edit = "took",
  spend = NO_SPEND,
  frozen = false,
  onEdits,
  setUp,
  onStartRun = nothingHappens,
}: {
  /** `false` where the repository has no root `armada.yml`. */
  setUp?: boolean;
  /** What pressing Run sends, for a play to read. */
  onStartRun?: (body: StartCheckoutRun) => Promise<Outcome>;
  /** `GET /manifest/drift`. Every line current, unless a story says otherwise. */
  drift?: ManifestDriftRead;
  sheet: CheckoutRunSheetRead;
  followed?: CheckoutRunFollowed;
  /** What `list_checkout_runs` answers — *Earlier runs*, and what Undo acts on. */
  runs?: CheckoutRunList;
  /** What `get_repository_allowed_commands` answers, oldest first. */
  alwaysAllowed?: AllowedCommandRow[];
  now?: number;
  /** Which view the surface opens on. */
  view?: ManifestView;
  /** What pressing Save comes to. */
  save?: SaveGoesTo;
  /** What `get_checkout_run_diff` answers when *Open the diff* is pressed. Absent: not connected. */
  diff?: CheckoutRunDiff;
  /** What pressing Save on the forms comes to. */
  edit?: EditGoesTo;
  /** What `get_manifest_spend` answers. */
  spend?: ManifestSpend;
  /** Whether the file opens declaring `freeze: true`. */
  frozen?: boolean;
  /** What a Save on the forms sends, for a play to read. */
  onEdits?: (body: EditManifest) => void;
}) {
  // Fleet's own table, faked just far enough to answer both routes: a read
  // returns what is held, and a remove takes a row out and answers the rest.
  const [allowed, setAllowed] = useState(alwaysAllowed);
  const onListRepositoryAllowedCommands = (): Promise<RepositoryAllowedCommandsRead> =>
    Promise.resolve({ ok: true, commands: { commands: allowed } });
  const onRemoveRepositoryAllowedCommand = (run: string): Promise<RepositoryAllowedCommandsRead> => {
    const left = allowed.filter((row) => row.run !== run);
    setAllowed(left);
    return Promise.resolve({ ok: true, commands: { commands: left } });
  };

  // A disk and a watch, faked just far enough to be Fleet's: a read answers
  // what is on disk, a save compares against it, and a reading follows.
  const disk = useRef(MANIFEST_TEXT);
  const declared = useRef(frozen ? { ...DECLARED, freeze: true } : DECLARED);
  const [reading, setReading] = useState<ManifestReading | null>(null);
  const readFile = () =>
    Promise.resolve({
      ok: true as const,
      file: { path: MANIFEST_PATH, text: disk.current, declared: declared.current },
    });
  const form = useManifestForm({
    showing: true,
    reading,
    onReadFile: readFile,
    onEditManifest: (body: EditManifest): Promise<ManifestEditAnswer> => {
      onEdits?.(body);
      if (edit === "refused") {
        return Promise.resolve({
          state: "refused",
          saying: "armada.yml would not load after these edits: 1 fault",
          faults: [{ key: "checks.typecheck.requires", fault: "names bootstrap, which no Command declares" }],
        });
      }
      declared.current = appliedTo(declared.current, body.edits);
      return Promise.resolve({
        state: "edited",
        edited: { path: MANIFEST_PATH, at: WROTE_AT, text: disk.current, declared: declared.current },
      });
    },
    onReadSpend: () => Promise.resolve({ ok: true, spend }),
  });
  const editing = useManifestEditing({
    showing: true,
    reading,
    initialView: view,
    onReadFile: readFile,
    onSaveFile: (body: SaveManifestFile): Promise<ManifestSaveAnswer> => {
      if (save === "moved") {
        disk.current = PULLED_TEXT;
        return Promise.resolve({ state: "moved", onDisk: PULLED_TEXT });
      }
      disk.current = body.text;
      setReading(
        save === "refused"
          ? {
              path: MANIFEST_PATH,
              at: READ_AT,
              refused: {
                summary: "armada.yml could not be adopted: 1 fault",
                faults: [{ key: "drone.poke_limit", fault: "invalid type: string, expected u32" }],
              },
            }
          : {
              path: MANIFEST_PATH,
              at: READ_AT,
              moved: [{ key: "drone.poke_limit", before: "3", after: "5" }],
            },
      );
      return Promise.resolve({ state: "saved", saved: { path: MANIFEST_PATH, at: WROTE_AT } });
    },
  });
  const statement = statementOf(CONNECTED, now, now);
  const head = headOf({
    reading: false,
    composing: false,
    auditing: false,
    clearing: false,
    // The view the surface is showing, as `App` passes it.
    manifest: editing.view,
    live: true,
    refreshing: false,
    onCloseComposer: noop,
    onCompose: noop,
    onCloseReports: noop,
    onReadReports: noop,
    onCloseWorktrees: noop,
    onReadWorktrees: noop,
    onOpenLimits: noop,
    onRefresh: noop,
    jobs: [],
    onClearTerminal: noop,
    onForgetTerminal: noop,
    sweeping: null,
  });
  return (
    <div style={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <Shell
        connection={CONNECTED}
        repositories={[repository()]}
        scope={repository().root}
        onScope={noop}
        onCompose={noop}
        onSearch={noop}
        boardJobs={[]}
        stats={{
          rows: [
            { id: "approval", label: "Awaiting approval", value: 0 },
            { id: "review", label: "Needs review", value: 0 },
            { id: "escalated", label: "Escalated", value: 0 },
            { id: "jobs", label: "Jobs", value: 0 },
            { id: "drones", label: "Drones", value: "0 of 4" },
            { id: "manifest", label: "Manifest", value: "Current" },
          ],
          open: true,
          onOpenChange: noop,
        }}
        fleet={{ state: "running", label: "Running", detail: statement.detail, open: true, onOpenChange: noop }}
        title={head?.title}
        summary={head?.summary}
        actions={head?.actions}
        showing={SURFACE.manifest}
      >
        <div className="armada-screen__mounted">
          <Manifest
            sheet={sheet}
            followed={followed}
            picked={null}
            now={now}
            editing={editing}
            form={form}
            onSaid={noop}
            onObserveRun={noop}
            onStartRun={onStartRun}
            {...(setUp === undefined
              ? {}
              : { setUp, setup: <p className="text-fg-muted">Screens/Setup draws this tab.</p> })}
            drift={drift}
            onStartVerify={nothingHappens}
            onStopRun={nothingHappens}
            onUndoRun={nothingHappens}
            onListRuns={() => Promise.resolve({ ok: true, runs })}
            // Any earlier run's log reads, so a story can open one from *Earlier runs*.
            onGetRunOutput={(runId): Promise<RunOutputRead> => {
              const run = runs.runs.find((one) => one.id === runId);
              return Promise.resolve(
                run === undefined
                  ? { ok: false, outcome: { ok: false, why: "not_connected" } }
                  : {
                      ok: true,
                      output: {
                        id: run.id,
                        name: run.name,
                        path: `.armada/${run.log}`,
                        lines: [],
                        from_line: 1,
                        total_lines: 0,
                        bytes: 0,
                        whole: true,
                      },
                    },
              );
            }}
            onGetRunDiff={(): Promise<CheckoutRunDiffRead> =>
              Promise.resolve(
                diff === undefined
                  ? { ok: false, outcome: { ok: false, why: "not_connected" } }
                  : { ok: true, diff },
              )
            }
            onListRepositoryAllowedCommands={onListRepositoryAllowedCommands}
            onRemoveRepositoryAllowedCommand={onRemoveRepositoryAllowedCommand}
            onStartServer={nothingHappens}
            onStopServer={nothingHappens}
            onOpenServerLink={() => Promise.resolve({ ok: true })}
          />
        </div>
      </Shell>
    </div>
  );
}

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
