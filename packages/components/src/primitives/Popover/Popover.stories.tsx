import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
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
  /**
   * Closed and opened again, because `defaultOpen` is the only state the rest
   * of this sheet ever draws — the trigger is a `span` wrapping whatever the
   * caller passed, so "the press reaches it" is a claim about event flow and
   * not about markup.
   *
   * Esc first: it is the exit a person reaches for when the panel is over the
   * thing they wanted to read.
   */
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("dialog")).toBeVisible();

    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog")).toBeNull();

    // And the control that opened it is the control that opens it again.
    await userEvent.click(canvas.getByRole("button", { name: "Spend" }));
    await expect(canvas.getByRole("dialog")).toBeVisible();
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

const body = (
  <span>
    Spend follows the active billing mode. This machine gates on the quota
    floor, so the row shows <span className="mono">68% quota</span> left.
  </span>
);

/**
 * A frame that puts the trigger against an edge, so the flip is visible at a
 * size that fits on the page. `contain: layout` makes the frame the panel's
 * containing block, which is the window in the app.
 */
function Frame({ edge, children }: { edge: "left" | "right" | "bottom"; children: React.ReactNode }) {
  return (
    <div className={`armada-popover-frame armada-popover-frame--${edge}`}>{children}</div>
  );
}

/**
 * Trailing-aligned against the leading edge — the reported failure. The panel
 * is wider than the space on that side, so the alignment flips.
 */
export const AtTheLeftEdge: Story = {
  render: () => (
    <Frame edge="left">
      <Popover defaultOpen align="end" trigger={trigger}>
        {body}
      </Popover>
    </Frame>
  ),
};

/**
 * Leading-aligned against the trailing edge — the same failure mirrored. The
 * requested alignment is kept wherever it fits and only the edge overrides it.
 */
export const AtTheRightEdge: Story = {
  render: () => (
    <Frame edge="right">
      <Popover defaultOpen align="start" trigger={trigger}>
        {body}
      </Popover>
    </Frame>
  ),
};

/** No room below. The panel opens above the trigger rather than squashing. */
export const WithNoRoomBelow: Story = {
  render: () => (
    <Frame edge="bottom">
      <Popover defaultOpen trigger={trigger}>
        {body}
      </Popover>
    </Frame>
  ),
};
