// The Board, built: one row per state a Job can be listed in, in the shape
// `list_jobs` sends. Every row is the fixture Job's own record with a
// different id, title and status, so the tabs, the sort and the step bar all
// run the app's own arithmetic over it.
import type { JobSummary, WorkflowSummary } from "@armada/protocol";
import { job, workflow } from "./base";

/** One row. `at` is the step it is on, by id in the fixture workflow. */
function row(
  n: number,
  handle: string,
  title: string,
  status: string,
  over: Partial<JobSummary> = {},
): JobSummary {
  return job(status, {
    id: `01M2C1TJ8G00${String(n).padStart(2, "0")}BOARDROW0000`,
    handle: `${n}-${handle}`,
    title,
    created_at: `2026-09-10T${String(8 + n).padStart(2, "0")}:00:00Z`,
    ...over,
  });
}

/** Every tab has something in it, and needs-you has more than one kind. */
export function boardJobs(): JobSummary[] {
  return [
    row(1, "coalesce-token-refreshes", "Coalesce concurrent token refreshes", "awaiting_approval", {
      branch: undefined,
      assigned_drone: undefined,
    }),
    row(2, "split-the-settings-reducer", "Split the settings reducer so the selectors can be tested alone", "awaiting_review", {
      current_step_id: "regression_verify",
    }),
    row(3, "retry-ceiling-on-poke", "Add a retry ceiling to the poke loop", "escalated", {
      current_step_id: "fix",
    }),
    row(4, "cache-the-manifest-read", "Cache the manifest read", "running", {
      current_step_id: "fix",
    }),
    row(5, "screenshots-in-job-context", "Support attaching screenshots and file search in job context", "running", {
      current_step_id: "repro",
    }),
    row(6, "retire-the-legacy-poke-path", "Retire the legacy poke path", "queued", {
      branch: undefined,
      assigned_drone: undefined,
    }),
    row(7, "clear-reclaims-worktrees", "Board's clear button should reclaim worktrees, not delete records", "completed_success", {
      current_step_id: "land",
    }),
    row(8, "refuse-a-merge-press", "Refuse a merge press whose chosen comments won't fit the brief", "completed_failed", {
      current_step_id: "regression_verify",
    }),
    row(9, "rename-session-token", "Rename the session token field", "killed", {
      current_step_id: "root_cause",
    }),
    row(10, "fold-the-two-sockets", "Fold a Job's two sockets in the wire package", "completed_success", {
      current_step_id: "land",
      reclaimed_at: "2026-09-10T20:00:00Z",
    }),
  ];
}

/** The workflows the rows name. One, because every row is on the fixture's. */
export function boardWorkflows(): WorkflowSummary[] {
  return [workflow()];
}
