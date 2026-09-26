import type { Guide } from "./guide";

/**
 * The line that used to stand over the group controls on the Plan board
 * (`ASKS_SAY`, `#1552`). It is the only place the rule was written down: the
 * controls read as edits, and none of them edits anything.
 */
export const GUIDE_PLAN_ASKS: Guide = {
  number: 6,
  group: "plan",
  title: "How do I change the plan?",
  piece: "plan.asks",
  concept: "docs/concepts/plan.md",
  steps: [
    "The plan is the Drone's record of how it means to work.",
    "Nothing on this screen edits it.",
    "Move up, Move down, Remove and Rewrite this task are requests.",
    "Each one reaches the Drone that wrote the plan, and it may refuse.",
    "A refusal is drawn under the ask, in the Drone's own words, and the plan below stays as it was.",
    "The asks are offered while the step that recorded the plan is waiting on a person.",
    "Past that gate the plan is a record of what was run.",
    "Two groups claiming one file have an order between them their own scopes decide.",
    "That is the one contradiction Armada can catch mechanically, and it is drawn above the cards.",
  ],
};
