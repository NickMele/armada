import type { Guide } from "./guide";

/**
 * What the bar under a group is, and why a boundary nobody reached says
 * nothing rather than `0 failed` — the owner's rule that a screen never says a
 * thing it does not know.
 */
export const GUIDE_GROUP_BOUNDARY: Guide = {
  number: 3,
  group: "plan",
  title: "A group's boundary",
  piece: "plan.group-boundary",
  concept: "docs/concepts/plan.md",
  body: [
    "The boundary is the end of a group, and it is where the work is checked. Fleet runs the " +
      "Checks the repository's Manifest declares, and the bar carries one segment for each.",
    "A boundary that has not run reads as nothing. There is no verdict to draw before anything " +
      "has been checked, and drawing a green count there would be a claim rather than a reading.",
    "Passing leaves a commit. Failing stops the step where it stands, and the next Drone is given " +
      "the failed Check's own output rather than a summary of it.",
  ],
};
