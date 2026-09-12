import type { Meta, StoryObj } from "@storybook/react-vite";

import { running } from "../../../fixtures/build/running";
import { recorded } from "../../../fixtures/recorded";
import { WorkingGroupedFrom } from "./WorkingGrouped";

/**
 * The Working body with the activity folded by kind — the first of two shapes for
 * the owner to choose between.
 *
 * **Working draws its two products as peers**: what the Drone did, and what came
 * out of it. The footprint is a part of its own with the same chrome, not a row
 * at the foot of a list two hundred rows long.
 *
 * **The question the activity part answers is "what did the Drone do".**
 * Consecutive calls of one tool collapse to a heading with a count and their
 * added-up time, a failure opens itself, and the rows this Bridge cannot draw
 * are counted rather than listed. What it cannot answer is where the time went:
 * every row is the same height whether it took 200ms or four minutes.
 *
 * **`Open the diff` draws nothing yet.** The panel proper opens the diff sheet
 * there, and a draft that built it would be building the thing before it is
 * agreed.
 */
const meta = {
  title: "Drafts/Working grouped",
  component: WorkingGroupedFrom,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof WorkingGroupedFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/**
 * A real step off a real Job: 1763 rows, 351 calls, 36 files, 43m 36s.
 *
 * **The only honest test of a folding rule.** A built fixture carries three
 * turns, and a body that reads well with three says nothing about one that has
 * to hold the owner's own step.
 */
export const RecordedStep: Story = {
  name: "Recorded, a real step",
  args: { fixture: recorded("done-worktree-given-back"), stepId: "implement" },
};

/** A small step, where the folding has almost nothing to do. */
export const SmallStep: Story = {
  name: "A small step",
  args: { fixture: running(), stepId: "repro" },
};
