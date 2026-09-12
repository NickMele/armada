import type { Meta, StoryObj } from "@storybook/react-vite";

import { escalatedGateFailure } from "../../../fixtures/build/escalated";
import { retryingCheckFailure } from "../../../fixtures/build/gating";
import { running } from "../../../fixtures/build/running";
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

/** Three attempts, the last of them stopped at the gate. */
export const ThreeAttempts: Story = {
  name: "Three attempts",
  args: { fixture: escalatedGateFailure() },
};
