import type { Meta, StoryObj } from "@storybook/react-vite";
import { GitBranch, GitCommitHorizontal, GitPullRequest, FileCheck } from "lucide-react";
import { expect } from "storybook/test";
import { JobOutcome } from "./JobOutcome";

/**
 * What a finished Job produced — the region a finished Job is opened for.
 *
 * Four of the five parts are not on the wire. Each keeps its row and names the
 * operation that would have to serve it, because a region that closes up around
 * the one served value draws a finished outcome that is a fifth of one.
 */
const meta: Meta<typeof JobOutcome> = {
  title: "Compositions/Job outcome",
  component: JobOutcome,
};
export default meta;

type Story = StoryObj<typeof JobOutcome>;

const NOTE = "Armada does not merge. The branch is pushed and the review is yours to take.";

/** Today: a branch, and four parts nothing serves. */
export const WhatIsServedToday: Story = {
  args: {
    note: NOTE,
    parts: [
      {
        name: "Branch",
        icon: GitBranch,
        iconLabel: "Branch",
        value: "armada/01M130Y1380016YK5S0JXBXDQ5",
      },
      {
        name: "Commit",
        icon: GitCommitHorizontal,
        iconLabel: "Commit",
        value: "5375d705cb7713a21a91681c1028166b98a0d6de",
        meta: "origin/armada/01M1CNPKTV0018H2M1CXDNBK06",
      },
      {
        name: "Pull request",
        icon: GitPullRequest,
        iconLabel: "Pull request",
        value: "https://example.invalid/armada/pull/229",
      },
      {
        /* No glyph. `file` is reserved to the log row and `file-check` to a
           submission that landed, so a changed-file row has nothing in the
           registry to take. The mark column stays and renders empty. */
        name: "Files changed",
        absent:
          "job.files_changed is published while a drone is working. Nothing serves a finished job's footprint.",
      },
      {
        name: "Evidence",
        icon: FileCheck,
        iconLabel: "Evidence",
        absent: "No operation serves a work submission, so there is nothing to draw.",
      },
    ],
  },
  /**
   * **The claim is the count.** Four of the five parts are unserved, and the
   * defect this story exists to prevent is a region that quietly closes up
   * around the one value it has — drawing a finished outcome that is a fifth of
   * one. So the first assertion is that five rows are on screen, and the second
   * is that the unserved row names the operation that would fill it rather than
   * rendering blank.
   *
   * Read by position, because the order is the contract: parts are drawn in the
   * order a reader asks for them.
   */
  play: async ({ canvas }) => {
    const parts = canvas.getAllByRole("listitem");
    await expect(parts).toHaveLength(5);
    await expect(parts[0]).toHaveTextContent("armada/01M130Y1380016YK5S0JXBXDQ5");
    await expect(parts[3]).toHaveTextContent("Files changed");
    await expect(parts[3]).toHaveTextContent("Nothing serves a finished job's footprint.");
    await expect(canvas.getByText(/Armada does not merge/)).toBeVisible();
  },
};

/** Every part served — what the region becomes as the four operations land. */
export const EveryPartServed: Story = {
  args: {
    note: NOTE,
    parts: [
      {
        name: "Branch",
        icon: GitBranch,
        iconLabel: "Branch",
        value: "armada/01M130Y1380016YK5S0JXBXDQ5",
        meta: "from main",
      },
      {
        name: "Commit",
        icon: GitCommitHorizontal,
        iconLabel: "Commit",
        value: "9f2c1ab",
        meta: "1 commit",
      },
      {
        name: "Pull request",
        icon: GitPullRequest,
        iconLabel: "Pull request",
        value: "armada#42",
      },
      { name: "Files changed", value: "4 files", meta: "+214 −96" },
      { name: "Evidence", icon: FileCheck, iconLabel: "Evidence", value: "3 submissions" },
    ],
  },
};

/**
 * A Job that finished with no worktree. The branch row says why rather than
 * disappearing — an absent branch is a fact about the Job, not a gap in Bridge.
 */
export const NoBranch: Story = {
  args: {
    parts: [
      {
        name: "Branch",
        icon: GitBranch,
        iconLabel: "Branch",
        absent: "This job has no worktree, so it has no branch.",
      },
    ],
  },
};
