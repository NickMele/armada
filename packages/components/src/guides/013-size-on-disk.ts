import type { Guide } from "./guide";

/**
 * Why Pulse draws a figure per checkout and no total. A total would be a fact
 * about the job, and disk is held by a checkout — which is also what makes
 * reclaiming one the act that gives the disk back.
 */
export const GUIDE_WORKTREE_SIZE: Guide = {
  number: 13,
  group: "machine",
  title: "Size on disk is a fact about a checkout",
  piece: "pulse.worktree-size",
  concept: "docs/concepts/job.md",
  body: [
    "Each checkout this job holds carries its own size, and there is no total under them. Disk is " +
      "taken by a checkout rather than by a job, so a sum would be a figure about nothing.",
    "A row with no figure is a walk that did not finish inside its bound. It is never zero, and a " +
      "zero would be the one reading that says the checkout is empty.",
    "Reclaiming a checkout takes the disk back and leaves the branch and the record. Cleanup is " +
      "where that is done across every job at once.",
  ],
};
