import type { Meta, StoryObj } from "@storybook/react-vite";

import { running } from "../../../fixtures/build/running";
import { recorded } from "../../../fixtures/recorded";
import { WorkingWaterfallFrom } from "./WorkingWaterfall";

/**
 * The Working body as a waterfall — the second of two shapes for the owner to
 * choose between.
 *
 * **Working draws its two products as peers**: what the Drone did, and what came
 * out of it. That matters more here than in the folded draft — a write takes no
 * time, so on a clock it is one tick between two calls and can only be a part of
 * its own. The Produced part is identical in both drafts on purpose.
 *
 * **The question the lane answers is "where did the time go".** One clock across
 * the top, a bar per act placed where it happened and as wide as it took, a
 * collapse by tool, and a filter down to what failed. On the recorded step it
 * reports something no folded list can: 35 of the 43 minutes had nothing open
 * at all.
 *
 * **`Open the diff` draws nothing yet**, the same as in the folded draft.
 */
const meta = {
  title: "Drafts/Working waterfall",
  component: WorkingWaterfallFrom,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof WorkingWaterfallFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** A real step off a real Job: 351 calls over 43m 36s, 28 of them failures. */
export const RecordedStep: Story = {
  name: "Recorded, a real step",
  args: { fixture: recorded("done-worktree-given-back"), stepId: "implement" },
};

/** A small step, where a lane has almost nothing to place. */
export const SmallStep: Story = {
  name: "A small step",
  args: { fixture: running(), stepId: "repro" },
};
