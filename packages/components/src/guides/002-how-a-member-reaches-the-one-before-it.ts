import type { Guide } from "./guide";

/**
 * The three links a member can carry, off `JobMembers.MEMBER_LINK`. The card
 * still says which link this member has — that is a fact about it. What the
 * three *are*, and that they differ in what happens while the one before is
 * still open, is here.
 *
 * **One of two guides written in all three shapes**, for the comparison in
 * `guide-shape.tsx`. *Members landing in order* is one of the three relations
 * `docs/contracts/design-system.md` names as worth drawing, so this is the
 * guide both new shapes get their animation from.
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
  shapes: {
    steps: {
      lines: [
        "Several pull requests landing in order are one job with members.",
        "Each member is a job of its own, and their order is why they were dispatched together.",
        "Every member carries one of three links to the one before it.",
        "Stacked keeps working while the branch under it is still open, and rebases once that one lands.",
        "Parked stops until the one before it lands, so nothing is rebased and nothing is lost.",
        "Waiting on a release waits on what that merge publishes rather than on the merge itself — " +
          "the case where a version has to exist first.",
        "Dropping a member closes its pull request, keeps its branch, and rebases whatever was " +
          "stacked on it onto the one before.",
      ],
      // Under the line that names the order, because the order is the relation.
      figure: { id: "members-landing", at: 2 },
    },
    figure: {
      id: "members-landing",
      captions: [
        "Several pull requests landing in order are one job with members, each a job of its own.",
        "The order between them is the reason they were dispatched together.",
        "Stacked keeps working while the branch under it is still open, and rebases once that one lands.",
        "Parked stops until the one before it lands, so nothing is rebased and nothing is lost.",
        "Waiting on a release waits on what that merge publishes rather than on the merge itself — " +
          "the case where a version has to exist first.",
        "Dropping a member closes its pull request, keeps its branch, and rebases whatever was " +
          "stacked on it onto the one before.",
      ],
    },
  },
};
