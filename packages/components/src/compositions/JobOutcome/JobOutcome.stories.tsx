import type { Meta, StoryObj } from "@storybook/react-vite";
import { GitBranch, GitCommitHorizontal, GitPullRequest, FileCheck } from "lucide-react";
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

const NOTE = "Armada does not push and does not merge. The branch is yours to take.";

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
        absent: "Fleet does not commit at the last step yet, so there is nothing to name.",
      },
      {
        name: "Pull request",
        icon: GitPullRequest,
        iconLabel: "Pull request",
        absent: "Fleet does not open one yet, so there is nothing to open.",
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
