import type { Guide } from "./guide";

/**
 * The two sentences `ProposalDoneWhen` drew under the criteria, one before
 * approval and one after. Both said what a criterion is for; which words this
 * job froze, and where each came from, stays on the rows themselves.
 */
export const GUIDE_CRITERIA: Guide = {
  number: 10,
  group: "run",
  title: "What does the judge check?",
  piece: "run.criteria",
  concept: "docs/concepts/judge.md",
  steps: [
    "A criterion is one thing the work has to be true of.",
    "The Judge marks against these words and answers one criterion at a time.",
    "It writes no verdict on the job as a whole.",
    "The criteria are read out of the issue the work is linked to, where there is one.",
    "They are yours to change until you approve.",
    "Approving freezes them, and the job is held to the words as they were at that moment.",
    "An issue edited afterwards moves nothing.",
    "A Judge may only refuse.",
    "A Judge with no objection writes nothing, so silence on a criterion is a pass.",
  ],
};
