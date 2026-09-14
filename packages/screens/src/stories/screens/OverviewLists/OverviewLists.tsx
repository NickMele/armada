import type { JobSummary, RepositorySummary, WorkflowSummary } from "@armada/protocol";
import { OverviewLists } from "../../../OverviewLists";
import { job, repository } from "../../../fixtures/build/base";
import { boardJobs, boardWorkflows } from "../../../fixtures/build/board";

/** The moment every row's elapsed is read at, so a story never moves. */
export const NOW = Date.parse("2026-09-10T21:00:00Z");

export const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
export const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};

/**
 * One Job in every state `boardJobs` names, plus a status this build's registry does not know —
 * the same case `Jobs.tsx` names beneath its own list rather than drawing.
 */
export const JOBS: JobSummary[] = [
  ...boardJobs(),
  job("not_a_status_the_registry_has", { id: "01M2C1TJ8G00UNKNOWNSTATUS00", handle: "11-unknown-status" }),
];

/** `boardJobs`, split across two repositories, for the naming story. */
export const acrossTwo = (): JobSummary[] =>
  JOBS.map((row, at) => ({ ...row, owner_manifest_id: at % 2 === 0 ? "armada" : "storefront" }));

/**
 * Two running Jobs, one with a plan and one without — the owner's own report against the Running
 * panel, that the step bar sat beside the title on one row and to its own right on the other. The
 * row is the Board's own now, and its shared columns are what line the two up rather than a second
 * arrangement of the row.
 */
export const RUNNING_ONE_WITH_A_PLAN: JobSummary[] = [
  job("running", {
    id: "01M2C1TJ8G00RUNNINGWITHTASK",
    handle: "16-fix-801-unanswered-permission-ask-holds-dr",
    title: "Fix 801: unanswered permission ask holds Drone slot, 2nd dispatch",
    workflow_id: "implement",
    owner_manifest_id: "armada",
    current_step_id: "fix",
    started_at: new Date(NOW - 476_000).toISOString(),
    tasks: { done: 0, working: 0, open: 6, dropped: 0 },
  }),
  job("running", {
    id: "01M2C1TJ8G00RUNNINGNOPLAN00",
    handle: "17-job-details-view-shows-queued-while-fleet-r",
    title: 'Job details view shows "queued" while Fleet reports job as running',
    workflow_id: "plan",
    owner_manifest_id: "armada",
    current_step_id: "plan",
    started_at: new Date(NOW - 138_000).toISOString(),
  }),
];

const noop = () => {};

/**
 * Overview's lists, drawn by the app's own `OverviewLists` from Jobs in the shape `list_jobs`
 * sends. Only the data is made up.
 */
export function OverviewListsFrom({
  jobs = JOBS,
  workflows = boardWorkflows(),
  repositories = [ARMADA],
  picked = null,
  disconnected = null,
  stale = false,
  now = NOW,
}: {
  jobs?: readonly JobSummary[];
  workflows?: readonly WorkflowSummary[];
  repositories?: readonly RepositorySummary[];
  /** The rail's pick, by root. `null` is All repositories. */
  picked?: string | null;
  disconnected?: string | null;
  stale?: boolean;
  now?: number;
}) {
  return (
    <OverviewLists
      jobs={jobs}
      stale={stale}
      now={now}
      workflows={workflows}
      repositories={repositories}
      picked={picked}
      disconnected={disconnected}
      selected={null}
      onOpen={noop}
      onKill={noop}
      onCopied={noop}
    />
  );
}
