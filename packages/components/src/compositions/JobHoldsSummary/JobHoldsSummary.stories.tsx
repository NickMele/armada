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
 * **A Job holding a live worktree.** Two processes and a checkout that takes
 * 1.2 GiB — the figures a person came to this column for, in a handful of
 * lines instead of a card.
 *
 * **The worktree row is the size.** It said `on disk` over a `Size on disk`
 * row until #1484, and `on disk` was the same word on every Job ever opened:
 * nothing looks at a worktree unless somebody presses `Look now`.
 */
export const HoldingALiveWorktree: Story = {
  args: {
    latest: LATEST,
    spend: "at least ~$1.96",
    turns: "82 of 300",
    figures: { processes: 2, worktree: "1.2 GiB" },
    age: "3s",
  },
  play: async ({ canvas, canvasElement }) => {
    await everyRowSharesItsEdges(canvasElement, LATEST);
    // One worktree row, carrying the figure the second row used to hold.
    await expect(canvas.queryByText("Size on disk")).toBeNull();
    await expect(canvas.getByText("1.2 GiB")).toBeVisible();
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
    spend: "~$0.84",
    turns: "31 of 300",
    figures: null,
    note: "Fleet did not answer, so what this Job holds is unknown.",
    age: "3s",
  },
  /**
   * **What the Job is spending does not wait on the look.** Spend and Turns
   * come off the Job itself; a silent machine read is a fact about the machine
   * and says nothing about either figure.
   */
  play: async ({ canvas }) => {
    await expect(canvas.getByText("~$0.84")).toBeVisible();
    await expect(canvas.getByText("31 of 300")).toBeVisible();
    await expect(canvas.queryByText("Processes")).toBeNull();
  },
};

/**
 * **A Fleet that does not price, on a Job that has taken turns.** Spend draws
 * no row rather than `~$0.00` — a Job that cost nothing and a Fleet with no
 * figure are two different facts, and a zero says the first about the second.
 */
export const NothingIsPriced: Story = {
  args: {
    latest: LATEST,
    turns: "7 of 300",
    figures: { processes: 1, worktree: "640 MiB" },
    age: "5s",
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Spend")).toBeNull();
    await expect(canvas.getByText("7 of 300")).toBeVisible();
  },
};

/**
 * **A Job that holds nothing, and is right to.** A Job at its approval gate has
 * no process and no checkout, which is a real answer and reads as one.
 *
 * **Words rather than a figure, in the slot the figure uses.** There is nothing
 * on disk to size, and `0 B` would claim a directory was walked.
 */
export const HoldingNothing: Story = {
  args: {
    latest: { at: "09:22:04", actor: "Fleet", said: "Worktree reclaimed" },
    spend: "~$3.10",
    turns: "144 of 300",
    figures: { processes: 0, worktree: "none on disk" },
    age: "11s",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("none on disk")).toBeVisible();
    await expect(canvas.queryByText("0 B")).toBeNull();
  },
};

/**
 * **The walk ran past its bound.** A size that did not finish is worth knowing
 * and it is not zero, so the row says so in the slot the figure would take.
 */
export const TheWalkDidNotFinish: Story = {
  args: {
    latest: LATEST,
    spend: "~$2.40",
    turns: "96 of 300",
    figures: { processes: 1, worktree: "not measured" },
    age: "8s",
  },
};

/**
 * **The worktree is in trouble.** The Job reads running, Fleet holds no process
 * for it and the checkout it recorded is gone — the state the full reading was
 * built for, said here in one word and a hue.
 *
 * **The fault takes the size's slot rather than sitting beside it.** A size for
 * a directory that is not there is a figure about nothing.
 *
 * **Whether an absence is loud is the caller's answer, not this block's.** The
 * sheet decides it from the examination; a second judgment made here would
 * disagree with the reading it opens.
 */
export const TheWorktreeIsInTrouble: Story = {
  args: {
    latest: { at: "09:16:47", actor: "Fleet", said: "A preparation command failed", wrong: true },
    spend: "at least ~$5.28",
    turns: "300 of 300",
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
    spend: "~$0.12",
    turns: "3 of 300",
    figures: { processes: 1, worktree: "1.2 GiB" },
    age: "2s",
  },
};
