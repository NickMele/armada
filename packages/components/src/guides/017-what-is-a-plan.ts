import type { Guide } from "./guide";

/**
 * What a plan is, before any of the four guides about working one. Written
 * from `docs/concepts/plan.md`.
 *
 * `concurrent_with` and the fifth task state `failed` are decided and not
 * built, so neither is here.
 */
export const GUIDE_PLAN: Guide = {
  number: 17,
  group: "plan",
  title: "What is a plan?",
  piece: "plan.what",
  concept: "docs/concepts/plan.md",
  steps: [
    "A plan is a job's own record of how it means to do the work.",
    "It holds an approach in a paragraph, and an ordered list of tasks.",
    "A task carries a title, the paths it touches, and what should prove it.",
    "A task is open, working, done or dropped, and a dropped one carries the reason.",
    "The tasks sit in groups, and a group is what runs.",
    "One plan per job. A step writes it, and the steps after it keep it current.",
    "Every change is appended and records who made it, so nothing is overwritten.",
    "The plan is the Drone's record, which is why the screen asks rather than edits.",
  ],
  // Under the line that names the group, because the group is the relation.
  figure: { id: "group-order", at: 5 },
};
