import type { Guide } from "./guide";

/**
 * The run's own explanation of itself, moved off the screen. The bar not
 * pulsing is the part nobody guesses: the contract gives motion to what is
 * still working, and where the work got to is a static fact.
 */
export const GUIDE_STEP_BAR: Guide = {
  number: 8,
  group: "run",
  title: "How a step's bar fills",
  piece: "run.step-bar",
  concept: "docs/concepts/workflow.md",
  body: [
    "The run is the workflow's steps, in the order this job runs them. The order is the " +
      "workflow's and never changes.",
    "A bar carries one segment for each piece of work under it: a task in a group, a Check at a " +
      "boundary. A segment fills as its own piece stops, so the bar says where the work got to.",
    "The bar never moves on its own. Where the work got to is a static fact, and the mark beside " +
      "the step is the thing that says it is still running.",
  ],
};
