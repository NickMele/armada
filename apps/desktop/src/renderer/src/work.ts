// Where a Job's work is, and what the Job was told — the region a person opens
// when they want a path rather than a reading.
//
// # Derived is not served
//
// `branch` is served. Where the work sits on disk is **derived**, from the
// Job's id and the repository its Manifest was read from — see
// `shared/artifacts.ts`, which owns that arithmetic for both sides. The
// architecture fixes the layout, says it is not configurable, and says any path
// Fleet needs is derived rather than stored, so a path derived on this side is
// the same path and not a guess.
//
// # A row opens by naming itself, never by handing over its path
//
// `open` sends the Job id and one of three words. Main derives the path again
// and hands that to the OS — a string composed here and passed to
// `shell.openPath` would be an arbitrary-file capability wearing a row's
// clothes. So this file draws the path and main opens it, from one derivation.
//
// # Five rows, and three of them came down from the header
//
// Where the work is, what branch it is on, and the three identifiers the header
// used to stack into a second line: the Manifest, the workflow and the Drone.
// The two `.jsonl` rows and the spend block are gone — spend is one of the
// header's four facts, and a log path is not one of the things this region
// names. **The drawing keeps a `Job log` and a `Transcript` row; the issue asks
// for them dropped.** Reported.
//
// # The brief is one line
//
// `Done means` and `What it was told` were two sub-headings inside a tall card
// and neither is in the drawing. The brief is the sentence the Job was given,
// on the panel's own surface, above the step. What the Job's criteria are is
// what the Judge stage of the phase strip opens to, which is where a person
// asks the question.

import { File, Folder, GitBranch } from "lucide-react";
import type { JobBriefProps, JobLogReferenceRow, NotOpened } from "@armada/components";

import type { Watched } from "../../shared/bridge";
import { artifactPath, repoOf } from "../../shared/artifacts";
import type { Artifact, Opened } from "../../shared/artifacts";
import type { JobDetail as JobWhole, JobSummary } from "../../shared/protocol";
import type { ManifestSummary, WorkflowSummary } from "../../shared/setup";

export { repoOf };

/**
 * What the Job was told, in the words it was told it — one line, on the panel's
 * raised surface, above the step every step is read against.
 *
 * **The waiting note rides with it and is never remembered.** Fleet clears it
 * off the record the instant a drone's opening brief is built from it, so
 * `redirect_waiting` absent is both "nobody wrote one" and "the one somebody
 * wrote has gone in" — and neither of those is a thing to draw. The move that
 * delivers it puts the job at `running`, which is a `job.state_changed` that
 * `connection.ts` re-reads the open job on, so the block leaves the screen on
 * the same transition that empties the field.
 */
export function briefOf(whole: JobWhole): JobBriefProps {
  return {
    // Required by the shape and not drawn: `only` picks the half this region
    // is. The criteria are what the Judge stage opens to, with each one's
    // verdict beside it, which is one place rather than two.
    criteria: [],
    only: "facts",
    // No label. The region is called Brief and the sentence follows it; a
    // second heading over one line is the sub-heading this screen removed.
    factsLabel: null,
    facts: whole.facts,
    factsAbsent: "This job was given no context beyond its title.",
    waiting: whole.redirect_waiting?.note,
  };
}

/**
 * Why an open did not happen, in the app's voice.
 *
 * **Four sentences, not one.** A reclaimed directory, a Manifest Fleet no
 * longer holds and an OS with no handler need three different next steps, and a
 * shared sentence would send a person to look for the wrong one.
 */
function whyNotOpened(opened: Opened, what: Artifact): string | null {
  if (opened.ok) return null;
  switch (opened.why) {
    case "unknown_job":
      return "Bridge no longer holds this job, so it could not work out where that is.";
    case "no_repository":
      return (
        "Fleet holds no manifest with this job's id, so Bridge cannot say which repository " +
        "the work is in. The path above follows from the job id once it can."
      );
    case "not_there":
      return what === "worktree"
        ? "Nothing is there. It was reclaimed, and the branch survives it."
        : "That file has not been written, or it has been removed. The path is still where it goes.";
    case "refused":
      return `This machine did not open it: ${opened.detail}`;
  }
}

