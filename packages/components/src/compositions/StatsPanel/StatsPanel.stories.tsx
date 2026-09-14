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
  { id: "approval", label: "Awaiting approval", value: 1, tone: "warn" },
  { id: "review", label: "Needs review", value: 1, tone: "warn" },
  { id: "escalated", label: "Escalated", value: 1, tone: "hot" },
  { id: "jobs", label: "Jobs", value: 10 },
  { id: "drones", label: "Drones", value: "2 of 4", hint: "Fleet runs up to 4 drones at once. 2 are working now." },
  { id: "manifest", label: "Manifest", value: "1 of 2 behind", tone: "warn" },
];

export const NeedsAttention: Story = {
  args: { rows: ROWS, open: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Escalated")).toBeVisible();
    await expect(canvas.getByText("1 of 2 behind")).toBeVisible();
  },
};

export const NothingWaiting: Story = {
  args: {
    rows: [
      { id: "approval", label: "Awaiting approval", value: 0 },
      { id: "review", label: "Needs review", value: 0 },
      { id: "escalated", label: "Escalated", value: 0 },
      { id: "jobs", label: "Jobs", value: 3 },
      { id: "drones", label: "Drones", value: "0 of 2" },
      { id: "manifest", label: "Manifest", value: "Current" },
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
