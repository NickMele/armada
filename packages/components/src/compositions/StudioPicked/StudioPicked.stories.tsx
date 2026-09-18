import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { StudioPicked, type StudioPickedAct } from "./StudioPicked";

const meta: Meta<typeof StudioPicked> = {
  title: "Compositions/Studio picked",
  component: StudioPicked,
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-studio-node)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof StudioPicked>;

/** Everything a Contradiction offers, with #1406's Open and the delete beside them. */
const EVERY_ACT: StudioPickedAct[] = [
  { id: "open", label: "Open" },
  { id: "write_up", label: "Write up" },
  { id: "defer", label: "Defer" },
  { id: "settled", label: "Not a problem" },
  { id: "resolved", label: "Resolved here" },
  { id: "remove", label: "Delete node", danger: true },
];

/**
 * One node offering six acts. **The card is a line and a control at any number
 * of them** — a seventh is a row inside the menu, which resolves against the
 * window rather than against this 198px column.
 */
export const OneNode: Story = {
  args: {
    picked: ["Contradiction: The chip reads the Board and the row disagrees"],
    acts: EVERY_ACT,
    onAct: fn(),
  },
  play: async ({ canvas, args, userEvent, step }) => {
    await step("every act is one press away, the delete last", async () => {
      await userEvent.click(canvas.getByRole("button", { name: "Acts" }));
      const offered = canvas.getAllByRole("menuitem").map((item) => item.textContent);
      await expect(offered).toEqual(EVERY_ACT.map((act) => act.label));
    });

    await step("the act pressed is the act reported", async () => {
      await userEvent.click(canvas.getByRole("menuitem", { name: "Resolved here" }));
      await expect(args.onAct).toHaveBeenCalledWith("resolved");
    });
  },
};

/** A Studio picked over whole, to delete it or to read it as one Outline. */
export const ManyNodes: Story = {
  args: {
    picked: Array.from({ length: 40 }, (_, at) => `Note: What the ${at + 1}th note said`),
    acts: [
      { id: "outline", label: "Outline" },
      { id: "remove", label: "Delete node", danger: true },
    ],
    onAct: fn(),
  },
  play: async ({ canvas, step }) => {
    await step("how many are picked, and not which", async () => {
      await expect(canvas.getByText("40 nodes picked")).toBeVisible();
      await expect(canvas.queryByText(/What the 1th note said/)).toBeNull();
      await expect(canvas.queryByText(/What the 40th note said/)).toBeNull();
    });
  },
};

/** A Studio reopened read-only offers nothing to act with, and still says what is picked. */
export const NothingOffered: Story = {
  args: { picked: ["Note: The legend under the step bar is unreadable"], acts: [], onAct: fn() },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Acts" })).toBeNull();
  },
};
