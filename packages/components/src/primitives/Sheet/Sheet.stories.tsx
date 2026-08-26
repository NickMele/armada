import type { Meta, StoryObj } from "@storybook/react-vite";
import { Sheet } from "./Sheet";

const meta: Meta<typeof Sheet> = {
  title: "Primitives/Sheet",
  component: Sheet,
};
export default meta;

type Story = StoryObj<typeof Sheet>;

/**
 * The contract names no Sheet state. It names one surface treatment and no
 * side, no width, no use. This story exists so the surface treatment can be
 * seen; what a sheet is for on Bridge is in the report as an open item — the
 * layout model says full-width routes with no inspector pane and no modal for
 * job detail, which rules out the two things a sheet usually does.
 */
export const Right: Story = {
  args: {
    open: true,
    side: "right",
    title: "Kit allowlist",
    children:
      "The command tripped the allowlist 5 times and was approved every time. Adding it here stops the prompt.",
  },
};

export const Left: Story = {
  args: {
    open: true,
    side: "left",
    title: "Kit allowlist",
    children:
      "The command tripped the allowlist 5 times and was approved every time. Adding it here stops the prompt.",
  },
};
