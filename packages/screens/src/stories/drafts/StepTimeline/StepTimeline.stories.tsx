import type { Meta, StoryObj } from "@storybook/react-vite";

import { escalatedGateFailure } from "../../../fixtures/build/escalated";
import { retryingCheckFailure } from "../../../fixtures/build/gating";
import { running } from "../../../fixtures/build/running";
import { recorded } from "../../../fixtures/recorded";
import { StepTimelineFrom } from "./StepTimeline";

/**
 * The step panel as one timeline, drawn from the same wire data `Screens/Job
 * detail` draws — a draft of what would replace `Where this step is` and the
 * chapters under it.
 *
 * **A draft, so it is deliberately thin below the surface.** Each row opens to
 * a short reading rather than to the chapter it would open to in the panel. The
 * question it is asking is whether the shape is right: one list, a row per
 * phase, repeated where the step was handed back, with earlier attempts folded
 * rather than hinted at.
 */
const meta = {
  title: "Drafts/Step timeline",
  component: StepTimelineFrom,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof StepTimelineFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** One attempt, and the Drone is in it. */
export const Working: Story = { name: "Working", args: { fixture: running() } };

/** Handed back once by a Check, and working again — the owner's second sketch. */
export const HandedBack: Story = {
  name: "Handed back",
  args: { fixture: retryingCheckFailure() },
};

/**
 * A step that finished, with what it wrote under the turns that wrote it —
 * the owner's call that Working is the activity log and Produced together.
 */
export const WroteFiles: Story = {
  name: "Wrote files",
  args: { fixture: running(), stepId: "repro" },
};

/**
 * A real step off a real Job: 1763 turns and 36 files in one attempt.
 *
 * **The honest test of the shape.** The built fixtures carry two or three
 * turns and one file, and the owner's own Job wrote 42 in a step — a row that
 * reads well with one file says nothing about what this panel has to hold.
 */
export const RecordedStep: Story = {
  name: "Recorded, a real step",
  args: { fixture: recorded("done-worktree-given-back"), stepId: "implement" },
};

/** Three attempts, the last of them stopped at the gate. */
export const ThreeAttempts: Story = {
  name: "Three attempts",
  args: { fixture: escalatedGateFailure() },
};
