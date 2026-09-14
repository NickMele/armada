import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { RUNNING_ONE_WITH_A_PLAN } from "../OverviewLists/OverviewLists";
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
    // `getByRole("heading", …)`, not `getByText` — the tile band has its own
    // "Queued" tile, and a panel's own head is a heading where a tile's label
    // is not.
    await expect(canvas.getByRole("heading", { name: "Needs you" })).toBeVisible();
    await expect(canvas.getByRole("heading", { name: "Running" })).toBeVisible();
    await expect(canvas.getByRole("heading", { name: "Queued" })).toBeVisible();
    await expect(canvas.getByRole("complementary", { name: "Helm" })).toBeVisible();
  },
};

/**
 * The Running panel beside Helm's open dock, at the shell's own width — one row with a plan and
 * one without. The owner's own report: the action went missing at 1284px here, because the row's
 * fixed-floor facts overflowed a box this narrow. The row keeps every fact and wraps rather than
 * losing the action.
 */
export const RunningPanelBesideDock: Story = {
  name: "Running panel beside Helm's dock",
  args: { jobs: RUNNING_ONE_WITH_A_PLAN },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getAllByRole("button", { name: /Redirect/ })[0]).toBeVisible();
  },
};
