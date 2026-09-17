import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { OverviewSummaryStrip } from "./OverviewSummaryStrip";

/**
 * Overview's summary strip: a count first, then the panel it opens. #1091.
 * The count is the design system's one `--text-2xl` figure, set at body
 * weight — its size is the emphasis.
 */
const meta = {
  title: "Compositions/Overview summary strip",
  component: OverviewSummaryStrip,
  parameters: { layout: "padded" },
} satisfies Meta<typeof OverviewSummaryStrip>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Every count past zero, and Needs you amber for it — the everyday case. */
export const SomeOfEach: Story = {
  name: "Some of each",
  args: {
    items: [
      { id: "needs-you", label: "Needs you", count: 3, tone: "awaiting-review", onPress: fn() },
      { id: "running", label: "Running", count: 2, onPress: fn() },
      { id: "queued", label: "Queued", count: 1, onPress: fn() },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    const needsYou = canvas.getByRole("button", { name: "3 Needs you" });
    await expect(needsYou).toBeVisible();
    await userEvent.click(needsYou);
    await expect(args.items[0]!.onPress).toHaveBeenCalledOnce();
  },
};

/** Nothing waiting on anyone: every count reads subtle, none of them amber. */
export const AllEmpty: Story = {
  name: "All empty",
  args: {
    items: [
      { id: "needs-you", label: "Needs you", count: 0, tone: "awaiting-review", onPress: fn() },
      { id: "running", label: "Running", count: 0, onPress: fn() },
      { id: "queued", label: "Queued", count: 0, onPress: fn() },
    ],
  },
};

/** Overview 28 (#1092) adds Recently ended as a fourth item, red past zero. */
export const FourItems: Story = {
  name: "Four items, Recently ended red",
  args: {
    items: [
      { id: "needs-you", label: "Needs you", count: 1, tone: "awaiting-review", onPress: fn() },
      { id: "running", label: "Running", count: 2, onPress: fn() },
      { id: "queued", label: "Queued", count: 0, onPress: fn() },
      {
        id: "recently-ended",
        label: "Recently ended",
        count: 3,
        tone: "completed-failed",
        onPress: fn(),
      },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    const endedItem = canvas.getByRole("button", { name: "3 Recently ended" });
    await expect(endedItem).toBeVisible();
    await userEvent.click(endedItem);
    await expect(args.items[3]!.onPress).toHaveBeenCalledOnce();
  },
};

/** Narrower than the mock's own two-up floor: the strip wraps to 2×2. */
export const Narrow: Story = {
  name: "Narrow, two by two",
  args: SomeOfEach.args,
  decorators: [
    (Story) => (
      <div style={{ width: "var(--touch-floor)", containerType: "inline-size" }}>
        <Story />
      </div>
    ),
  ],
};
