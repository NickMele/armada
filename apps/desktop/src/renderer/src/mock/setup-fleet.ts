// A Fleet far enough along to set a repository up: a storefront monorepo nobody
// set up, the proposals Scan makes for it, each edit and Write applied the way
// Fleet applies them, and a folder added or cloned. Moved here from the
// `Screens/Setup` story's wrapper, so Setup and Locate run through `App` — #1224.

import type {
  CheckoutRunSheetRead,
  CheckoutVerify,
  EditManifestProposal,
  ManifestProposal,
  ManifestProposals,
  Provenance,
  RepositoryScan,
  RepositorySummary,
  ScannedWorkspace,
  VerifyStep,
} from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { landsIn, type LocateAnswer } from "@armada/screens/src/locate-reads";
import type { ProposalAnswer } from "@armada/screens/src/setup-reads";

import type { BridgeApi } from "../../../shared/api";
import { onBoard } from "./scenario";
import type { FleetHandle, Scenario } from "./scenario";

const OK = { ok: true } as const;
const WROTE_AT = "2026-09-12T14:20:03.120Z";

function workspace(dir: string, evidence: string, files: string[]): ScannedWorkspace {
  return {
    dir, declared_by: [], evidence, manifests: files.map((file) => ({ file, tool: "npm" })), lockfiles: [],
    runnables: [], tools: [], services: [], ports: [], missing: [], not_read: [],
  };
}

