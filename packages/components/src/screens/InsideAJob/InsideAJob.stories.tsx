import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { InsideAJob } from "./InsideAJob";
import { HEADING, OnAScreen } from "./fixtures";

/**
 * The arrangement job detail is drawn in, and only what it decides itself.
 *
 * **Every state a Job can be in is `Screens/Job detail`**, drawn by the app's
 * own `JobDetail` from wire data — that is where to look at a Job. This entry
 * kept fifteen states of its own, built from props typed by hand, and they could
 * not show a bug in the derivation the app runs between the wire and these
 * props. The unclamped Drone instructions shipped through exactly that gap.
 *
 * What stays is the one rule this component holds on its own: a region with
 * nothing to draw says why, rather than leaving a hole in the screen.
 */
const meta: Meta<typeof InsideAJob> = {
  title: "Screens/Inside a job",
  component: InsideAJob,
  decorators: [(Story) => <OnAScreen><Story /></OnAScreen>],
};
export default meta;

type Story = StoryObj<typeof InsideAJob>;

/** Nothing served to draw. Each region names what it could not read. */
export const NothingToDraw: Story = {
  args: { heading: HEADING, run: [] },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/Nothing serves this Job's workflow/)).toBeVisible();
  },
};
