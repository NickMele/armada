import type { Guide } from "./guide";

/**
 * The run's own explanation of itself, moved off the screen. The bar not
 * pulsing is the part nobody guesses: the contract gives motion to what is
 * still working, and where the work got to is a static fact.
 */
export const GUIDE_STEP_BAR: Guide = {
  number: 8,
  group: "run",
  title: "What does the progress bar show?",
  piece: "run.step-bar",
  concept: "docs/concepts/workflow.md",
  steps: [
    "The run is the workflow's steps, in the order this job runs them.",
    "The order is the workflow's and never changes.",
    "A bar carries one segment for each piece of work under it.",
    "On a job that is a step; on a group that is a task; at a boundary that is a Check.",
    "A segment fills as its own piece stops, so the bar says where the work got to.",
    "The bar never moves on its own.",
    "Where the work got to is a static fact.",
    "The mark beside the step is what says the work is still running.",
  ],
  // Under the line that names the segment, which is what the drawing fills.
  figure: { id: "step-bar", at: 3 },
};
