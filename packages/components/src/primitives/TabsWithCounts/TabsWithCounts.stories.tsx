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
