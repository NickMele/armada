import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { OverviewSurfaceFrom } from "./OverviewSurface";

/**
 * Overview's whole surface — the tile band over the lists, inside the shell with Helm's dock open
 * — so the arrangement can be checked against the drawing it was measured against: a gap between
 * every panel, every corner rounded, nothing against the window's edge. #948.
 */
const meta = {
  title: "Screens/Overview surface",
  component: OverviewSurfaceFrom,
  parameters: { layout: "padded" },
} satisfies Meta<typeof OverviewSurfaceFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WholeSurface: Story = {
  name: "Whole surface",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Needs you")).toBeVisible();
    await expect(canvas.getByText("Running")).toBeVisible();
    await expect(canvas.getByText("Queued")).toBeVisible();
    await expect(canvas.getByRole("complementary", { name: "Helm" })).toBeVisible();
  },
};
