// Where a Job's work is, and what the Job was told — the region a person opens
// when they want a path rather than a reading.
//
// # Derived is not served
//
// `branch` is served. Where the work sits on disk is **derived**, from the
// Job's id and the repository its Manifest was read from — see
// `@armada/protocol`'s `artifacts.ts`, which owns that arithmetic for both sides. The
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
import { Button } from "@armada/components";
import type { JobBriefProps, JobLogReferenceRow, NotOpened } from "@armada/components";

import type { ServerState, Watched } from "@armada/protocol";
import { artifactPath, recordsOf, repoOf } from "@armada/protocol";
import type { Artifact } from "@armada/protocol";
import { openArtifact, type OpenArtifact } from "./opening";
import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";
import type { ManifestSummary, WorkflowSummary } from "@armada/protocol";
import type { RunSheetSlice } from "./rehearsal";

/**
 * What the worktree row's **Run…** and a *Serving* row need — Journey 9, on
 * the one surface both open onto besides the sheet itself.
 */
export type WorkRehearsal = {
  /** Opens the run sheet with nothing selected. */
  onRun: () => void;
  /** The sheet's own reading of whether the worktree is on disk, once it has
   * one — `undefined` before anybody has opened the sheet, which falls back
   * to whether the Job has dispatched a worktree at all. */
  worktreeOnDisk: boolean | undefined;
  /** Every server Fleet holds. Filtered to this Job and to the live ones. */
  servers: readonly ServerState[];
  onStopServer: (serverId: string) => void;
  onOpenServerLink: (serverId: string, url: string) => void;
};

/** Builds `WorkRehearsal` off `useRunSheet`'s own return, so a caller passes one line. */
export function workRehearsalOf(
  onRun: () => void,
  worktreeOnDisk: boolean | undefined,
  slice: RunSheetSlice,
): WorkRehearsal {
  return {
    onRun,
    worktreeOnDisk,
    servers: slice.servers.servers,
    onStopServer: (serverId) => void slice.onStopServer(serverId),
    onOpenServerLink: (serverId, url) => void slice.onOpenServerLink(serverId, url),
  };
}

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
 * Open one of a Job's artifacts, and say why it did not when it did not.
 *
 * **The sentences moved to `opening.ts`.** The phase strip opens the three
 * per-step records now and needs the same five, and two copies of them would
 * drift into two vocabularies for one failure.
 */
function opener(open: OpenArtifact, jobId: string, what: Artifact, label: string) {
  return {
    label,
    go: async (): Promise<NotOpened> => {
      const because = await openArtifact(open, jobId, what);
      return because === null ? null : { because };
    },
  };
}

/**
 * The rows: where the work is, and the identifiers that name it.
 *
 * **None of them waits on this Job's own read** but Overlaps. The rest come off
 * the `JobSummary` the Board already holds and the Manifest and workflow holds
 * loaded for every Job, so the region draws the moment the screen opens.
 */
