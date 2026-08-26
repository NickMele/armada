import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, CircleDot, Clock, GitBranch, Power, UserCheck, X } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { JobRowStacked } from "../JobRowStacked/JobRowStacked";
import { StepBar } from "../StepBar/StepBar";
import { ActiveJobsList, type ActiveJobsListProps } from "./ActiveJobsList";

/**
 * The list at the six Job states M1 renders, and the two empty cases.
 *
 * The one row that needs a person sorts first; the rest are newest work first.
 * The list renders that order rather than deciding it.
 */
const meta: Meta<typeof ActiveJobsList> = {
  title: "Compositions/Active jobs list",
  component: ActiveJobsList,
};
export default meta;

type Story = StoryObj<typeof ActiveJobsList>;

const menu = [
  { label: "Copy job id", shortcut: "⌘C" },
  { label: "Kill", shortcut: "x", danger: true },
];

/**
 * Six states, one row shape. The running row is focused, so it carries the one
 * pulse on the screen — fourteen breathing dots is what the motion rules
 * forbid outright.
 */
export const SixStates: Story = {
  args: {
    heading: "Active jobs",
    summary: "6 jobs. 1 awaiting approval.",
    action: <Button variant="primary">New job</Button>,
    children: [
      <JobRowStacked
        key="a"
        status="awaiting-approval"
        statusIcon={UserCheck}
        statusLabel="Needs approval"
        headline="Coalesce concurrent token refreshes"
        jobId="job_7c31"
        fields={[
          { label: "Workflow", value: "bug, 4 steps", quiet: true },
          { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
          { value: "Not started", quiet: true },
          { value: "created 09:12", quiet: true },
          { value: "Dispatched by you" },
        ]}
        action={<SplitButton ground="card" items={[{ label: "Reject", danger: true }]}>Approve</SplitButton>}
      />,
      <JobRowStacked
        key="b"
        status="not-started"
        statusIcon={Clock}
        statusLabel="Queued"
        headline="Retire the legacy poke path"
        jobId="job_8b42"
        fields={[
          { label: "Workflow", value: "bug, 4 steps", quiet: true },
          { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
          { value: "Waiting on a drone", emphasis: true },
          { value: "approved 09:20", quiet: true },
          { value: "Dispatched by you" },
        ]}
        action={<SplitButton ground="card" items={menu}>Open</SplitButton>}
      />,
      <JobRowStacked
        key="c"
        status="running"
        statusIcon={CircleDot}
        statusLabel="Running"
        headline="Split the settings reducer"
        jobId="job_2d90bb"
        focused
        pulsing
        fields={[
          { value: "fix/settings-split", mono: true, icon: GitBranch, copyValue: "fix/settings-split" },
          { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
          { value: "Implement", emphasis: true },
          { value: "11m 03s", mono: true },
          { value: "~$1.80", mono: true },
        ]}
        action={<SplitButton ground="card" items={menu}>Open</SplitButton>}
      />,
      <JobRowStacked
        key="d"
        status="completed-failed"
        statusIcon={X}
        statusLabel="Failed"
        headline="Cache the manifest read"
        jobId="job_91ab"
        fields={[
          { value: "feat/manifest-cache", mono: true, icon: GitBranch, copyValue: "feat/manifest-cache" },
          { value: <StepBar total={4} current={3} activity="failed" label="Step 3 of 4" /> },
          { value: "Run tests", emphasis: true },
          { value: "22m 41s", mono: true },
          { value: "~$2.10", mono: true },
        ]}
        action={<SplitButton ground="card" items={menu}>Open</SplitButton>}
      />,
      <JobRowStacked
        key="e"
        status="completed-success"
        statusIcon={Check}
        statusLabel="Done"
        headline="Add a retry ceiling to the poke loop"
        jobId="job_4f10"
        fields={[
          { value: "fix/poke-ceiling", mono: true, icon: GitBranch, copyValue: "fix/poke-ceiling" },
          { value: <StepBar total={4} current={5} activity="advanced" label="All 4 of 4 steps advanced" /> },
          { value: "Summarise" },
          { value: "18m 22s", mono: true },
          { value: "~$2.40", mono: true },
        ]}
        action={<SplitButton ground="card" items={menu}>Open</SplitButton>}
      />,
      <JobRowStacked
        key="f"
        status="killed"
        statusIcon={Power}
        statusLabel="Killed"
        headline="Rename the session token field"
        jobId="job_5e88"
        fields={[
          { value: "feat/session-rename", mono: true, icon: GitBranch, copyValue: "feat/session-rename" },
          { value: <StepBar total={4} current={2} activity="killed" label="Step 2 of 4" /> },
          { value: "Implement", emphasis: true },
          { value: "4m 09s", mono: true },
          { value: "~$0.60", mono: true },
        ]}
        action={<SplitButton ground="card" items={menu}>Open</SplitButton>}
      />,
    ],
  },
};

/**
 * At the 768px floor with the rail at 48px. The rows keep their shape and
 * their whole field set; only the headline and the branch truncate.
 */
export const AtTheWidthFloor: StoryObj = {
  render: () => (
    <div style={{ width: "calc(var(--window-floor) - var(--sidebar-rail))" }}>
      <ActiveJobsList {...(SixStates.args as ActiveJobsListProps)} />
    </div>
  ),
};

/**
 * No rows, and no `empty` supplied. The bare frame is what renders — which is
 * the visible shape of `Board empty state` not existing yet. It has a
 * `components.toml` row at status Missing and is not built here; the two Fleet
 * readings a first launch shows are that component's to carry.
 */
export const EmptyWithNoEmptyState: Story = {
  args: {
    heading: "Active jobs",
    summary: "No active jobs. 3 waiting on the Job Board.",
    action: <Button variant="primary">New job</Button>,
    children: [],
  },
};
