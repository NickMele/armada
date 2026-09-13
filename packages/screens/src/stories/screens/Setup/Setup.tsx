// The app's Setup view, in the window Bridge draws it in: `Shell`, `Manifest` and `Setup`,
// called the way `App` calls them. Only the data is made up, and a fake Fleet applies each
// edit and Write the way Fleet does, as far as a play test presses them.

import { useRef, useState } from "react";
import { connectedTo, PROTOCOL_VERSION, type Connection } from "@armada/protocol";
import type {
  CheckoutRunSheetRead,
  EditManifestProposal,
  ManifestProposal,
  ManifestProposals,
  Outcome,
  Provenance,
  RepositoryScan,
  RepositorySummary,
  ScannedWorkspace,
} from "@armada/protocol";
import { headOf, Shell, statementOf, SURFACE } from "@armada/shell";

import { Manifest } from "../../../Manifest";
import { useManifestEditing } from "../../../manifest-file";
import { useManifestForm } from "../../../manifest-form";
import { Locate, LocatedNotice, useLocate } from "../../../Locate";
import { landsIn, NO_REPOSITORY, type LocateAnswer } from "../../../locate-reads";
import { Setup } from "../../../Setup";
import { useSetup } from "../../../setup-held";
import type { ProposalAnswer } from "../../../setup-reads";
import { CREATED_AT, repository } from "../../../fixtures/build/base";
import { NOW } from "../Manifest/Manifest";

const CONNECTED: Connection = connectedTo(
  { protocolVersion: PROTOCOL_VERSION, pid: 4242, port: 7878, startedAt: CREATED_AT },
  1,
);
const noop = () => {};
const nothingHappens = (): Promise<Outcome> => Promise.resolve({ ok: true });
const WROTE_AT = "2026-09-12T14:20:03.120Z";

function workspace(dir: string, evidence: string, files: string[]): ScannedWorkspace {
  return {
    dir, declared_by: [], evidence, manifests: files.map((file) => ({ file, tool: "npm" })), lockfiles: [],
    runnables: [], tools: [], services: [], ports: [], missing: [], not_read: [],
  };
}

/** A storefront monorepo nobody set up: a root, two apps, and docs with nothing runnable. */
export const SCAN: RepositoryScan = {
  checkout: "/Users/user/storefront",
  workspaces: [
    workspace(".", "strong", ["package.json", "pnpm-lock.yaml"]),
    workspace("apps/web", "strong", ["apps/web/package.json"]),
    workspace("services/api", "strong", ["services/api/package.json"]),
    workspace("docs", "thin", ["docs/package.json"]),
  ],
  not_read: [],
};

const convention = (file: string, key: string): Provenance => ({ source: "convention", file, key });
const DEFAULTS = [
  { key: "auto_merge", value: "never", provenance: { source: "default" } },
  { key: "review_gate", value: "human_always", provenance: { source: "default" } },
];

function proposal(dir: string, checks: string[], commands: string[] = []): ManifestProposal {
  const file = dir === "." ? "package.json" : `${dir}/package.json`;
  return {
    dir,
    file: dir === "." ? "armada.yml" : `${dir}/armada.yml`,
    id: { value: dir === "." ? "storefront" : dir.split("/").pop()!, provenance: convention(file, "name") },
    ports:
      dir === "services/api"
        ? [{ name: "db", container: 5432, provenance: { source: "read", file: "compose.yaml", key: "services.db.ports" } }]
        : [],
    checks: checks.map((name) => ({ name, run: `pnpm ${name}`, provenance: convention(file, `scripts.${name}`) })),
    commands: commands.map((name) => ({ name, run: `pnpm ${name}`, provenance: convention(file, `scripts.${name}`) })),
    policy: DEFAULTS,
  };
}

export const PROPOSALS: ManifestProposals = {
  checkout: SCAN.checkout,
  proposals: [
    { ...proposal(".", ["test"]), present: true },
    proposal("apps/web", ["test", "lint", "typecheck"], ["dev"]),
    proposal("services/api", ["test", "typecheck"], ["migrate", "reset"]),
    proposal("docs", []),
  ],
  caps: { cost_micros: 5_000_000, turns: 200 },
};

