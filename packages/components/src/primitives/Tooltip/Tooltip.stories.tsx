import type { Meta, StoryObj } from "@storybook/react-vite";
import { Tooltip } from "./Tooltip";

const meta: Meta<typeof Tooltip> = {
  title: "Primitives/Tooltip",
  component: Tooltip,
};
export default meta;

type Story = StoryObj<typeof Tooltip>;

/**
 * The state the contract names first: a secondary value truncates in a row and
 * does not vanish, and the tooltip carries the full string. No field is
 * dropped at any width.
 */
export const TruncatedValue: Story = {
  args: {
    defaultOpen: true,
    label: "crates/api/src/session/refresh_coalescing.rs",
    children: (
      <span className="armada-tooltip__truncated">
        crates/api/src/session/refresh_coalescing.rs
      </span>
    ),
  },
};

/**
 * The consequence the keyboard section records: a tooltip gains a trailing kbd
 * where the action has a binding. The delay is unchanged by it.
 */
export const WithShortcut: Story = {
  args: {
    defaultOpen: true,
    label: "Approve dispatch",
    shortcut: "a",
    children: (
      <button type="button" className="armada-tooltip__action">
        Approve
      </button>
    ),
  },
};

/**
 * Closed. The 400ms delay is `--tooltip-delay` and the component reads it from
 * the token rather than carrying its own number — hover the value to see it.
 */
export const Resting: Story = {
  args: {
    label: "crates/api/src/session/refresh_coalescing.rs",
    children: (
      <span className="armada-tooltip__truncated">
        crates/api/src/session/refresh_coalescing.rs
      </span>
    ),
  },
};

const longPath = "crates/api/src/session/refresh_coalescing.rs";

/**
 * A frame that puts the trigger against an edge, so the flip is visible at a
 * size that fits on the page. `contain: layout` makes the frame the bubble's
 * containing block, which is the window in the app.
 */
function Frame({ edge, children }: { edge: "left" | "right" | "bottom"; children: React.ReactNode }) {
  return (
    <div className={`armada-tooltip-frame armada-tooltip-frame--${edge}`}>{children}</div>
  );
}

/**
 * On the leading edge, where the bubble's own alignment already fits. Nothing
 * flips — the placement is the preference and only the edge overrides it.
 */
export const AtTheLeftEdge: Story = {
  render: () => (
    <Frame edge="left">
      <Tooltip defaultOpen label={longPath}>
        <span className="armada-tooltip__truncated">{longPath}</span>
      </Tooltip>
    </Frame>
  ),
};

/**
 * On the trailing edge. A path is long and the bubble is wide, so a
 * leading-aligned bubble would run past the window; the alignment flips.
 */
export const AtTheRightEdge: Story = {
  render: () => (
    <Frame edge="right">
      <Tooltip defaultOpen label={longPath}>
        <span className="armada-tooltip__truncated">{longPath}</span>
      </Tooltip>
    </Frame>
  ),
};

/** No room below — the bubble opens above the value it describes. */
export const WithNoRoomBelow: Story = {
  render: () => (
    <Frame edge="bottom">
      <Tooltip defaultOpen label={longPath}>
        <span className="armada-tooltip__truncated">{longPath}</span>
      </Tooltip>
    </Frame>
  ),
};
