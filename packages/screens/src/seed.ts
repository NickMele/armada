// What Setup says about `setup.seed`, on a Job's run sheet and on the Manifest
// surface. #1064. Fleet writes the reason a worktree started cold; this only
// frames it.

import type { DeclaredSeed, WorktreeSeeding } from "@armada/protocol";

const short = (commit: string): string => commit.slice(0, 12);

/** The run sheet's Setup line. Absent where the Job's Manifest declares no seed. */
export function seedingSaid(seeding: WorktreeSeeding | undefined): string | undefined {
  if (seeding === undefined) return undefined;
  switch (seeding.state) {
    case "seeded":
      return `This worktree was seeded with ${seeding.paths.join(", ")} from base ${short(seeding.commit)}.`;
    case "cold":
      return `This worktree started cold: ${seeding.why}.`;
    case "unrecorded":
      return "Nothing records whether this worktree was seeded.";
  }
}

/** The Manifest surface's Setup line. Absent where the Manifest declares no seed. */
export function seedSaid(seed: DeclaredSeed | undefined): string | undefined {
  if (seed === undefined) return undefined;
  const declared = `A new worktree is seeded with ${seed.paths.join(", ")}, warmed by ${seed.warmed_by.join(", ")}.`;
  switch (seed.warmth.state) {
    case "warm":
      return `${declared} Warm at base ${short(seed.warmth.commit)}.`;
    case "warming":
      return `${declared} Warming at base ${short(seed.warmth.commit)}, so a Job cut now starts cold.`;
    case "cold":
      return `${declared} Not warm: ${seed.warmth.why}.`;
  }
}
