import type { Guide } from "./guide";

/**
 * How to read the workflow graph, which is a picture with a rule behind it —
 * one node for a group, drawn where it originated, with a second edge from the
 * step that worked it (the owner, 23 September 2026, `#1530`). The agent that
 * built the graph asked whether the plan step's card should say this; the
 * answer was no, because that is the prose the owner cut from every screen.
 *
 * **Its `?` is owed.** It goes beside the graph rather than beside a node —
 * what is being explained is how to read the picture — and the Workflow files
 * were another agent's while this landed.
 */
export const GUIDE_GROUP_EDGES: Guide = {
  number: 5,
  group: "workflow",
  title: "A group with two edges",
  piece: "workflow.group-edges",
  concept: "docs/concepts/workflow.md",
  body: [
    "On the graph a group hangs off the step whose plan wrote it. That first edge says the group " +
      "has been planned, and it is drawn from the moment the plan exists.",
    "A second edge arrives from the step that worked it. So a group with two edges has been " +
      "planned and worked, and a group with one is still waiting for its turn.",
    "A job that has only been planned draws no second edge at all. Four groups on one edge each " +
      "is what a plan looks like before any of it has been picked up.",
    "Edges cross as a job gets on. The group stays where it was written rather than moving to " +
      "whoever is working it, so where a group sits is always the step that thought of it.",
  ],
};
