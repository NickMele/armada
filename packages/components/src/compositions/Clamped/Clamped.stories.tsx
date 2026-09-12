import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { Clamped } from "./Clamped";
import { DroneBrief, type BriefLine } from "../DroneBrief/DroneBrief";

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

/**
 * A step's opening turn, in the blocks `crates/fleet/src/briefing.rs` writes —
 * headings marked as the wire marks them, by line number rather than by
 * capitals. Long enough to pass twelve lines in the panel's width, because a
 * fixture that fitted would assert nothing.
 */
const INSTRUCTION: BriefLine[] = [
  { text: "JOB BRIEF", named: "heading" },
  { text: "" },
  { text: "Coalesce concurrent token refreshes" },
  { text: "" },
  { text: "Two requests arriving inside the refresh window each start their own refresh," },
  { text: "and the second overwrites the first's token." },
  { text: "" },
  { text: "WHERE YOU ARE", named: "heading" },
  { text: "" },
  { text: "This task runs in 4 parts. You are on part 1." },
  { text: "" },
  { text: "  1. Plan the change — you are here" },
  { text: "     STOP. Submit when this part is done, then wait." },
  { text: "  2. Implement — not yours — do not do it" },
  { text: "  3. Verify — not yours — do not do it" },
  { text: "  4. Hand off — not yours — do not do it" },
  { text: "" },
  { text: "WHAT THIS PART DELIVERS", named: "heading" },
  { text: "" },
  { text: "Write this part's finding to a file, at this exact path in your worktree:" },
  { text: "" },
  { text: "  .armada/artifacts/coalesce-concurrent-token-refreshes-root-cause-and-plan.md" },
  { text: "" },
  { text: "This is the work product, not a note to yourself. This exact path is the one" },
  { text: "that is read: an empty file or no file stops this part." },
];

/**
 * The turn Armada opened a step with, held to the panel's twelve lines — a
 * `DroneBrief`, not a string.
 *
 * Every story above passes a string, which is why the panel shipped with
 * no clamp at all: `-webkit-line-clamp` counts line boxes, and `DroneBrief`
 * was a flex column — one box carrying none — so the clamp did nothing and
 * a step's whole opening turn, a screen and a half of it, sat above the
 * Checks and Verdicts. Only the component the app actually wraps catches
 * that; a string cannot.
 *
 * The assertion checks the clipped height, not just the control, since the
 * control is downstream and reading it alone would leave the mechanism
 * untested.
 */
export const ClampsTheComponentTheAppWraps: Story = {
  args: {
    lines: 12,
    moreLabel: "Read the whole instruction",
    children: <DroneBrief lines={INSTRUCTION} />,
  },
  play: async ({ canvas, canvasElement }) => {
    const body = canvasElement.querySelector(".armada-clamped__body");
    if (body === null) throw new Error("no clamped body to measure");
    // The two were equal while the brief was a flex column. That equality is
    // the whole defect: nothing clipped, so nothing to open.
    await expect(body.scrollHeight).toBeGreaterThan(body.clientHeight);
    await expect(canvas.getByRole("button", { name: "Read the whole instruction" })).toBeVisible();
  },
};

