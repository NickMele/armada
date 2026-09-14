import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { SettingsSurfaceFrom } from "./SettingsSurface";

/**
 * Settings' whole surface — Fleet's four limits and this machine's own
 * settings, inside the shell with Helm's dock open. #1089.
 */
const meta = {
  title: "Screens/Settings surface",
  component: SettingsSurfaceFrom,
  parameters: { layout: "padded" },
} satisfies Meta<typeof SettingsSurfaceFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WholeSurface: Story = {
  name: "Whole surface",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { name: "Fleet" })).toBeVisible();
    await expect(canvas.getByRole("heading", { name: "This machine" })).toBeVisible();
    await expect(canvas.getByRole("complementary", { name: "Helm" })).toBeVisible();
  },
};
