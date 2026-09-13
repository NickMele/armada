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
  ScannedWorkspace,
} from "@armada/protocol";
import { headOf, Shell, statementOf, SURFACE } from "@armada/shell";

import { Manifest } from "../../../Manifest";
import { useManifestEditing } from "../../../manifest-file";
import { useManifestForm } from "../../../manifest-form";
import { Setup } from "../../../Setup";
import { useSetup } from "../../../setup-held";
import type { ProposalAnswer } from "../../../setup-reads";
import { CREATED_AT, manifest, MANIFEST_ID } from "../../../fixtures/build/base";
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
        ? [{ name: "db", container: 5432, provenance: { source: "read", file: "docker-compose.yml", key: "services.db.ports" } }]
        : [],
    checks: checks.map((name) => ({ name, run: `pnpm ${name}`, provenance: convention(file, `scripts.${name}`) })),
    commands: commands.map((name) => ({ name, run: `pnpm ${name}`, provenance: convention(file, `scripts.${name}`) })),
    policy: DEFAULTS,
  };
}

export const PROPOSALS: ManifestProposals = {
  checkout: SCAN.checkout,
  proposals: [
    proposal(".", ["test"]),
    proposal("apps/web", ["test", "lint", "typecheck"], ["dev"]),
    proposal("services/api", ["test", "typecheck"], ["migrate"]),
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
      const line = { name: edit.name, run: edit.run, provenance: was ? edited : added };
      return { ...one, checks: was ? one.checks.map((check) => (check.name === edit.name ? line : check)) : [...one.checks, line] };
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

/** What Write comes to: the file lands, the parser refuses it, or a file is already there. */
export type WriteGoesTo = "took" | "refused" | "appeared";

export function SetupFrom({
  write = "took",
  sheet = { state: "none" },
}: {
  write?: WriteGoesTo;
  /** The Manifest Fleet holds, which a Verify after a root Write reads. */
  sheet?: CheckoutRunSheetRead;
}) {
  const held = useRef(PROPOSALS);
  const [settingUp, setSettingUp] = useState(true);
  const answer = (dir: string, change: (one: ManifestProposal) => ManifestProposal): Promise<ProposalAnswer> => {
    const next = held.current.proposals.map((one) => (one.dir === dir ? change(one) : one));
    held.current = { ...held.current, proposals: next };
    return Promise.resolve({ state: "took", proposal: next.find((one) => one.dir === dir)! });
  };
  const setting = useSetup({
    showing: settingUp,
    onReadScan: () => Promise.resolve({ ok: true, scan: SCAN }),
    onReadProposals: () => Promise.resolve({ ok: true, proposals: held.current }),
    onEditProposal: (body) => answer(body.dir, (one) => applied(one, body)),
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
      return answer(dir, (one) => ({ ...one, written: { path: file, at: WROTE_AT } }));
    },
  });

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
        manifests={[manifest()]}
        scope={MANIFEST_ID}
        onScope={noop}
        jobs={[]}
        capacity={{ bound: 4, occupied: 0 }}
        title={head?.title}
        summary="Set up a Manifest for each workspace in this checkout. Write puts one file down and stops, without staging or committing it."
        actions={head?.actions}
        showing={SURFACE.manifest}
        onOpenLimits={noop}
      >
        <div className="armada-screen__mounted">
          <Manifest
            sheet={sheet}
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
            setup={
              <Setup setting={setting} now={NOW} sheet={sheet} onStartVerify={nothingHappens} onStopRun={nothingHappens} />
            }
          />
        </div>
      </Shell>
    </div>
  );
}
