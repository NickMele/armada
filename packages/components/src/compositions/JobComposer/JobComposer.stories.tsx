import type { Meta, StoryObj } from "@storybook/react-vite";
import { JobComposer } from "./JobComposer";

/**
 * The composer as the dispatch journey draws it, and the two shapes a workflow
 * gives its glance strip.
 *
 * The fixture is the drawing's own: `Coalesce concurrent token refreshes`, on
 * the `bug` workflow, in `armada`.
 */
const meta: Meta<typeof JobComposer> = {
  title: "Compositions/Job composer",
  component: JobComposer,
};
export default meta;

type Story = StoryObj<typeof JobComposer>;

/**
 * What M1 renders. Title, brief, workflow, project, then the two values that
 * can be known before a drone runs — and `Approve and dispatch`, the one
 * accent fill in the milestone.
 */
export const WhatM1Renders: Story = {
  args: {
    title: "Coalesce concurrent token refreshes",
    brief:
      "A burst of 401s should produce one refresh call, not one per request. Keep the retry ceiling where it is.",
    workflows: <option>bug — 4 steps</option>,
    project: "armada",
    glance: [
      { label: "Steps", value: "4 · 2 gated" },
      { label: "Checks", value: "build, test" },
    ],
    provenance: "Dispatched by you",
  },
};

/**
 * A workflow whose steps are all ungated. The Checks field still renders, and
 * says so in words: a blank would read as a strip that failed to fill, which
 * is the reading an ungated step exists to rule out everywhere else in Bridge.
 */
export const NoChecksOnTheWorkflow: Story = {
  args: {
    title: "Draft the release note",
    brief: "One paragraph per merged job since the last tag. No links out.",
    workflows: <option>note — 2 steps</option>,
    project: "armada",
    glance: [
      { label: "Steps", value: "2 · 0 gated" },
      { label: "Checks", value: "none" },
    ],
    provenance: "Dispatched by you",
  },
};
