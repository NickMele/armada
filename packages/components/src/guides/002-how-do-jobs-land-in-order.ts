import type { Guide } from "./guide";

/**
 * The three links a member can carry, off `JobMembers.MEMBER_LINK`. The card
 * still says which link this member has — that is a fact about it. What the
 * three *are*, and that they differ in what happens while the one before is
 * still open, is here.
 *
 * *Members landing in order* is one of the three relations
 * `docs/contracts/design-system.md` names as worth drawing.
 */
export const GUIDE_MEMBER_LINK: Guide = {
  number: 2,
  group: "job",
  title: "How do jobs land in order?",
  piece: "members.link",
  concept: "docs/concepts/landing.md",
  steps: [
    "Several pull requests landing in order are one job with members.",
    "Each member is a job of its own, and their order is why they were dispatched together.",
    "Every member carries one of three links to the one before it.",
    "Stacked keeps working while the branch under it is still open, and rebases once that one lands.",
    "Parked stops until the one before it lands, so nothing is rebased and nothing is lost.",
    "Waiting on a release waits on what that merge publishes rather than on the merge itself.",
    "That is the case where a version has to exist first.",
    "Dropping a member closes its pull request, keeps its branch, and rebases whatever was " +
      "stacked on it onto the one before.",
  ],
  // Under the line that names the order, because the order is the relation.
  figure: { id: "members-landing", at: 2 },
};
