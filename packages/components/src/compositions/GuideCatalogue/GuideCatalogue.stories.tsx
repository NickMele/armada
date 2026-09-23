import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { GUIDES } from "../../guides";
import { GuideCatalogue } from "./GuideCatalogue";

const meta: Meta<typeof GuideCatalogue> = {
  title: "Compositions/Guide catalogue",
  component: GuideCatalogue,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof GuideCatalogue>;

/**
 * **Every guide is in here.** The claim the catalogue exists to make: a person
 * who met a word once finds it again without the screen that raised it, so a
 * guide missing from this page is a guide nobody can go back to.
 */
export const EveryGuide: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    for (const guide of GUIDES) {
      await expect(canvas.getByRole("heading", { name: guide.title })).toBeVisible();
      // The number is what a person cites, so it is drawn rather than implied
      // by position — a filtered or reordered page would otherwise renumber.
      await expect(canvas.getByText(String(guide.number))).toBeVisible();
    }
    await expect(canvas.getAllByRole("listitem")).toHaveLength(GUIDES.length);
  },
};

/** One group, to see a section on its own. The page is the same shape at any length. */
export const OneGroup: Story = {
  args: { guides: GUIDES.filter((guide) => guide.group === "machine") },
};
