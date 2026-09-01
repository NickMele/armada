// Where a Job's artifacts are, derived once for both sides of the wire.
//
// The architecture fixes the layout — `<repo>/.armada/worktrees/<job-id>`,
// `logs/<job-id>.jsonl`, `transcripts/<drone-id>.jsonl` — and says it is not
// configurable, so a path derived here is the path Fleet wrote rather than a
// guess. Only `branch` is served, and it is not a path.
//
// **One derivation, because there are two callers.** The renderer derives to
// draw a row; main derives to open one, and main derives it again from the Job
// id rather than taking the string the row was drawn with — a path arriving
// from a click handler and going to `shell.openPath` is a capability the
// renderer was never given. Two copies of this arithmetic would be two
// capabilities that agree until one of them is edited.
//
// **Three paths are not derivable and are served instead.** A Check's output, a
// Judge's brief and a step's deliverable are keyed by step, run, ordinal and
// criterion, which is Fleet's arithmetic and not a layout rule — `get_job`
// carries each as a path relative to the repository root. The rule above still
// holds for them, one level up: main does not trust the string, it checks the
// string is one Fleet named for that Job. A set membership rather than a
// derivation, because there is nothing here to derive from.

import type { ManifestSummary } from "./setup";

/** The per-repo directory the architecture fixes. Not configurable. */
export const ARMADA = ".armada";

/**
 * Which artifact of a Job.
 *
 * **Two shapes, because there are two kinds of path and only one of them can
 * be derived.** The three words name the Job-level paths the architecture
 * fixes, and main rebuilds each from the Job id. [`Kept`] names the three
 * per-step records — a Check's output, a Judge's brief, a step's deliverable —
 * whose paths only Fleet knows, because each carries a step, a run, and an
 * ordinal or a criterion that no layout rule derives.
 *
 * **Neither shape hands the renderer an arbitrary-file capability.** A word is
 * rebuilt from the Job id; a kept path is checked against the paths Fleet named
 * for that Job in the detail main is holding, and refused where it is not one
 * of them. `main/open.ts` is where that check is.
 *
 * The branch is neither. It is served rather than derived, it is not a path,
 * and it is the one row the owner said was already right as a copy.
 */
export type Artifact = "worktree" | "log" | "transcript" | Kept;

/**
 * One of Fleet's per-step records, by the path Fleet named it with.
 *
 * **The path is the identity, and that is the decision.** The alternative was a
 * key — step, run, ordinal — that main would rebuild the file name from, which
 * would be a second spelling of arithmetic living in `crates/fleet`'s
 * `keeping.rs`, `asked.rs` and `check_output.rs`. Two spellings of one name
 * agree until one of them is edited, and what that failure produces is a path
 * to a file nobody has — which is the defect being fixed, one layer down.
 *
 * `what` is what the row is, for the sentence a failed open says. **It never
 * decides where the file is.**
 */
export type Kept = { kept: string; what: "check" | "brief" | "deliverable" };

/** Whether this names a per-step record rather than one of the fixed paths. */
export function isKept(what: Artifact): what is Kept {
  return typeof what !== "string";
}

/**
 * The repository a Job's artifacts sit under, or `null`.
 *
 * The Manifest's `path` is the absolute `armada.yml`, and its parent is the
 * workspace root Fleet canonicalises into `repo_root` — `Host.repo_root` is
 * not on the wire, so this is the same value read from the same place. `null`
 * where Fleet holds no Manifest by that id: an older Job, or one whose
 * Manifest was removed.
 */
export function repoOf(manifest: ManifestSummary | undefined): string | null {
  if (manifest === undefined) return null;
  const cut = manifest.path.lastIndexOf("/");
  return cut <= 0 ? null : manifest.path.slice(0, cut);
}

/**
 * Where one artifact of one Job is.
 *
 * **The transcript is a directory where the drone id is unknown.**
 * `assigned_drone` has no event that sets it, so on most Jobs the only record
 * the id existed is a line inside the Job log — and a directory a person can
 * open beats a path with a hole in it.
 */
export function artifactPath(
  what: Artifact,
  repo: string,
  jobId: string,
  droneId: string | undefined,
): string {
  // A kept record's path is Fleet's own, relative to the repository root, so
  // this only joins the two halves. Deciding the string is one Fleet named is
  // main's, not this function's — the renderer reaches here too, to draw a row.
  if (isKept(what)) return `${repo}/${what.kept}`;
  switch (what) {
    case "worktree":
      return `${repo}/${ARMADA}/worktrees/${jobId}`;
    case "log":
      return `${repo}/${ARMADA}/logs/${jobId}.jsonl`;
    case "transcript":
      return droneId === undefined
        ? `${repo}/${ARMADA}/transcripts/`
        : `${repo}/${ARMADA}/transcripts/${droneId}.jsonl`;
  }
}

/**
 * What opening one answered.
 *
 * **`not_there` is a first-class answer rather than a failure to report.**
 * `armada clean` reclaims worktrees and a Job outlives its own directory, so
 * a path that is gone is the ordinary case on an old Job — and it has to say
 * so, because a handler that opened nothing and said nothing is the silent
 * failure `#98` refuses.
 */
export type Opened =
  | { ok: true }
  /** No Job by that id on the board or open. Nothing was derived. */
  | { ok: false; why: "unknown_job" }
  /** Fleet holds no Manifest for the Job, so there is no repository to derive from. */
  | { ok: false; why: "no_repository" }
  /** The path was derived and nothing is at it. */
  | { ok: false; why: "not_there"; path: string }
  /**
   * A kept record was named that the Job's detail main is holding does not
   * name.
   *
   * **Not something a person can cause, and it still gets a sentence.** A row
   * only ever hands back the path it was drawn from, so this is a detail main
   * published and then replaced — the Job re-read between the draw and the
   * click — or a caller sending a string it never read off the wire. Refusing
   * is what keeps `shell.openPath` off an arbitrary path; refusing *silently*
   * would be the same dead click this whole seam exists to remove.
   */
  | { ok: false; why: "not_named"; path: string }
  /**
   * The path was derived, it is there, and nothing opened it.
   *
   * Two things can decline: the OS handed the path, whose words are carried
   * through rather than replaced, and the editor `$VISUAL` or `$EDITOR` names,
   * which is Bridge saying which variable named what is not there. **One
   * variant rather than two**, because a distinct `why` would have to be read
   * by `whyNotOpened` in the renderer — worth doing, and it is the shape to
   * reach for the next time this is opened.
   */
  | { ok: false; why: "refused"; path: string; detail: string };
