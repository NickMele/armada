import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { Folder, GitBranch } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { JobLogReference } from "./JobLogReference";

/**
 * The log reference on a running job, on a failed one, and on a finished one —
 * the three states the log-envelope concept names, because Bridge names a
 * job's log on every job from dispatch.
 */
const meta: Meta<typeof JobLogReference> = {
  title: "Compositions/Job log reference",
  component: JobLogReference,
};
export default meta;

type Story = StoryObj<typeof JobLogReference>;

/**
 * The log row wants the plain page outline. **`file` has no entry in
 * `packages/icons/icons.toml`**, so the row renders a channel short rather
 * than reaching for an unregistered glyph. The concept names the treatment —
 * the `file-*` family is reserved to evidence, so the log takes the plain page
 * outline rather than a new glyph — and the registry has no row for it.
 * Reported.
 */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

/**
 * A running job. The log is being written now, and the counts are the reason
 * to look: a person reaches the file without knowing the sink layout, and it
 * costs one row and one button rather than a viewer.
 */
export const OnARunningJob: Story = {
  args: {
    rows: [
      {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Log",
        value: ".armada/logs/job_2d90bb.jsonl",
        copyValue: ".armada/logs/job_2d90bb.jsonl",
        meta: "142 lines · 0 error",
      },
    ],
    children: "Fleet, the drone and Bridge in one order, keyed on this job. It is being written now.",
    actions: <Button ground="sunken">Open the log</Button>,
  },
};

/**
 * A failed job, where the same block states where the work is: the branch, the
 * worktree, then the log. The worktree and the branch are left in place, and
 * that is written out rather than inferred from an absence of buttons.
 *
 * The worktree row borrows `folder`, whose registry entry means "workspace". A
 * worktree is not a workspace. Reported.
 */
export const OnAFailedJob: Story = {
  args: {
    rows: [
      {
        icon: GitBranch,
        iconLabel: "Branch",
        value: "feat/manifest-cache",
        copyValue: "feat/manifest-cache",
        meta: "2 files +48 −11",
      },
      { icon: Folder, iconLabel: "Worktree", value: "~/.armada/worktrees/job_91ab" },
      {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Log",
        value: ".armada/logs/job_91ab.jsonl",
        copyValue: ".armada/logs/job_91ab.jsonl",
        meta: "318 lines · 4 error",
        separated: true,
      },
    ],
    children:
      "The worktree and the branch are left in place. Armada will not touch either. The log holds Fleet, the drone and Bridge in one order, keyed on this job.",
    actions: (
      <>
        <Button ground="sunken">Open the log</Button>
        <Button ground="sunken">Open the worktree</Button>
      </>
    ),
  },
};

/**
 * A finished job. Nothing changes about the log because the job ended well —
 * it is named from dispatch, so a person does not learn where it is only when
 * something breaks.
 */
export const OnAFinishedJob: Story = {
  args: {
    rows: [
      {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Log",
        value: ".armada/logs/job_4f10.jsonl",
        copyValue: ".armada/logs/job_4f10.jsonl",
        meta: "204 lines · 0 error",
      },
    ],
    actions: <Button ground="sunken">Open the log</Button>,
  },
};

/**
 * A non-zero error count stays neutral. Nothing in
 * `packages/tokens/src/status.css` declares a hue for it, and anything the
 * file does not declare stays neutral — the count carries position and mono
 * weight instead.
 */
export const WithErrors: Story = {
  args: {
    rows: [
      {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Log",
        value: ".armada/logs/job_91ab.jsonl",
        copyValue: ".armada/logs/job_91ab.jsonl",
        meta: "318 lines · 4 error",
      },
    ],
    children: "Whether the error count is computed per view or carried on the job record is open.",
    actions: <Button ground="sunken">Open the log</Button>,
  },
};
