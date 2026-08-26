import type { Meta, StoryObj } from "@storybook/react-vite";
import { Popover } from "./Popover";

const meta: Meta<typeof Popover> = {
  title: "Primitives/Popover",
  component: Popover,
};
export default meta;

type Story = StoryObj<typeof Popover>;

const trigger = (
  <button type="button" className="armada-popover__button">
    Spend
  </button>
);

/**
 * The contract names no popover state. It names the surface and the fact that
 * a shadow is legal on it. This is that surface, open, carrying the kind of
 * content a popover holds rather than a tooltip: something you read at your
 * own pace, with a machine value in mono.
 */
export const Open: Story = {
  args: {
    defaultOpen: true,
    trigger,
    children: (
      <span>
        Spend follows the active billing mode. This machine gates on the quota
        floor, so the row shows <span className="mono">68% quota</span> left.
      </span>
    ),
  },
};

export const AlignedToTheEnd: Story = {
  args: {
    defaultOpen: true,
    align: "end",
    trigger,
    children: (
      <span>
        Spend follows the active billing mode. This machine gates on the quota
        floor, so the row shows <span className="mono">68% quota</span> left.
      </span>
    ),
  },
};
