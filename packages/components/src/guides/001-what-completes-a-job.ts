import type { Guide } from "./guide";

/**
 * Land draws the rule this job carries — *Completes when its pull request
 * lands*. That sentence is a fact about this job. What a landing rule is, and
 * that there are four of them, is not, so it is here.
 */
export const GUIDE_COMPLETION: Guide = {
  number: 1,
  group: "job",
  title: "What completes a job",
  piece: "land.completion",
  concept: "docs/concepts/landing.md",
  body: [
    "A job completes when its landing rule is met. The rule is chosen when the job is dispatched " +
      "and frozen from then on, so nothing about it moves while the job runs.",
    "There are four rules: the pull request merged, the pull request opened, every member landed, " +
      "and the delivering step delivered. Land says which one this job carries.",
    "Completing is not the same as ending. A job that was killed, rejected or escalated ended " +
      "without its rule being met, and Land still draws what it left behind.",
  ],
};