/** A storefront monorepo nobody set up: a root, two apps, and docs with nothing runnable. */
const SCAN: RepositoryScan = {
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

const PROPOSALS: ManifestProposals = {
  checkout: SCAN.checkout,
  proposals: [
    { ...proposal(".", ["test"]), present: true },
    proposal("apps/web", ["test", "lint", "typecheck"], ["dev"]),
    proposal("services/api", ["test", "typecheck"], ["migrate", "reset"]),
    proposal("docs", []),
  ],
  caps: { cost_micros: 5_000_000, turns: 200 },
};

const NO_ROOT_FILE: ManifestProposals = {
  ...PROPOSALS,
  proposals: PROPOSALS.proposals.map((one) => (one.dir === "." ? { ...one, present: false } : one)),
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

/** The file Write puts down, laid out as Fleet's writer lays out the keys a test reads. */
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

/** `more` packages beside the storefront's own four, each proposing the apps' three Checks. */
function widened(base: ManifestProposals, more: number): { scan: RepositoryScan; proposals: ManifestProposals } {
  const dirs = Array.from({ length: more }, (_, at) => `packages/pkg-${String(at + 1).padStart(2, "0")}`);
  return {
    scan: { ...SCAN, workspaces: [...SCAN.workspaces, ...dirs.map((dir) => workspace(dir, "strong", [`${dir}/package.json`]))] },
    proposals: { ...base, proposals: [...base.proposals, ...dirs.map((dir) => proposal(dir, ["test", "lint", "typecheck"]))] },
  };
}

function ran(group: string, name: string, run: string, exit: number, ms: number): VerifyStep {
  const record = {
    id: `crun_${name}`, name, command: run, required: [],
    started_at: "2026-09-12T14:11:00Z", ended_at: "2026-09-12T14:11:09Z", duration_ms: ms,
    exit_code: exit, expect_exit_code: 0, ended: `exited ${exit}`, stopped: false,
    changed: [], undoable: false, log: `runs/crun_${name}/output.log`,
  };
  return { group, name, run, state: "ran", record };
}

/** A Verify that ran all four of its steps, one of them failing. */
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

/** An empty Verify sheet, read: what Fleet holds for a Manifest nothing has run against. */
export const SHEET_READ: CheckoutRunSheetRead = { state: "read", sheet: { setup: [], checks: [], commands: [] } };

/** What the OS folder dialog answers. Nothing native opens. */
export const CHOSEN = "/Users/user/scratch";

/** A folder added by path, with no `armada.yml`. */
export const SCRATCH: RepositorySummary = {
  root: "/Users/user/scratch",
  records_root: "/Users/user/Library/Application Support/Armada/records/scratch",
};

/** What Write comes to: the file lands, the parser refuses it, or a file is already there. */
export type WriteGoesTo = "took" | "refused" | "appeared";

/** What a clone comes to: served, refused by git, into a folder already full, still running, or served late. */
export type CloneGoesTo = "took" | "refused" | "occupied" | "underway" | "late";

/** How long a late clone runs: long enough for a test to close its dialog first. */
const LATE_MS = 800;

/** What main resolves a parent to. `/tmp` is a symlink on macOS, and Fleet clones under its target. */
const RESOLVED: Record<string, string> = { "/tmp": "/private/tmp" };

export type SettingUp = {
  write?: WriteGoesTo;
  clone?: CloneGoesTo;
  /** Whether a set-up repository's root already has an `armada.yml`. */
  rootSetUp?: boolean;
  /** Workspaces beyond the storefront's four, for a batch taller than the window. */
  more?: number;
  /** The Verify sheet Fleet holds for the picked repository. */
  sheet?: CheckoutRunSheetRead;
  /** What the rail lists, the first picked. */
  repositories?: RepositorySummary[];
  onWritten?: (file: string, text: string) => void;
  onVerify?: (workspace?: string) => void;
  onAdded?: (path: string) => void;
  onCloned?: (url: string, parent: string) => void;
};

/** A connected Fleet with no Jobs, answering Setup and Locate as Fleet does. */
export function settingUp(options: SettingUp = {}): Scenario {
  const { write = "took", clone = "took", rootSetUp = true, more = 0, sheet = { state: "none" } } = options;
  const repositories = options.repositories ?? [repository()];
  const base = onBoard([], { repositories, picked: repositories[0]?.root ?? null });
  return {
    ...base,
    name: "setting-up",
    says: "A storefront nobody set up, and a Fleet that applies each edit and Write",
    behaves: (fleet) => behaviour(fleet, { write, clone, rootSetUp, more, sheet, ...options }),
  };
}

function behaviour(
  fleet: FleetHandle,
  options: SettingUp & Required<Pick<SettingUp, "write" | "clone" | "rootSetUp" | "more" | "sheet">>,
): Partial<BridgeApi> {
  const listed = (): RepositorySummary[] => fleet.state().holds.repositories ?? [];
  const picked = () => listed().find((one) => one.root === fleet.state().repository);
  // Each repository's proposals, as Fleet holds them between presses.
  const held = new Map<string, ManifestProposals>();
  const proposalsOf = (root: string): ManifestProposals => {
    const known = held.get(root);
    if (known !== undefined) return known;
    const one = listed().find((repo) => repo.root === root);
    const fresh = widened(options.rootSetUp && one?.manifest !== undefined ? PROPOSALS : NO_ROOT_FILE, options.more).proposals;
    held.set(root, fresh);
    return fresh;
  };
  const answer = (dir: string, change: (one: ManifestProposal) => ManifestProposal): ProposalAnswer => {
    const root = fleet.state().repository ?? "";
    const was = proposalsOf(root);
    const next = was.proposals.map((one) => (one.dir === dir ? change(one) : one));
    held.set(root, { ...was, proposals: next });
    return { state: "took", proposal: next.find((one) => one.dir === dir)! };
  };
  // Fleet serving a folder: listed, published to every window, and answered.
  const served = (root: string, cloned = false): LocateAnswer => {
    const one = { root, records_root: `/records/${root.split("/").pop()}` };
    if (!listed().some((repo) => repo.root === root)) {
      fleet.publish({ holds: { ...fleet.state().holds, repositories: [...listed(), one] } });
    }
    held.set(root, widened(NO_ROOT_FILE, options.more).proposals);
    if (cloned) fleet.publish({ located: { repository: one, at: Date.now() } });
    return { state: "located", repository: one };
  };
  return {
    readRepositoryScan: async () => ({ ok: true, scan: widened(proposalsOf(fleet.state().repository ?? ""), options.more).scan }),
    readManifestProposals: async () => ({ ok: true, proposals: proposalsOf(fleet.state().repository ?? "") }),
    editManifestProposal: async (body) => answer(body.dir, (one) => applied(one, body)),
    writeManifestProposal: async ({ dir }) => {
      if (options.write === "refused") {
        return {
          state: "refused",
          saying: `${dir}/armada.yml would not load: 1 fault`,
          faults: [{ key: "checks.lint.run", fault: "is empty." }],
        };
      }
      if (options.write === "appeared") return { state: "appeared", onDisk: "version: 1\nid: web\n" };
      const file = dir === "." ? "armada.yml" : `${dir}/armada.yml`;
      const root = picked();
      if (dir === "." && root !== undefined) {
        const manifest = {
          ...repository().manifest!,
          id: "scratch",
          repository: root.root.split("/").pop()!,
          path: `${root.root}/armada.yml`,
          records_root: root.records_root,
        };
        fleet.publish({
          holds: {
            ...fleet.state().holds,
            repositories: listed().map((one) => (one.root === root.root ? { ...one, manifest } : one)),
          },
        });
      }
      return answer(dir, (one) => {
        const text = textOf(one);
        options.onWritten?.(file, text);
        return { ...one, text, written: { path: file, at: WROTE_AT } };
      });
    },
    watchCheckoutRunSheet: async (want) => fleet.publish({ checkoutRunSheet: want ? options.sheet : { state: "none" } }),
    startCheckoutVerify: async (workspace) => {
      options.onVerify?.(workspace);
      return OK;
    },
    chooseFolder: async () => CHOSEN,
    resolveFolder: async (path) => RESOLVED[path] ?? path,
    addRepository: async (path) => {
      options.onAdded?.(path);
      return served(path);
    },
    cloneRepository: (url, parent) => {
      options.onCloned?.(url, parent);
      const into = landsIn(url, RESOLVED[parent] ?? parent)!;
      if (options.clone === "underway") return new Promise(() => {});
      if (options.clone === "late") return new Promise((done) => setTimeout(() => done(served(into, true)), LATE_MS));
      if (options.clone === "refused") {
        const saying = `git refused the clone: fatal: repository '${url}' not found`;
        return Promise.resolve({ state: "refused", code: "fleet.clone_refused", saying });
      }
      if (options.clone === "occupied") {
        return Promise.resolve({ state: "refused", code: "fleet.destination_occupied", saying: `${into} already exists and is not empty` });
      }
      return Promise.resolve(served(into, true));
    },
  };
}
