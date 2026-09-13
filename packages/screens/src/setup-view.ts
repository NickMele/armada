// Setup's arithmetic: what the picker ticks and says, the Check-names grid over the ticked
// batch, the sheet a proposal draws, and how an edit's or a Write's answer folds in.
// No React, so every rule here is a unit test.

import type { ManifestProposal, ManifestProposals, Outcome, Provenance, RepositoryScan, StatedCaps } from "@armada/protocol";
import type {
  ProposalPolicy,
  ProposalSheetProps,
  ProvenanceProps,
  SetupGridCell,
  SetupPickerProps,
} from "@armada/components";

import { clockOf } from "./duration";
import type { ProposalAnswer } from "./setup-reads";

/** What the last edit or Write on one workspace came to, where it did not simply take. */
export type SetupMark = {
  problem?: string;
  refused?: { saying: string; faults: { key: string; fault: string }[] };
  appeared?: { onDisk: string | null };
};

export type SetupOpen = {
  state: "open";
  scan: RepositoryScan;
  proposals: ManifestProposals;
  /** Whether each workspace is in the batch. Strong evidence starts ticked. */
  ticks: Record<string, boolean>;
  /** The workspace whose proposal is over the picker. */
  open: string | null;
  marks: Record<string, SetupMark>;
  /** The workspace an edit or a Write is out for. */
  busy: string | null;
};

export type SetupHeld = { state: "reading" } | { state: "failed"; saying: string } | SetupOpen;

/** A read folded in. Ticks a person set survive it; a workspace new to the read ticks by evidence. */
export function readInto(was: SetupHeld, scan: RepositoryScan, proposals: ManifestProposals): SetupHeld {
  const ticks = Object.fromEntries(scan.workspaces.map((one) => [one.dir, one.evidence === "strong"]));
  if (was.state === "open") return { ...was, scan, proposals, ticks: { ...ticks, ...was.ticks } };
  return { state: "open", scan, proposals, ticks, open: null, marks: {}, busy: null };
}

/** An edit's or a Write's answer. What Fleet answered with is what the sheet redraws from. */
export function answered(
  was: SetupHeld,
  dir: string,
  doing: "edit" | "write",
  answer: ProposalAnswer,
  say: (outcome: Outcome) => string,
): SetupHeld {
  if (was.state !== "open") return was;
  const busy = was.busy === dir ? null : was.busy;
  let mark: SetupMark = {};
  let proposals = was.proposals;
  switch (answer.state) {
    case "took":
      proposals = {
        ...proposals,
        proposals: proposals.proposals.map((one) => (one.dir === dir ? answer.proposal : one)),
      };
      break;
    case "refused":
      mark = doing === "write" ? { refused: { saying: answer.saying, faults: answer.faults } } : { problem: answer.saying };
      break;
    case "appeared":
      mark = { appeared: { onDisk: answer.onDisk } };
      break;
    case "failed":
      mark = { problem: say(answer.outcome) };
      break;
  }
  return { ...was, proposals, busy, marks: { ...was.marks, [dir]: mark } };
}

/** The picker's state column. It reports, never instructs, and none of it is hued. */
export function stateOf(proposal: ManifestProposal, open: boolean, mark: SetupMark = {}): string {
  if (proposal.written !== undefined) return "written";
  if (proposal.present === true || mark.appeared !== undefined) return "already set up";
  if (open) return "open, being edited";
  if (proposal.checks.length === 0) return "no checks proposed";
  if (proposal.refused !== undefined) return "would not load";
  return "ready to write";
}

export function pickerOf(held: SetupOpen): Pick<SetupPickerProps, "rows" | "names" | "grid"> {
  const scanned = new Map(held.scan.workspaces.map((one) => [one.dir, one]));
  const rows = held.proposals.proposals.map((proposal) => {
    const workspace = scanned.get(proposal.dir);
    const files = workspace === undefined ? [] : [...workspace.manifests, ...workspace.lockfiles].map((one) => one.file);
    const note =
      workspace?.evidence === "not_followed"
        ? "Nothing here could be read."
        : workspace?.evidence === "thin"
          ? "No file here names a script, so anything proposed is convention."
          : undefined;
    return {
      dir: proposal.dir,
      ticked: held.ticks[proposal.dir] ?? false,
      open: held.open === proposal.dir,
      files,
      ...(note === undefined ? {} : { note }),
      state: stateOf(proposal, held.open === proposal.dir, held.marks[proposal.dir]),
    };
  });
  return { rows, ...gridOf(held.proposals.proposals, held.ticks) };
}

/**
 * Check names across the ticked batch. `missing` is narrow: a name every other ticked
 * workspace declares and this one does not. **The root is never a sibling**, Scan's rule, and
 * the batch is recomputed as a person re-ticks it.
 */
