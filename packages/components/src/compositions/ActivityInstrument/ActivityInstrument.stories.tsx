import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { ActivityInstrument } from "./ActivityInstrument";

/**
 * Tool calls per 30s window over the last twelve minutes, on a running Job.
 *
 * The descriptions below are written as `packages/screens/src/instruments.ts`
 * words them; the arithmetic is tested there, not here.
 */
const meta: Meta<typeof ActivityInstrument> = {
  title: "Compositions/Activity instrument",
  component: ActivityInstrument,
  args: { label: "Tool calls per 30 seconds", from: "12m ago", to: "now" },
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-dialog-wide)", maxWidth: "100%" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof ActivityInstrument>;

const QUIET = new Array<number>(24).fill(0);

/** No calls in twelve minutes: every window draws, as the empty mark. */
export const Empty: Story = {
  args: { windows: QUIET, description: "No tool calls for the last 12 minutes" },
  play: async ({ canvas }) => {
    const drawing = canvas.getByRole("img", { name: "Tool calls per 30 seconds" });
    await expect(drawing).toHaveAccessibleDescription("No tool calls for the last 12 minutes");
    // No peak on nothing: a scale with no bar on it is a number about nothing.
    await expect(canvas.queryByText(/^peak/)).toBeNull();
  },
};

/** A few calls, scattered. One call still draws taller than an empty window. */
export const Sparse: Story = {
  args: {
    windows: [0, 1, 0, 0, 2, 0, 0, 0, 1, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 2, 0, 1, 0],
    description: "11 tool calls in the last 12 minutes, none for the last 30 seconds",
  },
};

/** Busy, then nothing for four minutes: the flat tail the instrument is for. */
export const SilentTail: Story = {
  args: {
    windows: [9, 14, 11, 7, 12, 16, 10, 8, 13, 9, 6, 11, 4, 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
    description: "134 tool calls in the last 12 minutes, none for the last 4 minutes",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Tool calls per 30 seconds" })).toHaveAccessibleDescription(
      "134 tool calls in the last 12 minutes, none for the last 4 minutes",
    );
    await expect(canvas.getByText("peak 16")).toBeVisible();
  },
};

/** Every window busy, up to the one being drawn. */
export const Dense: Story = {
  args: {
    windows: [18, 22, 25, 19, 30, 27, 24, 31, 28, 20, 26, 33, 29, 21, 24, 30, 35, 27, 23, 29, 32, 26, 28, 31],
    description: "638 tool calls in the last 12 minutes, some in the last 30 seconds",
  },
};