/** One edit, applied as Fleet applies it: a changed line reads edited, a new one added. */
function applied(one: ManifestProposal, { edit }: EditManifestProposal): ManifestProposal {
  const edited = { source: "edited_during_setup" };
  const added = { source: "added_during_setup" };
  switch (edit.edit) {
    case "check": {
      const was = one.checks.some((check) => check.name === edit.name);
      const requires = edit.requires ?? [];
      const line = { name: edit.name, run: edit.run, ...(requires.length === 0 ? {} : { requires }), provenance: was ? edited : added };
      return { ...one, checks: was ? one.checks.map((check) => (check.name === edit.name ? line : check)) : [...one.checks, line] };
    }
    case "command": {
      const was = one.commands.some((command) => command.name === edit.name);
      const line = { name: edit.name, run: edit.run, ...(edit.destructive === true ? { destructive: true } : {}), provenance: was ? edited : added };
      return {
        ...one,
        commands: was ? one.commands.map((command) => (command.name === edit.name ? line : command)) : [...one.commands, line],
      };
    }
    case "move": {
      const check = one.checks.find((line) => line.name === edit.name);
      if (check !== undefined) {
        return {
          ...one,
          checks: one.checks.filter((line) => line !== check),
          commands: [...one.commands, { name: check.name, run: check.run, provenance: edited }],
        };
      }
      const command = one.commands.find((line) => line.name === edit.name)!;
      return {
        ...one,
        commands: one.commands.filter((line) => line !== command),
        checks: [...one.checks, { name: command.name, run: command.run, provenance: edited }],
      };
    }
    case "policy":
      return { ...one, policy: one.policy.map((row) => (row.key === edit.key ? { ...row, value: edit.value ?? row.value, provenance: edited } : row)) };
    default:
      return one;
  }
}

/** The file Write puts down, laid out as Fleet's writer lays out the keys a play test reads. */
function textOf(one: ManifestProposal): string {
  const lines = ["version: 1", `id: ${one.id.value}`];
  if (one.checks.length > 0) lines.push("checks:");
  for (const check of one.checks) {
    lines.push(`  ${check.name}:`, `    run: ${check.run}`);
    if (check.requires !== undefined && check.requires.length > 0) {
      lines.push("    requires:", ...check.requires.map((name) => `      - ${name}`));
    }
  }
  if (one.commands.length > 0) lines.push("commands:");
  for (const command of one.commands) {
    lines.push(`  ${command.name}:`, `    run: ${command.run}`);
    if (command.destructive === true) lines.push("    destructive: true");
  }
  return `${lines.join("\n")}\n`;
}

/** What Write comes to: the file lands, the parser refuses it, or a file is already there. */
export type WriteGoesTo = "took" | "refused" | "appeared";

/** A folder added by path, with no `armada.yml`: what the rail's picker lists beside a Fleet's own. */
export const SCRATCH: RepositorySummary = { root: "/Users/user/scratch", records_root: "/Users/user/Library/Application Support/Armada/records/scratch" };

const NO_ROOT_FILE: ManifestProposals = {
  ...PROPOSALS,
  proposals: PROPOSALS.proposals.map((one) => (one.dir === "." ? { ...one, present: false } : one)),
};

/** What a clone comes to in a story: served, refused by git, into a folder already full, still running, or served late. */
export type CloneGoesTo = "took" | "refused" | "occupied" | "underway" | "late";

/** How long a late clone runs: long enough for a play to close its dialog first. */
const LATE_MS = 800;

/** What main resolves a parent to. `/tmp` is a symlink on macOS, and Fleet clones under its target. */
const RESOLVED: Record<string, string> = { "/tmp": "/private/tmp" };

/** What Fleet answers a per-repository read with while it serves none. */
const NOTHING_SERVED: CheckoutRunSheetRead = {
  state: "failed",
  outcome: {
    ok: false,
    why: "refused",
    error: { code: NO_REPOSITORY, message: "No repository has a Manifest yet, so add one by folder or clone one from its URL", run_id: "01M1RUN", fields: {}, chain: [] },
  },
};

/** What the OS folder dialog answers in a story. Nothing native opens. */
export const CHOSEN = "/Users/user/scratch";

/** `more` packages beside the storefront's own four, each proposing the apps' three Checks. */
function widened(base: ManifestProposals, more: number): { scan: RepositoryScan; proposals: ManifestProposals } {
  const dirs = Array.from({ length: more }, (_, at) => `packages/pkg-${String(at + 1).padStart(2, "0")}`);
  return {
    scan: { ...SCAN, workspaces: [...SCAN.workspaces, ...dirs.map((dir) => workspace(dir, "strong", [`${dir}/package.json`]))] },
    proposals: { ...base, proposals: [...base.proposals, ...dirs.map((dir) => proposal(dir, ["test", "lint", "typecheck"]))] },
  };
}

