import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { OverviewSummaryFrom } from "./OverviewSummary";

/**
 * Overview's summary strip, drawn by the app's own `OverviewSummary` from Jobs in the shape
 * `list_jobs` sends. Only the data is made up. #1091.
 */
const meta = {
  title: "Screens/Overview summary",
  component: OverviewSummaryFrom,
  parameters: { layout: "padded" },
} satisfies Meta<typeof OverviewSummaryFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** The fixture roster, read the same way the panels beneath the strip read it. */
export const Default: Story = {
  args: { onJump: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: /Needs you/ })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: /Queued/ }));
    await expect(args.onJump).toHaveBeenCalledWith("queued");
  },
};

/** Nothing in scope: every count reads zero and subtle. */
export const NoJobs: Story = {
  name: "No jobs",
  args: { jobs: [] },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "0 Needs you" })).toBeVisible();
  },
};
