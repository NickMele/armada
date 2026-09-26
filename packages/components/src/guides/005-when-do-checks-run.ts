import type { Guide } from "./guide";

/**
 * What the bar under a group is, and why a boundary nobody reached says
 * nothing rather than `0 failed` — the owner's rule that a screen never says a
 * thing it does not know.
 */
export const GUIDE_GROUP_BOUNDARY: Guide = {
  number: 5,
  group: "plan",
  title: "When do checks run?",
  piece: "plan.group-boundary",
  concept: "docs/concepts/plan.md",
  steps: [
    "Checks run at the end of a group, never once over the whole step.",
    "That end is the group's boundary, and it is where the work is checked.",
    "Fleet runs the Checks the repository's Manifest declares.",
    "The bar at the boundary carries one segment for each Check.",
    "A boundary nothing has reached reads as nothing.",
    "There is no verdict before anything has been checked, and a green count there would be a " +
      "claim rather than a reading.",
    "Passing leaves a commit.",
    "Failing stops the step where it stands.",
    "The next Drone is given the failed Check's own output rather than a summary of it.",
  ],
};
