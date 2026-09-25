import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { GUIDE_COMPLETION, GUIDE_MEMBER_LINK } from "../../guides";
import { GuideShaped } from "./GuideShaped";

/**
 * Two shapes of one guide, for a decision. The frame is the catalogue panel's
 * measure by default; `InTheCard` narrows it to the layer a `?` opens.
 */
const meta: Meta<typeof GuideShaped> = {
  title: "Compositions/Guide shaped",
  component: GuideShaped,
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

type Story = StoryObj<typeof GuideShaped>;

/**
 * **A relation, as steps.** Guide 2: a numbered sequence, one line each, and the
 * figure under the line that names the order — because the order is the relation.
 */
export const StepsWithAFigure: Story = {
  args: { guide: GUIDE_MEMBER_LINK, shape: "steps" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: /landing onto it in order/ })).toBeVisible();
  },
};

/**
 * **A rule, as steps, with no figure and no frame where one would go.** Guide 1
 * has no relation in it, so the lines stand alone — the claim being that a
 * missing drawing costs the shape nothing.
 */
export const StepsWithNoFigure: Story = {
  args: { guide: GUIDE_COMPLETION, shape: "steps" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole("img")).toBeNull();
  },
};

/** **A relation, figure first**, with the words as captions under the drawing. */
export const FigureLeading: Story = {
  args: { guide: GUIDE_MEMBER_LINK, shape: "figure" },
};

/**
 * **Figure first with nothing honest to draw.** Guide 1 leads with the four
 * rules in four boxes, because the shape requires a drawing. It is weaker than
 * the sentence under it, and that weakness is the cost being decided about.
 */
export const FigureLeadingWithNothingToDraw: Story = {
  args: { guide: GUIDE_COMPLETION, shape: "figure" },
};

/** Figure first in the layer a `?` opens, which is where the shape costs most. */
export const InTheCard: Story = {
  args: { guide: GUIDE_MEMBER_LINK, shape: "figure", where: "card" },
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
