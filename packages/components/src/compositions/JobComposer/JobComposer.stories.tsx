import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
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
    onCancel: fn(),
    onDispatch: fn(),
  },
  /**
   * **The project is not a control, and the workflow is.** Both are drawn at
   * the same height on the same line so their baselines agree, which is
   * exactly what makes the difference invisible: a disabled select would look
   * choosable and refuse, and this reads as the fact it is.
   *
   * Then the two acts, which are the pair worth pinning down because swapping
   * them draws identically and neither is recoverable. `Cancel` writes
   * `killed` — a job never dispatched was abandoned rather than stopped — and
   * `Approve and dispatch` lands it in `queued`.
   */
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("combobox", { name: "Workflow" })).toBeEnabled();
    await expect(canvas.queryByRole("combobox", { name: /Project/ })).toBeNull();
    await expect(canvas.getByText("armada")).toBeVisible();

    await userEvent.click(canvas.getByRole("button", { name: "Cancel" }));
    await expect(args.onCancel).toHaveBeenCalled();
    await expect(args.onDispatch).not.toHaveBeenCalled();

    await userEvent.click(canvas.getByRole("button", { name: "Approve and dispatch" }));
    await expect(args.onDispatch).toHaveBeenCalled();
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
