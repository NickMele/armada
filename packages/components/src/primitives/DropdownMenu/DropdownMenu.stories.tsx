import type { Meta, StoryObj } from "@storybook/react-vite";
import { DropdownMenu, type DropdownMenuEntry } from "./DropdownMenu";

const meta: Meta<typeof DropdownMenu> = {
  title: "Primitives/DropdownMenu",
  component: DropdownMenu,
};
export default meta;

type Story = StoryObj<typeof DropdownMenu>;

/**
 * The row menu, in the item order the component sheet draws: what the row
 * could also do, never a repeat of the row's own button, with Kill last and
 * below a rule. The binding sits right-aligned as a kbd and does not change
 * the item height.
 *
 * The trigger is label-only. The sheet draws a three-dot icon button, and no
 * ellipsis glyph is in the icon registry; see the report.
 */
export const RowMenu: Story = {
  args: {
    defaultOpen: true,
    triggerLabel: "More",
    entries: [
      { kind: "item", id: "worktree", label: "Open the worktree" },
      { kind: "item", id: "copy", label: "Copy job ID", shortcut: "⌘C" },
      { kind: "item", id: "board", label: "Send back to the Job Board" },
      { kind: "separator", id: "rule" },
      { kind: "item", id: "kill", label: "Kill job", shortcut: "x", danger: true },
    ],
  },
};

/** The section label the contract specifies: --text-2xs in --fg-subtle. */
export const WithSectionLabels: Story = {
  args: {
    defaultOpen: true,
    triggerLabel: "More",
    entries: [
      { kind: "label", id: "l-job", label: "This job" },
      { kind: "item", id: "worktree", label: "Open the worktree" },
      { kind: "item", id: "copy", label: "Copy job ID", shortcut: "⌘C" },
      { kind: "separator", id: "rule" },
      { kind: "label", id: "l-fleet", label: "Fleet" },
      { kind: "item", id: "pause", label: "Freeze dispatch" },
      { kind: "item", id: "kill", label: "Kill job", shortcut: "x", danger: true },
    ],
  },
};

const rowActions: DropdownMenuEntry[] = [
  { kind: "item", id: "worktree", label: "Open the worktree" },
  { kind: "item", id: "copy", label: "Copy job ID", shortcut: "⌘C" },
  { kind: "item", id: "board", label: "Send back to the Job Board" },
  { kind: "separator", id: "rule" },
  { kind: "item", id: "kill", label: "Kill job", shortcut: "x", danger: true },
];

/**
 * A frame that puts the trigger against an edge. The panel is anchor-positioned
 * and flips against its containing block, and `contain: layout` on the frame
 * makes the frame that block — so the story shows the same flip a window edge
 * produces, at a size that fits on the page.
 */
function Frame({ edge, children }: { edge: "left" | "right" | "bottom"; children: React.ReactNode }) {
  return (
    <div className={`armada-dropdown-menu-frame armada-dropdown-menu-frame--${edge}`}>
      {children}
    </div>
  );
}

/**
 * The reported failure. The trigger sits on the leading edge, where a
 * right-aligned menu would open off the screen, so the alignment flips and the
 * menu opens towards the space there is.
 */
export const AtTheLeftEdge: Story = {
  render: () => (
    <Frame edge="left">
      <DropdownMenu defaultOpen triggerLabel="More" entries={rowActions} />
    </Frame>
  ),
};

/**
 * The same menu with room on its own side. Nothing flips: the trigger's
 * alignment is the preference and only the edge overrides it.
 */
export const AtTheRightEdge: Story = {
  render: () => (
    <Frame edge="right">
      <DropdownMenu defaultOpen triggerLabel="More" entries={rowActions} />
    </Frame>
  ),
};

/**
 * No room below — the last row of a list. The menu opens above the trigger
 * rather than squashing to fit, because a menu that shortens itself hides
 * items without saying so.
 */
export const WithNoRoomBelow: Story = {
  render: () => (
    <Frame edge="bottom">
      <DropdownMenu defaultOpen triggerLabel="More" entries={rowActions} />
    </Frame>
  ),
};