export function workOf(
  open: OpenArtifact,
  job: JobSummary,
  whole: JobWhole | null,
  manifest: ManifestSummary | undefined,
  workflow: WorkflowSummary | undefined,
  rehearsal: WorkRehearsal,
): JobLogReferenceRow[] {
  const repo = repoOf(manifest);
  // Only `worktree` is drawn on this screen, which never reads `records` —
  // see `artifactPath`'s own doc for which word reads which. Resolved anyway
  // so a row added here later does not have to learn where it comes from.
  const records = recordsOf(manifest) ?? "";
  // The read's answer once it is in, and the Board's row until then. Both are
  // Fleet's word for whether a worktree exists yet.
  const branch = whole === null ? job.branch : whole.branch;
  const dispatched = branch !== undefined;
  const rows: JobLogReferenceRow[] = [];

  if (repo !== null) {
    const where = artifactPath("worktree", repo, records, job.id, job.assigned_drone);
    rows.push({
      // `folder` means "workspace" in the registry and this is not one; there
      // is no row for it. Reported, and no glyph is invented.
      icon: Folder,
      iconLabel: "Worktree",
      value: where,
      copyValue: where,
      open: opener(open, job.id, "worktree", "Open the worktree"),
      meta: dispatched ? undefined : NOT_WRITTEN,
      // **Run…**, Journey 9. Disabled with a reason rather than hidden.
      // `worktree_on_disk` is the sheet's own reading and wins once it
      // exists; before the sheet has ever been opened this falls back to
      // whether the Job has dispatched a worktree at all.
      run: { onRun: rehearsal.onRun, disabledReason: disabledReasonOf(dispatched, rehearsal.worktreeOnDisk) },
    });
  }

  if (branch !== undefined) {
    // No `open`, and none is coming. A branch is served rather than derived,
    // it is not a path, and copying it is the whole of what it is for.
    rows.push({
      icon: GitBranch,
      iconLabel: "Branch",
      value: branch,
      copyValue: branch,
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
  });
  if (job.assigned_drone !== undefined) {
    rows.push({
      iconLabel: "Drone",
      value: job.assigned_drone,
      copyValue: job.assigned_drone,
    });
  }

  if (whole !== null) rows.push(...overlapRows(whole));
  rows.push(...servingRows(job.id, rehearsal));
  return rows;
}

/**
 * A row per server this Job is running or has running — Journey 9's *A
 * server*. **The sheet closed stops nothing**, so this draws for as long as
 * the server itself is up, independent of whether anybody has the sheet
 * open. Exited servers are the sheet's own business, not this region's.
 */
function servingRows(jobId: string, rehearsal: WorkRehearsal): JobLogReferenceRow[] {
  return rehearsal.servers
    .filter((server) => server.job_id === jobId && server.phase !== "exited")
    .map((server) => ({
      iconLabel: "Serving",
      // The address, not the name — `WhereRow`'s own rule for a mono value.
      // **No `meta` beside it**: the shared label column `853da1d9` fixed
      // wide enough for "Size on disk" leaves this row's value area narrow,
      // and a note here squeezed the address down to one character before
      // its own ellipsis. The name is still on the sheet and on hover.
      value: server.ports[0] === undefined ? server.serve : `localhost:${server.ports[0].port}`,
      ...(server.phase === "starting" ? { meta: "starting" } : {}),
      actions: (
        <>
          {server.links.map((link) => (
            <Button
              key={link.url}
              variant="secondary"
              size="sm"
              onClick={() => rehearsal.onOpenServerLink(server.id, link.url)}
            >
              {link.name ?? link.url}
            </Button>
          ))}
          <Button variant="secondary" size="sm" onClick={() => rehearsal.onStopServer(server.id)}>
            Stop
          </Button>
        </>
      ),
    }));
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

/**
 * Whether this Job's own read has yet to answer, `read` or `failed`. `whole` is
 * `null` both while it has not and once Fleet refused, and only the first draws
 * as waiting.
 */
export function stillReading(watched: Watched, jobId: string): boolean {
  if (watched.state === "none") return true;
  if (watched.jobId !== jobId) return true;
  return watched.state !== "read" && watched.state !== "failed";
}

/** Why **Run…** is disabled on a Job that has not dispatched a worktree yet. */
const NOT_DISPATCHED = "This Job has no worktree yet.";

/** Why **Run…** is disabled on a Job whose worktree the sheet found gone. */
const WORKTREE_GONE = "This Job's worktree is gone.";

/**
 * Why **Run…** is disabled, or `undefined` where it is not. `worktreeOnDisk`
 * is the run sheet's own reading and wins once it exists — it is the only
 * signal that tells a reclaimed worktree from one still there — and the
 * cheaper `dispatched` guess carries the row until somebody opens the sheet.
 */
function disabledReasonOf(dispatched: boolean, worktreeOnDisk: boolean | undefined): string | undefined {
  if (worktreeOnDisk !== undefined) return worktreeOnDisk ? undefined : WORKTREE_GONE;
  return dispatched ? undefined : NOT_DISPATCHED;
}

/**
 * Why there is no brief. **Two sentences, and neither describes the wire** —
 * one is a Job that has not arrived and one is a Job Fleet would not answer
 * for, which are different things to do next.
 */
export function whyNoBrief(watched: Watched, jobId: string): string {
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer";
  }
  return "Reading this job.";
}
