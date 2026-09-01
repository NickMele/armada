// Hand one of a Job's artifacts to the OS.
//
// The one place in Bridge that reaches outside the app at all, and it does so
// on a path **it built** — the renderer names a Job id and one of three words,
// and nothing it sends is concatenated into what `shell.openPath` receives. A
// string arriving from a click handler and going to the shell would make every
// capability the CSP and the sandbox hold back reachable through one row.
//
// **The three per-step records are the exception, and they are checked rather
// than derived.** A Check's output, a Judge's brief and a step's deliverable
// are keyed by things only Fleet holds, so `get_job` carries each as a path.
// The property above survives as set membership: `named` collects every path
// the detail main is holding names for that Job, and a string that is not one
// of them is refused before anything is joined to it. So the renderer still
// cannot reach a file Fleet did not put on the wire — it can only ask for one
// that is already on this screen.
//
// Fleet is not involved. Nothing here is a Job act, which is why it is beside
// `command.ts` rather than in it: those are POSTs to routes under a Job, and
// this touches the filesystem and stops.
//
// **Which editor it lands in is `editor.ts`'s question, not this file's.** That
// module owns the precedence and the argv-only rule that keeps a configured
// string away from a shell; this one owns the path and the answer.

import { shell } from "electron";
import { stat } from "node:fs/promises";

import type { BridgeState } from "../shared/bridge";
import { artifactPath, isKept, repoOf } from "../shared/artifacts";
import type { Artifact, Opened } from "../shared/artifacts";
import { chooseEditor, startEditor, whereIs } from "./editor";
import type { Editor } from "./editor";
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
 * Every repo-relative path this Job's detail names, as one set.
 *
 * **Read off `watched` alone, and that is the whole of the rule.** `jobs` is
 * the Board's summaries and carries no step, so a Job that is not the one open
 * names no record — which is right, because the only surface that draws these
 * rows is the open Job's. A Job re-read between the draw and the click answers
 * from the new reading, so a record a re-run replaced stops being reachable at
 * the moment it stops being on screen.
 *
 * **Reachability is therefore keyed on the row, exactly as the row is.** The
 * files outlive the rows — `armada clean` forgets a Job and leaves its briefs
 * and deliverables under the repository root — and nothing here tries to reach
 * one whose row is gone. A cleaned Job has no screen either, so no path is
 * named-and-unreachable; what a person keeps is a directory they can open by
 * hand. Retention over all four artifact kinds is `#69`.
 */
function named(state: BridgeState, jobId: string): ReadonlySet<string> {
  const watched = state.watched;
  if (watched.state !== "read" || watched.jobId !== jobId) return new Set();
  const paths = watched.detail.steps.flatMap((step) => [
    ...step.check_runs.map((run) => run.output_path),
    ...step.judged.map((judged) => judged.brief_path),
    ...(step.deliverables ?? []).map((kept) => kept.path),
  ]);
  return new Set(paths.filter((path): path is string => path !== undefined));
}

/**
 * Why a named editor could not be used, in the voice the row will read it in.
 *
 * **A configured editor that is not there says so rather than falling through
 * to Finder.** Finder would open, the path would appear, and the setting would
 * look ignored — which is the one outcome that teaches a person the wrong thing
 * about their own configuration. Falling through from an *unset* variable is a
 * different case entirely and is silent, because nothing has gone wrong.
 *
 * The sentence names the variable, because the next step is editing the line
 * that set it.
 */
function noEditor(editor: Editor): string {
  if (!editor.command.includes("/")) {
    return `${editor.from} names ${editor.command}, and nothing on PATH is called that.`;
  }
  return editor.command.startsWith("/")
    ? `${editor.from} names ${editor.command}, and nothing executable is there.`
    : `${editor.from} names ${editor.command}, which is relative to nothing — an editor is named by an absolute path or by a name on PATH.`;
}

/**
 * Open one artifact of one Job, in the editor a person named or in Finder.
 *
 * **Existence is checked before the OS is asked**, because `shell.openPath`
 * answers a reclaimed worktree and a missing handler with the same shape of
 * string, and those are two different sentences for a person: one says the
 * directory is gone, the other says nothing here opens `.jsonl`.
 *
 * The editor is resolved here rather than passed in: the renderer names a Job
 * id and one of three words, and it supplies no editor for the same reason it
 * supplies no path. See `editor.ts` for the precedence and for why the config
 * tier above `$VISUAL` is absent.
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
  // Membership before the filesystem, and before the path is spoken to anything
  // outside this process. `not_there` would be the wrong answer here: it says
  // Fleet named a file and the file is gone, and this says nothing named it.
  if (isKept(what) && !named(state, jobId).has(what.kept)) {
    return { ok: false, why: "not_named", path };
  }
  try {
    await stat(path);
  } catch {
    return { ok: false, why: "not_there", path };
  }

  const editor = chooseEditor(process.env);
  if (editor !== null) {
    const program = await whereIs(editor.command, process.env);
    if (program === null) return { ok: false, why: "refused", path, detail: noEditor(editor) };
    startEditor(program, editor.args, path);
    return { ok: true };
  }

  // Empty is success. Anything else is the OS explaining itself, and it is
  // carried through rather than replaced with a sentence Bridge made up.
  const refused = await shell.openPath(path);
  return refused === "" ? { ok: true } : { ok: false, why: "refused", path, detail: refused };
}