export function gridOf(
  proposals: readonly ManifestProposal[],
  ticks: Record<string, boolean>,
): Pick<SetupPickerProps, "names" | "grid"> {
  const batch = proposals.filter((one) => ticks[one.dir] === true);
  const names: string[] = [];
  for (const proposal of batch) {
    for (const check of proposal.checks) if (!names.includes(check.name)) names.push(check.name);
  }
  const declared = new Map(batch.map((one) => [one.dir, new Set(one.checks.map((check) => check.name))]));
  const siblings = batch.filter((one) => one.dir !== ".");
  const grid = batch.map((proposal) => ({
    dir: proposal.dir,
    cells: names.map((name): SetupGridCell => {
      if (declared.get(proposal.dir)?.has(name)) return "declared";
      if (proposal.dir === ".") return "absent";
      const others = siblings.filter((one) => one.dir !== proposal.dir);
      return others.length > 0 && others.every((one) => declared.get(one.dir)?.has(name)) ? "missing" : "absent";
    }),
  }));
  return { names, grid };
}

/**
 * Every value each policy takes, with its consequence. `settings.toml` is the authority on
 * the words; a proposal carries only the value in force, so they are listed here.
 */
export const POLICY_CHOICES: Record<string, { label: string; options: { value: string; says: string }[] }> = {
  auto_merge: {
    label: "Auto merge",
    options: [
      { value: "never", says: "A person merges every pull request here." },
      { value: "checks-pass", says: "Fleet merges once every check the forge runs has passed." },
      { value: "always", says: "Fleet merges without waiting on the forge's checks or a review." },
    ],
  },
  review_gate: {
    label: "Review gate",
    options: [
      { value: "human_always", says: "A person answers every review step." },
      { value: "auto_if_judge_passes", says: "The checks decide a review step, unless the Judge objects." },
    ],
  },
};

/** The caps in one line, never controls: at Setup no Job has run here to set them against. */
export function capsLine(caps: StatedCaps): string {
  return (
    `Jobs here stop at $${(caps.cost_micros / 1_000_000).toFixed(2)} or ${caps.turns} turns, this machine's caps. ` +
    "Change these on the Manifest page, where past Jobs' costs are."
  );
}

function cited(provenance: Provenance): ProvenanceProps {
  return {
    source: provenance.source,
    ...(provenance.file === undefined ? {} : { file: provenance.file }),
    ...(provenance.key === undefined ? {} : { at: provenance.key }),
  };
}

type Drawn = Omit<
  ProposalSheetProps,
  | "open"
  | "busy"
  | "verify"
  | "floor"
  | "contained"
  | "onClose"
  | "onEditId"
  | "onEditRun"
  | "onMove"
  | "onRemove"
  | "onAdd"
  | "onAddPort"
  | "onPolicy"
  | "onWrite"
  | "onEditManifest"
>;

export function sheetOf(held: SetupOpen, proposal: ManifestProposal): Drawn {
  const mark = held.marks[proposal.dir] ?? {};
  const policy: ProposalPolicy[] = proposal.policy.map((row) => {
    const choices = POLICY_CHOICES[row.key];
    return {
      key: row.key,
      label: choices?.label ?? row.key,
      value: row.value,
      options: choices?.options ?? [{ value: row.value, says: "" }],
      cited: cited(row.provenance),
    };
  });
  const elsewhere = held.proposals.proposals
    .filter((one) => one.dir !== proposal.dir)
    .flatMap((one) => one.ports.flatMap((port) => (port.env === undefined ? [] : [{ env: port.env, dir: one.dir }])));
  return {
    dir: proposal.dir,
    file: proposal.file,
    id: { value: proposal.id.value, cited: cited(proposal.id.provenance) },
    ports: proposal.ports.map((port) => ({
      name: port.name,
      ...(port.container === undefined ? {} : { container: port.container }),
      ...(port.env === undefined ? {} : { env: port.env }),
      cited: cited(port.provenance),
    })),
    checks: proposal.checks.map((check) => ({
      name: check.name,
      run: check.run,
      ...(check.requires === undefined ? {} : { requires: check.requires }),
      cited: cited(check.provenance),
    })),
    commands: proposal.commands.map((command) => ({
      name: command.name,
      run: command.run,
      ...(command.destructive === undefined ? {} : { destructive: command.destructive }),
      cited: cited(command.provenance),
    })),
    ...(proposal.setup === undefined
      ? {}
      : { setup: { requires: proposal.setup.requires, cited: cited(proposal.setup.provenance) } }),
    policy,
    caps: capsLine(held.proposals.caps),
    elsewhere,
    ...(mark.problem === undefined ? {} : { problem: mark.problem }),
    ...(mark.refused === undefined ? {} : { refused: mark.refused }),
    ...(mark.appeared === undefined ? {} : { appeared: mark.appeared }),
    setUp: proposal.written === undefined && (proposal.present === true || mark.appeared !== undefined),
    ...(proposal.written === undefined
      ? {}
      : { written: `Wrote ${proposal.file} at ${clockOf(proposal.written.at)}. Nothing was staged or committed.` }),
  };
}
