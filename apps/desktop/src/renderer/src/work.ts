// Where a Job's work is, and what the Job was told — the two halves of the
// region a person opens when a Job has gone wrong.
//
// # Derived is not served, and the region says which
//
// `branch` is served. The worktree, the log and the transcript directory are
// **derived here** from the Job's id and the repository its Manifest was read
// from: `<repo>/.armada/worktrees/<job-id>`, `logs/<job-id>.jsonl`,
// `transcripts/`. The architecture fixes that layout, says it is not
// configurable, and says any path Fleet needs is derived rather than stored —
// so a path derived on this side is the same path, not a guess. What Bridge
// cannot derive is which repository: `Host.repo_root` is not on the wire, and
// the Manifest's `path` — the absolute `armada.yml` — has that directory as
// its parent, which is the same value Fleet resolves it from.
//
// # No count, ever
//
// The drawing shows "142 lines · 0 error". Nothing counts either, so a row
// names its path and stops. The only `meta` here says a file is not written
// yet, which is read off `branch`: the worktree is created and the branch
// stamped on the Job before the log is opened, so a Job with no branch has
// nothing under any of these paths.

import { File, Folder, GitBranch } from "lucide-react";
import type { JobBriefProps, JobDetailLog, JobLogReferenceRow } from "@armada/components";

import type { Watched } from "../../shared/bridge";
import type {
  JobDetail as JobWhole,
  JobSummary,
  ManifestSummary,
} from "../../shared/protocol";

/** The per-repo directory the architecture fixes. Not configurable. */
const ARMADA = ".armada";

/**
 * The repository a Job's artifacts sit under, or `null`.
 *
 * The Manifest's `path` is the absolute `armada.yml`, and its parent is the
 * workspace root Fleet canonicalises into `repo_root`. `null` where Fleet
 * holds no Manifest by that id — an older Job, or a Manifest that was removed.
 */
export function repoOf(manifest: ManifestSummary | undefined): string | null {
  if (manifest === undefined) return null;
  const cut = manifest.path.lastIndexOf("/");
  return cut <= 0 ? null : manifest.path.slice(0, cut);
}

/** What the Job was told, and what done means for it. Both are served. */
export function briefOf(whole: JobWhole): JobBriefProps {
  return {
    criteria: whole.acceptance_criteria.map((criterion) => ({
      text: criterion.text,
      // The wire spelling. No registry carries a verb for `criterion_source`,
      // and one written here would be a second vocabulary.
      source: criterion.source,
    })),
    criteriaAbsent:
      "This job was proposed with no acceptance criteria, so nothing states what done " +
      "means for it. Bridge's composer does not offer them yet.",
    facts: whole.facts,
    factsAbsent: "This job was given no context beyond its title.",
  };
}

/**
 * The region: the brief, then the paths.
 *
 * `undefined` while the Job has not been read whole — the caller says why,
 * because "still reading" and "Fleet did not answer" are different sentences.
 * `withBranch` is false on the finished render, where the handover above
 * already names the branch and drawing it twice is two places to keep in step.
 */
export function workOf(
  job: JobSummary,
  whole: JobWhole | null,
  manifest: ManifestSummary | undefined,
  withBranch: boolean,
): JobDetailLog | undefined {
  if (whole === null) return undefined;
  const repo = repoOf(manifest);
  const dispatched = whole.branch !== undefined;
  const rows: JobLogReferenceRow[] = [];

  if (repo !== null) {
    rows.push({
      // `folder` means "workspace" in the registry and a worktree is not one;
      // there is no row for a worktree. Reported, and no glyph is invented.
      icon: Folder,
      iconLabel: "Worktree",
      value: `${repo}/${ARMADA}/worktrees/${job.id}`,
      copyValue: `${repo}/${ARMADA}/worktrees/${job.id}`,
      meta: dispatched ? undefined : NOT_WRITTEN,
    });
  }

  if (withBranch && whole.branch !== undefined) {
    rows.push({
      icon: GitBranch,
      iconLabel: "Branch",
      value: whole.branch,
      copyValue: whole.branch,
    });
  }

  if (repo !== null) {
    rows.push({
      icon: File,
      iconLabel: "Log",
      value: `${repo}/${ARMADA}/logs/${job.id}.jsonl`,
      copyValue: `${repo}/${ARMADA}/logs/${job.id}.jsonl`,
      meta: dispatched ? undefined : NOT_WRITTEN,
      separated: true,
    });
    rows.push(transcript(repo, job));
  }

  return { brief: briefOf(whole), rows, note: noteOf(repo, dispatched) };
}

/** A file that will exist, named before it does. Never a count. */
const NOT_WRITTEN = "not written yet";

/**
 * The Drone's transcript.
 *
 * **The file is named by a drone id that is stored nowhere.** `assigned_drone`
 * has no event that sets it, so the only record the id existed is the line
 * Fleet writes into the Job log named above. The row names the directory and
 * says that, rather than printing a path with a hole in it. No glyph: nothing
 * in the registry means a transcript, and `file` is reserved to the log row.
 */
function transcript(repo: string, job: JobSummary): JobLogReferenceRow {
  const directory = `${repo}/${ARMADA}/transcripts/`;
  if (job.assigned_drone === undefined) {
    return {
      iconLabel: "Transcript",
      value: directory,
      copyValue: directory,
      meta: "named by a drone id nothing serves — the job log above names it",
    };
  }
  const path = `${directory}${job.assigned_drone}.jsonl`;
  return { iconLabel: "Transcript", value: path, copyValue: path };
}

/** What the paths are, said once beneath them rather than on every row. */
function noteOf(repo: string | null, dispatched: boolean): string {
  if (repo === null) {
    return (
      "Fleet holds no manifest with this job's id, so Bridge cannot say which repository " +
      "the work is in. The paths follow from the job id once it can."
    );
  }
  const derived =
    "The worktree, the log and the transcripts directory follow from this job's id and " +
    "the repository its manifest was read from. The branch is served.";
  return dispatched
    ? `${derived} Armada leaves the worktree and the branch in place.`
    : `${derived} Nothing has been dispatched, so nothing under these paths exists yet.`;
}

/** Why there is no region to draw, which is never the same sentence twice. */
export function whyNoWork(watched: Watched, jobId: string): string {
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer for this job, so its paths and its brief are unknown.";
  }
  return "Reading this job.";
}
