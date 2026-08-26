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
