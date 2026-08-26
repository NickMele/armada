import type { Meta, StoryObj } from "@storybook/react-vite";
import { Separator } from "./Separator";

/**
 * One value, two orientations, and no states. A rule does not hover and cannot
 * be focused.
 *
 * The margins in these stories belong to the surface, not to the separator —
 * a dropdown sets `--space-1` above and below its dividers, and a sidebar sets
 * more.
 */
const meta: Meta<typeof Separator> = {
  title: "Primitives/Separator",
  component: Separator,
  decorators: [
    (Story) => (
      <div style={{ maxWidth: "56ch" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof Separator>;

/** A dropdown menu's divider: Kill sits last, below a rule. */
export const Horizontal: Story = {
  render: () => (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        padding: "var(--space-1) 0",
        borderRadius: "var(--radius-lg)",
        background: "var(--bg-overlay)",
        fontSize: "var(--text-sm)",
      }}
    >
      <span style={{ height: "var(--h-menu-item)", padding: "0 var(--space-3)", lineHeight: "var(--h-menu-item)" }}>
        Open the worktree
      </span>
      <span style={{ height: "var(--h-menu-item)", padding: "0 var(--space-3)", lineHeight: "var(--h-menu-item)" }}>
        Send back to the Job Board
      </span>
      <div style={{ margin: "var(--space-1) 0" }}>
        <Separator />
      </div>
      <span
        style={{
          height: "var(--h-menu-item)",
          padding: "0 var(--space-3)",
          lineHeight: "var(--h-menu-item)",
          color: "var(--status-completed-failed)",
        }}
      >
        Kill job
      </span>
    </div>
  ),
};

/**
 * Between two groups of controls in a toolbar. Not the status bar: that
 * separates its fields with a middle dot in `--fg-subtle`, which is copy
 * rather than a component.
 */
export const Vertical: Story = {
  render: () => (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: "var(--space-3)",
        height: "var(--h-control)",
        color: "var(--fg-muted)",
        fontSize: "var(--text-sm)",
      }}
    >
      <span>Filter</span>
      <Separator orientation="vertical" />
      <span>Sort</span>
      <Separator orientation="vertical" />
      <span>Group</span>
    </div>
  ),
};

/**
 * Announced rather than decorative, for the sidebar rule that is the only
 * thing stating Bridge's surfaces end and Helm begins.
 */
export const Announced: Story = {
  args: { decorative: false },
};
