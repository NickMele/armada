// Hand one of a Job's artifacts to the OS.
//
// The one place in Bridge that reaches outside the app at all, and it does so
// on a path **it built** — the renderer names a Job id and one of three words,
// and nothing it sends is concatenated into what `shell.openPath` receives. A
// string arriving from a click handler and going to the shell would make every
// capability the CSP and the sandbox hold back reachable through one row.
//
// Fleet is not involved. Nothing here is a Job act, which is why it is beside
// `command.ts` rather than in it: those are POSTs to routes under a Job, and
// this touches the filesystem and stops.

import { shell } from "electron";
import { stat } from "node:fs/promises";

import type { BridgeState } from "../shared/bridge";
import { artifactPath, repoOf } from "../shared/artifacts";
import type { Artifact, Opened } from "../shared/artifacts";
import type { JobSummary } from "../shared/protocol";

/**
 * The Job by that id, from what main last published.
 *
 * The board first, then the open Job: a Job can be on screen without being in
 * `jobs` — the board is what Fleet last listed and `watched` is what a person
 * has open, and the second is the one being clicked.
 */
function jobOf(state: BridgeState, jobId: string): JobSummary | undefined {
  const row = state.jobs.find((job) => job.id === jobId);
  if (row !== undefined) return row;
  const watched = state.watched;
  return watched.state === "read" && watched.jobId === jobId ? watched.detail.job : undefined;
}

/**
 * Open one artifact of one Job in whatever the OS opens it with.
 *
 * **Existence is checked before the OS is asked**, because `shell.openPath`
 * answers a reclaimed worktree and a missing handler with the same shape of
 * string, and those are two different sentences for a person: one says the
 * directory is gone, the other says nothing here opens `.jsonl`.
 *
 * `shell.openPath` rather than `showItemInFolder`: the ask was to land in an
 * editor, and revealing a file in Finder is a different act with a different
 * next step. What the OS does with a directory is the OS's to decide.
 */
export async function openArtifact(
  state: BridgeState,
  jobId: string,
  what: Artifact,
): Promise<Opened> {
  const job = jobOf(state, jobId);
  if (job === undefined) return { ok: false, why: "unknown_job" };

  const repo = repoOf(
    state.holds.manifests.find((manifest) => manifest.id === job.owner_manifest_id),
  );
  if (repo === null) return { ok: false, why: "no_repository" };

  const path = artifactPath(what, repo, job.id, job.assigned_drone);
  try {
    await stat(path);
  } catch {
    return { ok: false, why: "not_there", path };
  }

  // Empty is success. Anything else is the OS explaining itself, and it is
  // carried through rather than replaced with a sentence Bridge made up.
  const refused = await shell.openPath(path);
  return refused === "" ? { ok: true } : { ok: false, why: "refused", path, detail: refused };
}