/** Open one of a Job's artifacts, and say why it did not when it did not. */
function opener(jobId: string, what: Artifact, label: string) {
  return {
    label,
    go: async (): Promise<NotOpened> => {
      const because = whyNotOpened(await window.armada.openArtifact(jobId, what), what);
      return because === null ? null : { because };
    },
  };
}

/**
 * The rows: where the work is, and the identifiers that name it.
 *
 * `undefined` while the Job has not been read whole — the caller says why,
 * because "still reading" and "Fleet did not answer" are different sentences.
 */
export function workOf(
  job: JobSummary,
  whole: JobWhole | null,
  manifest: ManifestSummary | undefined,
  workflow: WorkflowSummary | undefined,
): JobLogReferenceRow[] | undefined {
  if (whole === null) return undefined;
  const repo = repoOf(manifest);
  const dispatched = whole.branch !== undefined;
  const rows: JobLogReferenceRow[] = [];

  if (repo !== null) {
    const where = artifactPath("worktree", repo, job.id, job.assigned_drone);
    rows.push({
      // `folder` means "workspace" in the registry and this is not one; there
      // is no row for it. Reported, and no glyph is invented.
      icon: Folder,
      iconLabel: "Worktree",
      value: where,
      copyValue: where,
      open: opener(job.id, "worktree", "Open the worktree"),
      meta: dispatched ? undefined : NOT_WRITTEN,
    });
  }

  if (whole.branch !== undefined) {
    // No `open`, and none is coming. A branch is served rather than derived,
    // it is not a path, and copying it is the whole of what it is for.
    rows.push({
      icon: GitBranch,
      iconLabel: "Branch",
      value: whole.branch,
      copyValue: whole.branch,
    });
  }

  // The three the header used to carry. **Values you go and find, not values
  // you read** — which is the whole distinction between this region and the
  // four facts above the panel.
  rows.push({
    icon: File,
    iconLabel: "Manifest",
    value: manifest?.repository ?? job.owner_manifest_id,
    copyValue: job.owner_manifest_id,
    separated: true,
  });
  rows.push({
    iconLabel: "Workflow",
    value: workflow?.name ?? job.workflow_id,
    copyValue: job.workflow_id,
    meta: "as it was when the job was dispatched",
  });
  if (job.assigned_drone !== undefined) {
    rows.push({
      iconLabel: "Drone",
      value: job.assigned_drone,
      copyValue: job.assigned_drone,
    });
  }

  rows.push(...overlapRows(whole));
  return rows;
}

/**
 * Who else claims the paths this Job may write.
 *
 * **It names the workspace and the other Job, and stops.** It used to list
 * every shared path, comma-joined with no ceiling — five paths at forty
 * characters is eight hundred on one line, in mono, and it ran off the right
 * edge of the window and took the header's height with it. The overlap is a
 * fact a person acts on by deciding whether to wait, and that decision needs
 * which Job and roughly where. Whoever wants the file list is one click from
 * the other Job, and the whole list is still what copying the row writes.
 *
 * **A count for the rest, never the rest.** `+4` is the shape a Convoy's row
 * already uses for its extra write targets.
 */
function overlapRows(whole: JobWhole): JobLogReferenceRow[] {
  return (whole.write_scope_overlaps ?? []).map((other, at) => {
    const paths = other.paths.map((shared) => shared.path);
    const first = paths[0];
    const rest = paths.length - 1;
    const also = `${other.title} is writing into it too`;
    return {
      iconLabel: "Overlaps",
      value: first === undefined ? other.job_id : workspaceOf(first),
      copyValue: paths.join(" "),
      meta: rest > 0 ? `${also} · +${rest}` : also,
      separated: at === 0,
    };
  });
}

/**
 * The workspace a shared path is in — its first two segments, or the path where
 * it is shallower. **Not the path**: the Board's dispatch card names
 * `crates/fleet` and leaves the file list on the other Job.
 */
function workspaceOf(path: string): string {
  const parts = path.split("/");
  return parts.length <= 2 ? path : `${parts[0]}/${parts[1]}`;
}

/** A file that will exist, named before it does. Never a count. */
const NOT_WRITTEN = "not written yet";

/** Why there is no region to draw, which is never the same sentence twice. */
export function whyNoWork(watched: Watched, jobId: string): string {
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer for this job, so its paths and its brief are unknown.";
  }
  return "Reading this job.";
}
