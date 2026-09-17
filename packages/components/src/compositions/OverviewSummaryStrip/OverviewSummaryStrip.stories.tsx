import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { OverviewSummaryStrip } from "./OverviewSummaryStrip";

/**
 * Overview's summary tiles: four glass cards, two by two, each wearing its
 * status hue on its dot, label and corner wash. #1091, #1261. The count is the
 * design system's one `--text-2xl` figure, set at body weight — its size is
 * the emphasis.
 */
const meta = {
  title: "Compositions/Overview summary strip",
  component: OverviewSummaryStrip,
  parameters: { layout: "padded" },
} satisfies Meta<typeof OverviewSummaryStrip>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Counts past zero take the caller's tone: Needs you amber, Recently ended red. */
export const SomeOfEach: Story = {
  name: "Some of each",
  args: {
    items: [
      { id: "needs-you", label: "Needs you", count: 3, hue: "awaiting-review", tone: "awaiting-review", onPress: fn() },
      { id: "running", label: "Running", count: 2, hue: "running", onPress: fn() },
      { id: "queued", label: "Queued", count: 0, hue: "not-started", onPress: fn() },
      {
        id: "recently-ended",
        label: "Recently ended",
        count: 4,
        hue: "completed-success",
        tone: "completed-failed",
        onPress: fn(),
      },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    const needsYou = canvas.getByRole("button", { name: "3 Needs you" });
    await expect(needsYou).toBeVisible();
    await userEvent.click(needsYou);
    await expect(args.items[0]!.onPress).toHaveBeenCalledOnce();

    const ended = canvas.getByRole("button", { name: "4 Recently ended" });
    await userEvent.click(ended);
    await expect(args.items[3]!.onPress).toHaveBeenCalledOnce();
  },
};

/** An empty Overview: every tile keeps its hue and wash, and every "0" is neutral. */
export const AllEmpty: Story = {
  name: "All empty",
  args: {
    items: [
      { id: "needs-you", label: "Needs you", count: 0, hue: "awaiting-review", tone: "awaiting-review", onPress: fn() },
      { id: "running", label: "Running", count: 0, hue: "running", onPress: fn() },
      { id: "queued", label: "Queued", count: 0, hue: "not-started", onPress: fn() },
      {
        id: "recently-ended",
        label: "Recently ended",
        count: 0,
        hue: "completed-success",
        tone: "completed-failed",
        onPress: fn(),
      },
    ],
  },
};
