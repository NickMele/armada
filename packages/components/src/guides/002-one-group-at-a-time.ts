import type { Guide } from "./guide";

/**
 * The rule the implement board's one standing sentence is the only evidence of
 * (`#1530`). It is true of a job that has never run, which is the test that
 * makes it a guide rather than a line on the screen.
 */
export const GUIDE_GROUP_ORDER: Guide = {
  number: 2,
  group: "plan",
  title: "One group at a time",
  piece: "plan.group-order",
  concept: "docs/concepts/plan.md",
  body: [
    "A step's plan is a list of tasks, gathered into groups. Every task in a group is worked " +
      "before that group is checked, and no task of the next group starts while this one is " +
      "being checked.",
    "A task reads done only once its group has gone green. Until then its own mark says where it " +
      "got to and nothing more.",
    "The order is the plan's. It is written before the work starts and does not change while the " +
      "step runs.",
  ],
};
