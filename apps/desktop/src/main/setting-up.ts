// Setup's reads and acts: Scan, a proposal per workspace, one edit, and Write.
// No path crosses — Fleet serves the checkout it was started in until Locate (#821) names another.

import type {
  EditManifestProposal,
  ManifestProposal,
  ManifestProposals,
  RepositoryScan,
  WriteManifestProposal,
} from "@armada/protocol";
import type {
  ManifestProposalsRead,
  ProposalAnswer,
  RepositoryScanRead,
} from "@armada/screens/src/setup-reads";

import { ask, type Answer } from "./request";

/** Declared in `crates/fleet/src/manifest_proposal/held.rs`. Matched, never minted. */
const APPEARED = "fleet.manifest_appeared";
const REFUSALS = [
  "fleet.no_such_workspace",
  "fleet.proposal_not_amended",
  "fleet.proposal_written",
  "fleet.proposal_refused",
];

/** An edit's or a Write's answer. `faults` rides as `[key, fault]` pairs; any other shape is dropped. */
export function proposalAnswerOf(answer: Answer): ProposalAnswer {
  if (answer.ok === true) return { state: "took", proposal: answer.body as ManifestProposal };
  const outcome = answer.outcome;
  if (outcome.ok || outcome.why !== "refused") return { state: "failed", outcome };
  const { code, message, fields } = outcome.error;
  if (code === APPEARED) {
    const onDisk = fields["on_disk"];
    return { state: "appeared", onDisk: typeof onDisk === "string" ? onDisk : null };
  }
  if (!REFUSALS.includes(code)) return { state: "failed", outcome };
  const pairs = Array.isArray(fields["faults"]) ? (fields["faults"] as unknown[]) : [];
  const faults = pairs.flatMap((pair) =>
    Array.isArray(pair) && typeof pair[0] === "string" && typeof pair[1] === "string"
      ? [{ key: pair[0], fault: pair[1] }]
      : [],
  );
  return { state: "refused", saying: message, faults };
}

export class SetupCommands {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Every workspace in the checkout, read-only. */
  async readScan(): Promise<RepositoryScanRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "GET", "/repository/scan");
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, scan: answer.body as RepositoryScan };
  }

  /** One proposal per workspace, with every edit Fleet holds applied. */
  async readProposals(): Promise<ManifestProposalsRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "GET", "/repository/proposals");
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, proposals: answer.body as ManifestProposals };
  }

  /** One edit to one proposal, held in Fleet's memory. Nothing is written. */
  async edit(body: EditManifestProposal): Promise<ProposalAnswer> {
    const port = this.port();
    if (port === null) return { state: "failed", outcome: { ok: false, why: "not_connected" } };
    return proposalAnswerOf(await ask(port, "POST", "/repository/edit_proposal", body));
  }

  /** Create `armada.yml` for one workspace. **Creates, never replaces**, and stages nothing. */
  async write(body: WriteManifestProposal): Promise<ProposalAnswer> {
    const port = this.port();
    if (port === null) return { state: "failed", outcome: { ok: false, why: "not_connected" } };
    return proposalAnswerOf(await ask(port, "POST", "/repository/write_proposal", body));
  }
}
