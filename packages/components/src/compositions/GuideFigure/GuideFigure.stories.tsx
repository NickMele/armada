import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { GuideFigure } from "./GuideFigure";

/** The drawing takes the width it is given, so the frame is a reading measure. */
const meta: Meta<typeof GuideFigure> = {
  title: "Compositions/Guide figure",
  component: GuideFigure,
  args: { scale: "panel" },
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
 * **A job running its workflow's steps, in order** — the real
 * `WorkflowStepCard`, the same card the canvas and the stacked run both draw,
 * with the registry's own words on it. Guides 15 and 19 share it.
 */
export const WorkflowSteps: Story = {
  args: { figure: "workflow-steps" },
};

/**
 * **The groups of a plan, one at a time.** Real group and task cards: the first
 * group is through its checks before the second starts, which is what guides 4
 * and 17 both name.
 */
export const GroupOrder: Story = {
  args: { figure: "group-order" },
};

/**
 * **Two real `StepBar`s, filling.** The wrapper opens from the start of each
 * bar to its end; the bar's own segments are untouched, so what fills is the
 * component the run draws.
 */
export const StepBars: Story = {
  args: { figure: "step-bar" },
};

/**
 * **The real member list**, rail and all, each card arriving in its turn. This
 * is the figure that binds hardest: it is `JobMembers` itself, so a change to
 * that list changes this guide.
 */
export const MembersLanding: Story = {
  args: { figure: "members-landing" },
};

/** The same drawing in the layer a `?` opens, which is the narrower of the two. */
export const InTheCard: Story = {
  args: { figure: "members-landing", scale: "card" },
};

/**
 * **Held still, the drawing is its own finished state.** Under
 * `prefers-reduced-motion` nothing runs, and every reading the motion arrived
 * at is already there: the order on the rail, the link each member carries.
 */
export const HeldStill: Story = {
  args: { figure: "members-landing" },
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
    const figure = canvas.getByRole("img", { name: /landing in order/ });
    await expect(figure.getAnimations({ subtree: true })).toHaveLength(0);
    // The order and the links read from the drawing itself, not from the label.
    for (const word of ["1", "2", "3", "Stacked on the one before it."]) {
      await expect(within(figure).getByText(word)).toBeVisible();
    }
  },
};
