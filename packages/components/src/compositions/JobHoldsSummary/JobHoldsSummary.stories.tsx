import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { JobHoldsSummary, type HoldsLine } from "./JobHoldsSummary";

const meta: Meta<typeof JobHoldsSummary> = {
  title: "Compositions/Job holds (summary)",
  component: JobHoldsSummary,
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
const LATEST: HoldsLine = {
  at: "14:31:58",
  actor: "Drone",
  said: "Edit packages/settings/src/selectors.ts",
};

/** What a range says the text was actually drawn on, not the box holding it. */
function drawn(element: Element): DOMRect {
  const text = document.createRange();
  text.selectNodeContents(element);
  return text.getBoundingClientRect();
}

/**
 * **Every row shares two edges: labels on the left, values on the right.** The
 * top row is the one this measures — its value is words rather than a figure
 * and it carries a time under it, and both sat hard left for a day while the
 * three rows beneath them had gone to the edge.
 *
 * Alignment is geometry, so this measures rather than reading roles: a row
 * drawn out of line reads identically through `getByRole`. Text is measured
 * with a range, because every value's box runs to the edge whichever way its
 * text is aligned.
 */
async function everyRowSharesItsEdges(canvasElement: HTMLElement, line: HoldsLine) {
  const canvas = within(canvasElement);
  const labels = canvas.getAllByRole("term");
  const values = canvas.getAllByRole("definition");
  const list = values[0]?.closest("dl");
  await expect(list).toBeTruthy();
  const edge = list!.getBoundingClientRect().right;
  // A figure row's label, since the top row is the one on trial.
  const start = drawn(labels.at(-1)!).left;
  for (const [index, value] of values.entries()) {
    await expect(Math.abs(drawn(value).right - edge)).toBeLessThanOrEqual(1);
    await expect(Math.abs(drawn(labels[index]!).left - start)).toBeLessThanOrEqual(1);
  }
  // The top row by the words it drew rather than the markup it drew them in —
  // what it said, the time under it, and who said it on the labels' own edge.
  for (const text of [line.said, line.at]) {
    await expect(Math.abs(drawn(canvas.getByText(text)).right - edge)).toBeLessThanOrEqual(1);
  }
  await expect(Math.abs(drawn(canvas.getByText(line.actor)).left - start)).toBeLessThanOrEqual(1);
}

/**
 * **A Job holding a live worktree.** Two processes, a checkout on disk and the
 * size it takes — the four figures a person came to this column for, in five
 * lines instead of a card.
 */
export const HoldingALiveWorktree: Story = {
  args: {
    latest: LATEST,
    figures: { processes: 2, worktree: "healthy", size: "1.2 GiB" },
    age: "3s",
  },
  play: async ({ canvasElement }) => {
    await everyRowSharesItsEdges(canvasElement, LATEST);
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
    latest: LATEST,
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
    latest: { at: "09:22:04", actor: "Fleet", said: "Worktree reclaimed" },
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
    latest: { at: "09:16:47", actor: "Fleet", said: "A preparation command failed", wrong: true },
    figures: {
      processes: 0,
      nothingRunningIsWrong: true,
      worktree: "gone",
      worktreeIsWrong: true,
    },
    age: "4s",
  },
};

/** A person acted last, so the latest event is theirs. */
export const APersonActedLast: Story = {
  args: {
    latest: { at: "14:40:12", actor: "You", said: "Approved dispatch" },
    figures: { processes: 1, worktree: "healthy", size: "1.2 GiB" },
    age: "2s",
  },
};
