import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";

import { repository } from "../../../fixtures/build/base";
import { SCRATCH, SetupFrom } from "./Setup";

/**
 * The rail's project picker over every repository one Fleet serves — its own, set up, and a folder
 * added by path that nobody set up yet. Picking that one opens its Setup; a root Write lands on
 * Verify for it.
 */
const meta = {
  title: "Screens/Pick a repository",
  component: SetupFrom,
  parameters: { layout: "fullscreen" },
  args: { repositories: [repository(), SCRATCH], sheet: { state: "read", sheet: { setup: [], checks: [], commands: [] } } },
} satisfies Meta<typeof SetupFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Both repositories listed, the one not set up saying so, and the Fleet's own picked. */
export const BothListed: Story = {
  name: "A set-up and a not-set-up repository",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const picker = canvas.getByRole("combobox", { name: "Project" });
    await expect(within(picker).getByRole("option", { name: "armada" })).toBeInTheDocument();
    await expect(within(picker).getByRole("option", { name: "scratch · not set up" })).toBeInTheDocument();
    await expect(picker).toHaveValue("/Users/user/armada");
    await expect(canvas.queryByRole("region", { name: "Workspaces" })).toBeNull();
  },
};

/** Pick the folder nobody set up: Setup opens for it, Write puts its root file down, and Verify is there. */
export const PickNotSetUp: Story = {
  name: "Pick one not set up, write it, verify",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const picker = canvas.getByRole("combobox", { name: "Project" });
    await userEvent.selectOptions(picker, SCRATCH.root);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open ." }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for ." });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write armada.yml" }));
    const verify = await within(sheet).findByRole("region", { name: "Verify" });
    await expect(within(verify).getByRole("button", { name: "Verify" })).toBeEnabled();
    // Written, so the picker no longer calls it not set up.
    await expect(within(picker).getByRole("option", { name: "scratch" })).toBeInTheDocument();
    await expect(picker).toHaveValue(SCRATCH.root);
  },
};
