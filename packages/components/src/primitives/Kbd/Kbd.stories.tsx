import type { Meta, StoryObj } from "@storybook/react-vite";
import { Kbd, KbdChord } from "./Kbd";

/**
 * kbd has one rendering and no states: it is reference material, never a
 * control, so it has no hover, no focus and no pressed appearance.
 *
 * Its three contexts — a palette row, a dropdown-menu item's right-aligned
 * shortcut, and a tooltip's trailing binding — belong to those components and
 * are drawn there, not here.
 */
const meta: Meta<typeof Kbd> = {
  title: "Primitives/kbd",
  component: Kbd,
};
export default meta;

type Story = StoryObj<typeof Kbd>;

export const Default: Story = {
  args: { children: "Esc" },
};

/** A chord is one box per key. `⌘K` in a single box reads as a key. */
export const Chord: StoryObj = {
  render: () => (
    <KbdChord>
      <Kbd>⌘</Kbd>
      <Kbd>K</Kbd>
    </KbdChord>
  ),
};

/**
 * The contextual tier: single keys that act on the focused row, which is what
 * makes triage fast. Kill is `x` and never `k`, because `k` sits against `j`
 * and a mistyped navigation keystroke must not be able to end a running job.
 */
export const ContextualKeys: StoryObj = {
  render: () => (
    <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
      <Kbd>j</Kbd>
      <Kbd>k</Kbd>
      <Kbd>Enter</Kbd>
      <Kbd>a</Kbd>
      <Kbd>r</Kbd>
      <Kbd>x</Kbd>
      <Kbd>/</Kbd>
    </div>
  ),
};
