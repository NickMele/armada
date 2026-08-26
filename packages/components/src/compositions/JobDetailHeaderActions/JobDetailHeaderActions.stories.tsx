import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, CircleDot, X } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { JobDetailHeaderActions } from "./JobDetailHeaderActions";

/**
 * The header on every state job detail reaches: working, stopped badly, stopped
 * well. One story each, because the three are only comparable side by side —
 * this block was hand-built three times before it was converged, and the copies
 * had drifted on wrapping, on shrink order and on whether mono was the default.
 *
 * The act shape is the same on both terminals: none. A destructive control
 * exists only while the job is still running.
 *
 * **The labels come from the enum→verb map**, `crates/core-model/domain/
 * enum-verbs.toml`, sentence-cased, and the glyphs from the same rows. They
 * are written here because that map is not generated into TypeScript yet, and
 * nowhere that ships.
 */
const meta: Meta<typeof JobDetailHeaderActions> = {
  title: "Compositions/Job detail header actions",
  component: JobDetailHeaderActions,
};
export default meta;

type Story = StoryObj<typeof JobDetailHeaderActions>;

/**
 * A running job. The badge is static: the workflow rail beneath it on this
 * screen carries the one pulse, on its current step.
 *
 * The branch copies on click. `Kill` is the only act, and it is outlined —
 * there is nothing to approve and nothing to merge while a job works, so no
 * primary exists on this screen.
 */
export const ARunningJob: Story = {
  args: {
    status: "running",
    statusIcon: CircleDot,
    statusLabel: "Running",
    headline: "Split the settings reducer",
    jobId: "job_2d90bb",
    fields: [
      { label: "Step", value: "2 of 4", mono: true },
      {
        label: "Branch",
        value: "fix/settings-split",
        mono: true,
        copyValue: "fix/settings-split",
      },
      { label: "Elapsed", value: "11m 03s", mono: true },
      { label: "Spend, estimated", value: "~$1.80", mono: true },
      { label: "Dispatched by you" },
    ],
    actions: <Button variant="destructive">Kill</Button>,
  },
};

/**
 * A failed job. `Stopped at Run tests, step 3 of 4` is one fact, not two — the
 * comma joins them, and the step name stays sans beside its mono sibling
 * because a step name is a label and not a machine-derived value.
 *
 * No action here either. What you can do with a dead end is read its log and
 * its worktree, and those sit beside the branch further down the screen.
 */
export const AFailedJob: Story = {
  args: {
    status: "completed-failed",
    statusIcon: X,
    statusLabel: "Failed",
    headline: "Cache the manifest read",
    jobId: "job_91ab",
    fields: [
      { label: "Stopped at", value: "Run tests" },
      { label: "step", value: "3 of 4", mono: true, continues: true },
      { label: "Ran", value: "22m 41s", mono: true },
      { label: "Spend, estimated", value: "~$2.10", mono: true },
      { label: "Dispatched by you" },
    ],
  },
};

/**
 * A finished job carries no action in the header at all: the acts on a
 * finished job are about its branch and its log, and they sit beside those
 * rather than up here. The field run changes with the state — a job that has
 * stopped reports what it ran, not what step it is on.
 */
export const AFinishedJob: Story = {
  args: {
    status: "completed-success",
    statusIcon: Check,
    statusLabel: "Done",
    headline: "Add a retry ceiling to the poke loop",
    jobId: "job_4f10",
    fields: [
      { label: "All", value: "4 of 4", mono: true, suffix: "steps advanced" },
      { label: "Ran", value: "18m 22s", mono: true },
      { label: "Spend, estimated", value: "~$2.40", mono: true },
      { label: "Dispatched by you" },
    ],
  },
};
