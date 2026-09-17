import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { StatsPanel, type StatRow } from "./StatsPanel";

/**
 * Stats — the left column's second panel. Rows are the caller's; nothing here
 * counts a Job on its own. `Bridge/1088`.
 */
const meta: Meta<typeof StatsPanel> = {
  title: "Compositions/StatsPanel",
  component: StatsPanel,
};
export default meta;

type Story = StoryObj<typeof StatsPanel>;

const ROWS: StatRow[] = [
  { id: "approval", label: "Awaiting approval", value: 1, tone: "warn", hue: "status-awaiting-review", idle: false },
  { id: "review", label: "Needs review", value: 1, tone: "warn", hue: "status-awaiting-review", idle: false },
  { id: "escalated", label: "Escalated", value: 1, tone: "hot", hue: "status-escalated", idle: false },
  { id: "jobs", label: "Jobs", value: 10, hue: "status-not-started", idle: false },
  {
    id: "drones",
    label: "Drones",
    value: "2 of 4",
    hint: "Fleet runs up to 4 drones at once. 2 are working now.",
    hue: "stat-drones",
    idle: false,
  },
  { id: "manifest", label: "Manifest", value: "1 of 2 behind", tone: "warn", hue: "notice-caution", idle: false },
];

export const NeedsAttention: Story = {
  args: { rows: ROWS, open: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Escalated")).toBeVisible();
    await expect(canvas.getByText("1 of 2 behind")).toBeVisible();
  },
};

/** A quiet day: every dot dim in its own hue. */
export const NothingWaiting: Story = {
  args: {
    rows: [
      { id: "approval", label: "Awaiting approval", value: 0, hue: "status-awaiting-review", idle: true },
      { id: "review", label: "Needs review", value: 0, hue: "status-awaiting-review", idle: true },
      { id: "escalated", label: "Escalated", value: 0, hue: "status-escalated", idle: true },
      { id: "jobs", label: "Jobs", value: 0, hue: "status-not-started", idle: true },
      { id: "drones", label: "Drones", value: "0 of 2", hue: "stat-drones", idle: true },
      { id: "manifest", label: "Manifest", value: "Current", hue: "stat-manifest-current", idle: true },
    ],
    open: true,
  },
};

export const Collapsed: Story = {
  args: { rows: ROWS, open: false },
};

export const Narrow: Story = {
  args: { rows: ROWS, open: true, narrow: true },
};
