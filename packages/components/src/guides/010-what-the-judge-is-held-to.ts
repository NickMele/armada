import type { Guide } from "./guide";

/**
 * The two sentences `ProposalDoneWhen` drew under the criteria, one before
 * approval and one after. Both said what a criterion is for; which words this
 * job froze, and where each came from, stays on the rows themselves.
 */
export const GUIDE_CRITERIA: Guide = {
  number: 10,
  group: "run",
  title: "What the Judge is held to",
  piece: "run.criteria",
  concept: "docs/concepts/judge.md",
  body: [
    "A criterion is one thing the work has to be true of. The Judge marks against these words and " +
      "answers per criterion, never overall.",
    "They are read out of the issue the work is linked to where there is one, and they are yours " +
      "to change until you approve. Approving freezes them: the job is held to the words as they " +
      "were at that moment, and an issue edited afterwards moves nothing.",
    "A Judge may only refuse. A Judge with no objection writes nothing, so silence on a criterion " +
      "is a pass rather than a missing answer.",
  ],
};
