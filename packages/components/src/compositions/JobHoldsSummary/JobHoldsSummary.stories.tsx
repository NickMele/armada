import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { JobHoldsSummary, type HoldsLine } from "./JobHoldsSummary";

const meta: Meta<typeof JobHoldsSummary> = {
  title: "Compositions/Job holds (summary)",
  component: JobHoldsSummary,
  args: { onOpen: fn() },
  // The column it lives in is `--w-run-column`. Drawn at that width, because
  // what this block claims is that it fits under a run without pushing the
  // pointers below it off the screen.
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-run-column)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof JobHoldsSummary>;

/** The last two lines Armada wrote about this Job, newest first. */
const TAIL: HoldsLine[] = [
  { at: "14:31:58", actor: "Drone", said: "thinking" },
  { at: "14:30:28", actor: "Fleet", said: "Heartbeat — quiet 48s" },
];

/**
 * **A Job holding a live worktree.** Two processes, a checkout on disk and the
 * size it takes — the four figures a person came to this column for, in five
 * lines instead of a card.
 */
export const HoldingALiveWorktree: Story = {
  args: {
    tail: TAIL,
    figures: { processes: 2, worktree: "healthy", size: "1.2 GiB" },
    age: "3s",
  },
};

/**
 * **The read has not answered.** Not the same thing as a Job holding nothing,
 * and the whole reason `figures` is nullable — a block that drew `None` here
 * would report a silent seam as a settled reading.
 *
 * The tail still draws: the log socket and the machine read are two answers,
 * and one being silent says nothing about the other.
 */
export const TheReadHasNotAnswered: Story = {
  args: {
    tail: TAIL,
    figures: null,
    note: "Fleet did not answer, so what this Job holds is unknown.",
    age: "3s",
  },
};

/**
 * **A Job that holds nothing, and is right to.** A Job at its approval gate has
 * no process and no checkout, which is a real answer and reads as one.
 *
 * **No size row.** There is nothing on disk to size, and the worktree line above
 * already says so — a `0 B` beside it would be the same absence drawn twice as
 * a figure.
 */
export const HoldingNothing: Story = {
  args: {
    tail: [
      { at: "09:22:04", actor: "Fleet", said: "Worktree reclaimed" },
      { at: "09:21:58", actor: "Fleet", said: "Job finished" },
    ],
    figures: { processes: 0, worktree: "none on disk" },
    age: "11s",
  },
  /**
   * **The absent size must not come back as an empty row.** A figure with no
   * value renders as a label and a gap, which is indistinguishable from a
   * figure that failed to arrive — and nothing about the rendering says which
   * of the two a blank cell is.
   */
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Size on disk")).toBeNull();
    await expect(canvas.getByText("none on disk")).toBeVisible();
  },
};

/**
 * **The worktree is in trouble.** The Job reads running, Fleet holds no process
 * for it and the checkout it recorded is gone — the state the full reading was
 * built for, said here in two words and a hue.
 *
 * **Whether an absence is loud is the caller's answer, not this block's.** The
 * sheet decides it from the examination; a second judgment made here would
 * disagree with the reading it opens.
 */
export const TheWorktreeIsInTrouble: Story = {
  args: {
    tail: [
      { at: "09:16:47", actor: "Fleet", said: "A preparation command failed", wrong: true },
      { at: "09:14:02", actor: "Fleet", said: "Worktree cut" },
    ],
    figures: {
      processes: 0,
      nothingRunningIsWrong: true,
      worktree: "gone",
      worktreeIsWrong: true,
    },
    age: "4s",
  },
};
