import type { Guide } from "./guide";

/**
 * The three links a member can carry, off `JobMembers.MEMBER_LINK`. The card
 * still says which link this member has — that is a fact about it. What the
 * three *are*, and that they differ in what happens while the one before is
 * still open, is here.
 */
export const GUIDE_MEMBER_LINK: Guide = {
  number: 2,
  group: "job",
  title: "How a member reaches the one before it",
  piece: "members.link",
  concept: "docs/concepts/landing.md",
  body: [
    "Several pull requests landing in order are one job with members. Each member is a job of " +
      "its own, and the order between them is the reason they were dispatched together.",
    "A member is stacked, parked or waiting on a release. Stacked keeps working while the branch " +
      "under it is still open, and rebases once that one lands.",
    "Parked stops until the one before it lands, so nothing is rebased and nothing is lost. " +
      "Waiting on a release is the third: it waits on what the merge before it publishes rather " +
      "than on the merge itself, which is the case where a version has to exist first.",
    "Dropping a member closes its pull request, keeps its branch, and rebases whatever was " +
      "stacked on it onto the one before.",
  ],
};
