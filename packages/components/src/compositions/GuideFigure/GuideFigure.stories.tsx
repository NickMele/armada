import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { GuideFigure } from "./GuideFigure";

/** The drawing takes the width it is given, so the frame is a reading measure. */
const meta: Meta<typeof GuideFigure> = {
  title: "Compositions/Guide figure",
  component: GuideFigure,
  decorators: [
    (Story) => (
      <div
        style={{
          width: "var(--w-dialog-wide)",
          padding: "var(--space-6)",
          background: "var(--surface-canvas)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof GuideFigure>;

/**
 * **Three members landing onto one branch, in order** — the relation guide 2
 * teaches, and one of the three `docs/contracts/design-system.md` names as worth
 * drawing. Each member owns a segment of the branch, so a segment drawing is
 * that member landing. It runs once and never loops.
 */
export const MembersLanding: Story = {
  args: { figure: "members-landing", scale: "lead" },
};

/** The same drawing under one step, where the step's line is what is being read. */
export const MembersLandingInline: Story = {
  args: { figure: "members-landing", scale: "inline" },
};

/**
 * **The four landing rules, and the drawing that shows the weakness.** Guide 1
 * is a rule, not a relation: nothing relates to anything here, so nothing
 * animates and no picture carries a fact the sentence does not. It exists only
 * because the `figure` shape makes a guide lead with a drawing.
 */
export const CompletionRules: Story = {
  args: { figure: "completion-rules", scale: "lead" },
};

/**
 * **Held still, the drawing is the finished branch.** Under
 * `prefers-reduced-motion` nothing runs, and every reading the motion arrived at
 * is already there: three segments drawn, three marks numbered, three links
 * named. Nothing was carried by the movement.
 */
export const HeldStill: Story = {
  args: { figure: "members-landing", scale: "lead" },
  beforeEach: () => {
    const real = window.matchMedia;
    window.matchMedia = (query: string) => {
      if (!query.includes("prefers-reduced-motion")) return real.call(window, query);
      // A preference that never changes, so nothing is dispatched to a listener.
      const reduced = new EventTarget() as MediaQueryList;
      return Object.assign(reduced, { matches: true, media: query, onchange: null });
    };
    return () => {
      window.matchMedia = real;
    };
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const figure = canvas.getByRole("img", { name: /landing onto it in order/ });
    await expect(figure.getAnimations({ subtree: true })).toHaveLength(0);
    // The order and the links read from the drawing itself, not from the label.
    for (const word of ["1", "2", "3", "Stacked", "Parked", "Waiting on a release", "main"]) {
      await expect(within(figure).getByText(word)).toBeVisible();
    }
  },
};