export function SetupFrom({
  write = "took",
  sheet = { state: "none" },
  rootSetUp = true,
  repositories,
  more = 0,
  clone = "took",
  onAdded,
  onCloned,
  onWritten,
  onVerify,
}: {
  clone?: CloneGoesTo;
  /** Called on every add and every clone Locate sends. */
  onAdded?: (path: string) => void;
  onCloned?: (url: string, parent: string) => void;
  /** Called with the file and its text on every Write that lands. */
  onWritten?: (file: string, text: string) => void;
  /** Called with the workspace each Verify press on a sheet sends. */
  onVerify?: (workspace?: string) => void;
  write?: WriteGoesTo;
  /** Whether the root already has an `armada.yml`, as any Fleet's own repository does. */
  rootSetUp?: boolean;
  /** The Manifest Fleet holds, which a Verify after a root Write reads. */
  sheet?: CheckoutRunSheetRead;
  /**
   * What the rail's picker lists, the first picked. Picking one nobody set up opens its Setup, and
   * a root Write there gives it a Manifest, as Fleet's re-read list would. Absent is this Fleet's own.
   */
  repositories?: RepositorySummary[];
  /** Workspaces beyond the storefront's four, for a batch taller than the window. */
  more?: number;
}) {
  const [listed, setListed] = useState<RepositorySummary[]>(repositories ?? [repository()]);
  const [scope, setScope] = useState(listed[0]?.root ?? "");
  const picked = listed.find((one) => one.root === scope);
  // Fleet serving a folder: listed, and answered. Main's pick is `onLocated` below.
  const served = (root: string): Promise<LocateAnswer> => {
    const one = { root, records_root: `/records/${root.split("/").pop()}` };
    setListed((was) => [...was, one]);
    return Promise.resolve({ state: "located", repository: one });
  };
  const locate = useLocate({
    onChooseFolder: () => Promise.resolve(CHOSEN),
    onResolveFolder: (path) => Promise.resolve(RESOLVED[path] ?? path),
    nothingServed: listed.length === 0,
    onAdd: (path) => {
      onAdded?.(path);
      return served(path);
    },
    onClone: (url, parent) => {
      onCloned?.(url, parent);
      const into = landsIn(url, RESOLVED[parent] ?? parent)!;
      if (clone === "underway") return new Promise(() => {});
      if (clone === "late") return new Promise((done) => setTimeout(() => done(served(into)), LATE_MS));
      if (clone === "refused") {
        const saying = `git refused the clone: fatal: repository '${url}' not found`;
        return Promise.resolve({ state: "refused", code: "fleet.clone_refused", saying });
      }
      if (clone === "occupied") {
        return Promise.resolve({ state: "refused", code: "fleet.destination_occupied", saying: `${into} already exists and is not empty` });
      }
      return served(into);
    },
    onLocated: (one) => {
      held.current = widened(NO_ROOT_FILE, more).proposals;
      setScope(one.root);
      setSettingUp(true);
    },
  });
  const held = useRef<ManifestProposals>(widened(rootSetUp && picked?.manifest !== undefined ? PROPOSALS : NO_ROOT_FILE, more).proposals);
  const [settingUp, setSettingUp] = useState(repositories === undefined || picked?.manifest === undefined);
  const answer = (dir: string, change: (one: ManifestProposal) => ManifestProposal): Promise<ProposalAnswer> => {
    const next = held.current.proposals.map((one) => (one.dir === dir ? change(one) : one));
    held.current = { ...held.current, proposals: next };
    return Promise.resolve({ state: "took", proposal: next.find((one) => one.dir === dir)! });
  };
  const setting = useSetup({
    showing: settingUp,
    onReadScan: () => Promise.resolve({ ok: true, scan: widened(held.current, more).scan }),
    onReadProposals: () => Promise.resolve({ ok: true, proposals: held.current }),
    onEditProposal: (body) => answer(body.dir, (one) => applied(one, body)),
    repository: scope,
    onWriteProposal: ({ dir }) => {
      if (write === "refused") {
        return Promise.resolve({
          state: "refused",
          saying: `${dir}/armada.yml would not load: 1 fault`,
          faults: [{ key: "checks.lint.run", fault: "is empty." }],
        });
      }
      if (write === "appeared") return Promise.resolve({ state: "appeared", onDisk: "version: 1\nid: web\n" });
      const file = dir === "." ? "armada.yml" : `${dir}/armada.yml`;
      if (dir === ".") {
        const setUp = { ...manifestOf(picked!), id: "scratch" };
        setListed((was) => was.map((one) => (one.root === scope ? { ...one, manifest: setUp } : one)));
      }
      return answer(dir, (one) => {
        const text = textOf(one);
        onWritten?.(file, text);
        return { ...one, text, written: { path: file, at: WROTE_AT } };
      });
    },
  });

  // Main reads a repository with no Manifest by its root, so a workspace's Verify reads the sheet as well.
  const verifiable: CheckoutRunSheetRead = picked === undefined ? NOTHING_SERVED : sheet;

  // The Manifest surface's other views, faked only far enough to mount.
  const readFile = () => Promise.resolve({ ok: false as const, outcome: { ok: false as const, why: "not_connected" as const } });
  const editing = useManifestEditing({ showing: false, reading: null, onReadFile: readFile, onSaveFile: () => Promise.resolve({ state: "failed", outcome: { ok: false, why: "not_connected" } }) });
  const form = useManifestForm({
    showing: false,
    reading: null,
    onReadFile: readFile,
    onEditManifest: () => Promise.resolve({ state: "failed", outcome: { ok: false, why: "not_connected" } }),
    onReadSpend: () => Promise.resolve({ ok: false, outcome: { ok: false, why: "not_connected" } }),
  });
  const head = headOf({
    reading: false, composing: false, auditing: false, clearing: false, manifest: editing.view, live: true,
    refreshing: false, onCloseComposer: noop, onCompose: noop, onCloseReports: noop, onReadReports: noop,
    onCloseWorktrees: noop, onReadWorktrees: noop, onOpenLimits: noop, onRefresh: noop, jobs: [],
    onClearTerminal: noop, onForgetTerminal: noop,
  });

  return (
    <div style={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <Shell
        connection={CONNECTED}
        statement={statementOf(CONNECTED, NOW, NOW)}
        repositories={listed}
        listed
        scope={scope}
        onScope={(root) => {
          // All repositories asks which Manifest in `App`; this story draws one repository's.
          if (root === null) return;
          // `App`'s own rule: a repository nobody set up opens on Setup.
          const to = listed.find((one) => one.root === root)!;
          if (to.manifest === undefined) held.current = widened(NO_ROOT_FILE, more).proposals;
          setScope(root);
          setSettingUp(to.manifest === undefined || settingUp);
        }}
        jobs={[]}
        boardJobs={[]}
        capacity={{ bound: 4, occupied: 0 }}
        title={head?.title}
        summary="Set up a Manifest for each workspace in this checkout. Write puts one file down and stops, without staging or committing it."
        actions={head?.actions}
        showing={SURFACE.manifest}
        onOpenLimits={noop}
        onAddRepository={locate.onOpen}
      >
        <div className="armada-screen__mounted">
          <LocatedNotice locating={locate} repositories={listed} />
          <Manifest
            key={scope}
            sheet={verifiable}
            followed={{ state: "none" }}
            picked={null}
            now={NOW}
            editing={editing}
            form={form}
            onSaid={noop}
            onObserveRun={noop}
            onStartRun={nothingHappens}
            drift={{ state: "none" }}
            onStartVerify={nothingHappens}
            onStopRun={nothingHappens}
            onUndoRun={nothingHappens}
            onListRuns={() => Promise.resolve({ ok: true, runs: { runs: [], unreadable: [] } })}
            onGetRunOutput={() => Promise.resolve({ ok: false, outcome: { ok: false, why: "not_connected" } })}
            onGetRunDiff={() => Promise.resolve({ ok: false, outcome: { ok: false, why: "not_connected" } })}
            onListRepositoryAllowedCommands={() => Promise.resolve({ ok: true, commands: { commands: [] } })}
            onRemoveRepositoryAllowedCommand={() => Promise.resolve({ ok: true, commands: { commands: [] } })}
            onStartServer={nothingHappens}
            onStopServer={nothingHappens}
            onOpenServerLink={() => Promise.resolve({ ok: false, why: "no_address" } as const)}
            settingUp={settingUp}
            onSettingUp={setSettingUp}
            setUp={picked?.manifest !== undefined}
            setup={
              <Setup
                setting={setting}
                now={NOW}
                sheet={verifiable}
                onStartVerify={(workspace) => {
                  onVerify?.(workspace);
                  return nothingHappens();
                }}
                onStopRun={nothingHappens}
                onOpenEdit={() => {
                  setSettingUp(false);
                  editing.onView("form");
                }}
              />
            }
          />
        </div>
      </Shell>
      <Locate locating={locate} />
    </div>
  );
}

function manifestOf(one: RepositorySummary) {
  const base = repository().manifest!;
  return { ...base, repository: one.root.split("/").pop()!, path: `${one.root}/armada.yml`, records_root: one.records_root };
}
