import type { Guide } from "./guide";

/**
 * The only guide in its group, and what the group was missing. Guide 11 filed
 * here explained how to read the plan's graph; the graph moved to the Plan tab
 * on 25 September 2026 and the guide was retired with it.
 *
 * Written from `docs/concepts/workflow.md`. The three gate boxes are decided
 * and not built, and the guide says what the boxes do rather than that they
 * exist yet.
 */
export const GUIDE_WORKFLOW: Guide = {
  number: 19,
  group: "workflow",
  title: "What is a workflow?",
  piece: "workflow.what",
  concept: "docs/concepts/workflow.md",
  steps: [
    "A workflow is the template a job runs against.",
    "It is a set of steps, in order.",
    "Each step carries its own Checks, its own Judge checks and its own gate.",
    "The gate says what advances the step: Checks, a Judge, a person, or the submission itself.",
    "The workflow freezes into the job when the job is created.",
    "So the yardstick cannot move under the work.",
    "Coding workflows run their steps once, in sequence.",
    "Planning workflows loop between draft and feedback until they converge or hit their cap.",
  ],
  // Under the line that names the order, the same drawing guide 15 carries.
  figure: { id: "workflow-steps", at: 2 },
};
