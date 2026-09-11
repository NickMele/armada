// Written by `scripts/record-job.mjs`, which rewrites it whole on every
// recording. Editing it by hand is lost on the next one.

import r0 from "./done-worktree-given-back/recording.json";

/** Every recording, by its directory. Replayed by `../recorded.ts`. */
export const RECORDINGS: Record<string, unknown> = {
  "done-worktree-given-back": r0,
};
