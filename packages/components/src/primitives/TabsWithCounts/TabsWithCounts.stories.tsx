import type { Meta, StoryObj } from "@storybook/react-vite";
import { TabsWithCounts } from "./TabsWithCounts";

const meta: Meta<typeof TabsWithCounts> = {
  title: "Primitives/Tabs with counts",
  component: TabsWithCounts,
};
export default meta;

type Story = StoryObj<typeof TabsWithCounts>;

/**
 * Separate queues. Alerts and Reviews are backlogs and carry a count; Activity
 * is a stream and carries none — it always has items and none of them want
 * anything, so a number there would read as work outstanding.
 */
export const Queues: Story = {
  args: {
    defaultValue: "alerts",
    items: [
      { id: "alerts", label: "Alerts", count: 4 },
      { id: "reviews", label: "Reviews", count: 3 },
      { id: "activity", label: "Activity" },
    ],
  },
};

/**
 * Zero renders as no count, not as `0`. An empty queue is the resting state of
 * a healthy fleet; the absence is the signal, and the tab's own empty state
 * says what the fleet did instead.
 */
export const Zero: Story = {
  args: {
    defaultValue: "alerts",
    items: [
      { id: "alerts", label: "Alerts", count: 0 },
      { id: "reviews", label: "Reviews", count: 3 },
      { id: "activity", label: "Activity" },
    ],
  },
};

/**
 * The Job Board's five, each carrying the key that selects it. **The key is
 * bordered and the count is not** — both are numerals in the same row, so the
 * chip is what separates the one you press from the number of jobs behind it,
 * and the label sits between them.
 *
 * `All` carries a count too, which the rule about backlogs does not forbid:
 * every one of these is a queue of work, and `All` is the sum rather than a
 * stream. The two numbers a person reads off this strip are how many need them
 * and how many exist, and dropping the second would leave the first with
 * nothing to be a fraction of.
 */
export const TheBoard: Story = {
  args: {
    defaultValue: "all",
    items: [
      { id: "all", label: "All", count: 15, shortcut: "1" },
      { id: "needs-you", label: "Needs you", count: 4, shortcut: "2" },
      { id: "running", label: "Running", count: 6, shortcut: "3" },
      { id: "queued", label: "Queued", count: 2, shortcut: "4" },
      { id: "finished", label: "Finished", count: 3, shortcut: "5" },
    ],
  },
};

/**
 * Suspended: on screen, still selected, and narrowing nothing — because
 * something else on the surface is. The Job Board sets this while its search
 * field holds text.
 *
 * **Set back, never disabled.** `Running` is still the tab a person chose and
 * still says so; it has only given up the accent underline, which would
 * otherwise claim to be what is on screen. Pressing a tab is how the state is
 * left, so every tab stays pressable — a disabled strip would make the way out
 * the thing that does not work.
 */
export const Suspended: Story = {
  args: {
    defaultValue: "running",
    suspended: true,
    items: [
      { id: "all", label: "All", count: 3, shortcut: "1" },
      { id: "needs-you", label: "Needs you", count: 1, shortcut: "2" },
      { id: "running", label: "Running", count: 2, shortcut: "3" },
      { id: "queued", label: "Queued", shortcut: "4" },
      { id: "finished", label: "Finished", shortcut: "5" },
    ],
  },
};
