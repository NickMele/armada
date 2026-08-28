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

import type { ManifestSummary } from "./protocol";

/** The per-repo directory the architecture fixes. Not configurable. */
export const ARMADA = ".armada";

/**
 * Which artifact of a Job. **A closed set, and the whole of what the renderer
 * may name** — the path itself is main's to build.
 *
 * The branch is not here. It is served rather than derived, it is not a path,
 * and it is the one row the owner said was already right as a copy.
 */
export type Artifact = "worktree" | "log" | "transcript";

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
