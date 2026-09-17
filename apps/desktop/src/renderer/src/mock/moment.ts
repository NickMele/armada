// What every scenario is made of: the state a connected Fleet publishes, and a
// scenario built from rows a test picks. Apart from `scenario.ts`, so a
// scenario's own Fleet (`setup-fleet.ts`) can build on it and still be listed
// there without the two importing each other.

import { connectedTo, PROTOCOL_VERSION } from "@armada/protocol";
import type {
  Connection,
  JobSummary,
  ModelChoices,
  Outcome,
  RepositorySummary,
  Studio,
  WorkflowSummary,
} from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { repository, workflow } from "@armada/screens/src/fixtures/build/base";

import type { BridgeApi } from "../../../shared/api";
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
  /**
   * The Studios this scenario's Fleet keeps. **Every scenario answers the Studio reads and
   * writes** (#1341), so one that names none keeps an empty list and the surface draws its empty
   * state rather than a read failure.
   */
  studios?: readonly Studio[];
  /**
   * Calls this scenario answers as Fleet would, over the fake's own. For a
   * flow whose answers depend on what was pressed before — Setup's edits and
   * Writes, a clone — which a fixed read cannot hold.
   */
  behaves?: (fleet: FleetHandle) => Partial<BridgeApi>;
};

/** What a scenario's `behaves` reaches: the state as published, and the one way to change it. */
export type FleetHandle = {
  state: () => BridgeState;
  publish: (change: Partial<BridgeState>) => void;
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
export function connected(
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

/**
 * A connected Fleet holding exactly these rows, for a test that needs a Board
 * no named scenario draws — two repositories, one picked, a field changed.
 * `picked` is the rail's pick by root, as main holds it; absent is All.
 */
export function onBoard(
  jobs: JobSummary[],
  {
    workflows = [workflow()],
    repositories = [repository()],
    picked = null,
  }: { workflows?: WorkflowSummary[]; repositories?: RepositorySummary[]; picked?: string | null } = {},
): Scenario {
  return {
    name: "board",
    says: "A Board a test asked for",
    state: { ...connected(jobs, workflows, repositories), repository: picked },
    reads: {},
  };
}
