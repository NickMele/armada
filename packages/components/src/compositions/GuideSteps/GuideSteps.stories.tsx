import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { GUIDE_COMPLETION, GUIDE_MEMBER_LINK } from "../../guides";
import { GuideSteps } from "./GuideSteps";

/**
 * A guide, read. The frame is the catalogue panel's measure by default;
 * `InTheCard` narrows it to the layer a `?` opens.
 */
const meta: Meta<typeof GuideSteps> = {
  title: "Compositions/Guide steps",
  component: GuideSteps,
  args: { where: "panel" },
  decorators: [
    (Story) => (
      <div
        style={{
          width: "var(--w-dialog-wide)",
          padding: "var(--pad-card)",
          background: "var(--surface-card)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof GuideSteps>;

/**
 * **A relation, with the drawing under the line that names it.** Guide 2: a
 * numbered sequence, one line each, and the real member list under step 2 —
 * because the order is the relation.
 */
export const WithAFigure: Story = {
  args: { guide: GUIDE_MEMBER_LINK },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: /landing in order/ })).toBeVisible();
  },
};

/**
 * **A rule, with no figure and no frame where one would go.** Guide 1 has no
 * relation in it, so the lines stand alone.
 */
export const WithNoFigure: Story = {
  args: { guide: GUIDE_COMPLETION },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole("img")).toBeNull();
  },
};

/** The same guide in the layer a `?` opens, where the drawing has less width. */
export const InTheCard: Story = {
  args: { guide: GUIDE_MEMBER_LINK, where: "card" },
  decorators: [
    (Story) => (
      <div
        style={{
          width: "var(--w-dialog-wide)",
          maxWidth: "var(--w-dialog)",
          padding: "var(--pad-card)",
          background: "var(--bg-overlay)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
