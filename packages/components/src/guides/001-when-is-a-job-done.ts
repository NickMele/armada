import type { Guide } from "./guide";

/**
 * Land draws the rule this job carries — *Completes when its pull request
 * lands*. That sentence is a fact about this job. What a landing rule is, and
 * that there are four of them, is not, so it is here.
 *
 * **No figure.** Four rules and a distinction; nothing relates to anything,
 * and there is no honest picture of a rule.
 */
export const GUIDE_COMPLETION: Guide = {
  number: 1,
  group: "job",
  title: "When is a job done?",
  piece: "land.completion",
  concept: "docs/concepts/landing.md",
  steps: [
    "A job is done when its landing rule is met.",
    "The rule is chosen when the job is dispatched, and frozen from then on.",
    "Nothing about it moves while the job runs.",
    "There are four rules: the pull request merged, the pull request opened, every member landed, " +
      "and the delivering step delivered.",
    "Land says which one this job carries.",
    "Done is not the same as ended.",
    "A job that was killed, rejected or escalated ended without its rule being met.",
    "Land still draws what it left behind.",
  ],
};
