import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { Clamped } from "./Clamped";

/**
 * One story per state: a passage long enough to clamp, one short enough not to,
 * and one opened on mount.
 *
 * **The second story is the one that matters.** A *View more* under text that
 * was never truncated is a control that does nothing, and a reader who presses
 * it once stops trusting the surface.
 */
const meta: Meta<typeof Clamped> = {
  title: "Compositions/Clamped",
  component: Clamped,
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-dialog)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof Clamped>;

const LONG =
  "The selectors cannot be tested without constructing the whole store, which " +
  "makes every settings test an integration test. Move the selector block into " +
  "its own module so the tests can import it without constructing the store, and " +
  "do not change reducer behaviour. The columns selector is the one that matters: " +
  "it is memoised against the whole settings slice today, so any write to any " +
  "setting invalidates it, and the board re-sorts on a change to something it " +
  "does not read.";

const SHORT = "Run the regression suite and fix anything it turns up.";

/**
 * A brief long enough to push the run off the screen. Three lines, and the rest
 * one press away.
 */
export const LongEnoughToClamp: Story = {
  args: { children: LONG },
};

/**
 * Short enough that nothing is hidden. **No control**, because there is nothing
 * behind it.
 */
export const NothingToOpen: Story = {
  args: { children: SHORT },
};

/**
 * Opened on mount, for a caller that knows the passage is why the reader is
 * here — a redirect just written, a refusal's grounds.
 */
export const OpenOnMount: Story = {
  args: { children: LONG, defaultOpen: true },
};

/**
 * Two lines rather than three. The count is the caller's, because how much of a
 * passage is worth showing depends on what sits under it.
 */
export const HeldToTwoLines: Story = {
  args: { children: LONG, lines: 2 },
};

/**
 * The control appears only where the text overflows, and it says which way it
 * goes.
 *
 * **What earns the assertion is that the overflow is measured, not counted.**
 * The same words clamp at one width and do not at another, so a story that only
 * proved the long case would pass with a hard-coded character threshold that is
 * wrong on every panel but this one.
 */
export const TheControlExistsOnlyWhenThereIsMore: Story = {
  args: { children: LONG },
  play: async ({ canvas, userEvent }) => {
    const more = canvas.getByRole("button", { name: "View more" });
    await expect(more).toHaveAttribute("aria-expanded", "false");

    await userEvent.click(more);
    await expect(canvas.getByRole("button", { name: "View less" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  },
};

/**
 * And it is absent on a passage that fits.
 */
export const NoControlOnShortText: Story = {
  args: { children: SHORT },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button")).toBeNull();
  },
};
