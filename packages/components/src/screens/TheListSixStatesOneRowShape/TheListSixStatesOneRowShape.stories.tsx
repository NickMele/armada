import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, CircleDot, Cpu, GitBranch, Power, UserCheck, X } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { StepBar } from "../../compositions/StepBar/StepBar";
import type { JobRowStackedProps } from "../../compositions/JobRowStacked/JobRowStacked";
import { APPROVAL_TRACKS, TheListSixStatesOneRowShape } from "./TheListSixStatesOneRowShape";

/**
 * Journey · Monitor Active Work. Six Job states, one row shape, in the order
 * Fleet supplies: the one row that needs a person first, the rest newest work
 * first.
 *
 * **One story per state, and then the six together.** The list is what a person
 * looks at, so it stays; the six are what a row is, and a row is what changes.
 *
 * The badge on the awaiting-approval row reads **Needs approval**, which is
 * what `enum-verbs.toml` holds. The drawing writes "Awaiting approval". A
 * status label is never written by hand, so the registry wins on the word and
 * the drawing wins on everything else. Reported.
 *
 * The queued row's glyph is `cpu`, not `clock`: the registry's own rule is that
 * a reason's glyph replaces `clock` where one is present, and M1's only queued
 * reason is `waiting_on_resources`.
 */
const meta: Meta<typeof TheListSixStatesOneRowShape> = {
  title: "Screens/The list — six states, one row shape",
  component: TheListSixStatesOneRowShape,
};
export default meta;

type Story = StoryObj<typeof TheListSixStatesOneRowShape>;

const menu = [
  { label: "Copy job id", shortcut: "⌘C" },
  { label: "Kill", shortcut: "x", danger: true },
];

const open = (
  <SplitButton ground="card" items={menu}>
    Open
  </SplitButton>
);

const workflow = (
  <>
    <span className="armada-screen__mono">bug</span>, 4 steps
  </>
);

const awaitingApproval: JobRowStackedProps = {
  status: "awaiting-approval",
  statusIcon: UserCheck,
  statusLabel: "Needs approval",
  headline: "Coalesce concurrent token refreshes",
  jobId: "job_7c31",
  tracks: APPROVAL_TRACKS,
  fields: [
    { value: workflow },
    { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
    { value: "Not started", quiet: true },
    { value: "created 09:12", quiet: true },
    { value: "Dispatched by you" },
  ],
  action: (
    <SplitButton ground="card" items={[{ label: "Reject", danger: true }]}>
      Approve
    </SplitButton>
  ),
};

const queued: JobRowStackedProps = {
  status: "not-started",
  statusIcon: Cpu,
  statusLabel: "Queued",
  headline: "Retire the legacy poke path",
  jobId: "job_8b42",
  fields: [
    { value: workflow },
    { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
    { value: "Waiting on a drone", emphasis: true },
    { value: "approved 09:20", quiet: true },
    { value: "Dispatched by you" },
  ],
  action: open,
};

const running: JobRowStackedProps = {
  status: "running",
  statusIcon: CircleDot,
  statusLabel: "Running",
  headline: "Split the settings reducer",
  jobId: "job_2d90bb",
  pulsing: true,
  fields: [
    {
      value: "fix/settings-split",
      mono: true,
      icon: GitBranch,
      copyValue: "fix/settings-split",
    },
    { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
    { value: "Implement", emphasis: true },
    { value: "11m 03s", mono: true },
    { value: "~$1.80", mono: true },
  ],
  action: open,
};

const failed: JobRowStackedProps = {
  status: "completed-failed",
  statusIcon: X,
  statusLabel: "Failed",
  headline: "Cache the manifest read",
  jobId: "job_91ab",
  fields: [
    {
      value: "feat/manifest-cache",
      mono: true,
      icon: GitBranch,
      copyValue: "feat/manifest-cache",
    },
    { value: <StepBar total={4} current={3} activity="failed" label="Step 3 of 4" /> },
    { value: "Run tests", emphasis: true },
    { value: "22m 41s", mono: true },
    { value: "~$2.10", mono: true },
  ],
  action: open,
};

const done: JobRowStackedProps = {
  status: "completed-success",
  statusIcon: Check,
  statusLabel: "Done",
  headline: "Add a retry ceiling to the poke loop",
  jobId: "job_4f10",
  fields: [
    {
      value: "fix/poke-ceiling",
      mono: true,
      icon: GitBranch,
      copyValue: "fix/poke-ceiling",
    },
    {
      value: (
        <StepBar total={4} current={5} activity="advanced" label="All 4 of 4 steps advanced" />
      ),
    },
    { value: "Summarise" },
    { value: "18m 22s", mono: true },
    { value: "~$2.40", mono: true },
  ],
  action: open,
};

const killed: JobRowStackedProps = {
  status: "killed",
  statusIcon: Power,
  statusLabel: "Killed",
  headline: "Rename the session token field",
  jobId: "job_5e88",
  fields: [
    {
      value: "feat/session-rename",
      mono: true,
      icon: GitBranch,
      copyValue: "feat/session-rename",
    },
    { value: <StepBar total={4} current={2} activity="killed" label="Step 2 of 4" /> },
    { value: "Implement", emphasis: true },
    { value: "4m 09s", mono: true },
    { value: "~$0.60", mono: true },
  ],
  action: open,
};

const SIX = [awaitingApproval, queued, running, failed, done, killed];

/** One row, framed, with no count sentence above it — a row is not a surface. */
function one(row: JobRowStackedProps) {
  return (
    <div className="armada-screen">
      <TheListSixStatesOneRowShape label="Active jobs" rows={[row]} />
    </div>
  );
}

export const TheList: Story = {
  render: () => (
    <div className="armada-screen">
      <TheListSixStatesOneRowShape
        heading="Active jobs"
        summary="6 jobs. 1 awaiting approval."
        action={<Button variant="primary">New job</Button>}
        rows={SIX}
      />
    </div>
  ),
};

/**
 * The gate. **No branch, because no worktree exists** — track one is the
 * workflow until dispatch creates one, and the bar is drawn empty rather than
 * left out: a Job at the gate has its ordinals and no progress.
 */
export const AwaitingApproval: Story = { render: () => one(awaitingApproval) };

/** Approved, waiting on a drone. Still no worktree, so still no branch. */
export const Queued: Story = { render: () => one(queued) };

/** Dispatched: track one becomes the branch, and elapsed starts moving. */
export const Running: Story = { render: () => one(running) };

export const Failed: Story = { render: () => one(failed) };

export const Done: Story = { render: () => one(done) };

export const Killed: Story = { render: () => one(killed) };

/**
 * **The same six rows, carrying only what Fleet serves a list.** Three of the
 * drawing's five fields survive the trip, and this story is here so the gap is
 * something you can see rather than something you have to be told.
 *
 * | Field | Why it is not here |
 * |---|---|
 * | Branch | `JobDetail`, not `JobSummary`. One request per row is the failure `docs/practices/bridge.md` names first |
 * | Elapsed, `created 09:12`, `approved 09:20` | `created_at` is `JobDetail` too, and no instant on the summary stands in for it |
 * | `Dispatched by you` | No actor on the row, and `origin` has no verb in `enum-verbs.toml` |
 * | Spend | Measured nowhere — not on the wire, not in the store, not computed |
 *
 * The step's name is its id, because `StepDetail` carries no label. Issue #109.
 *
 * Absent rather than blank, in every case: a labelled gap on every row of the
 * list reads as a value that failed to load, which is worse than a shorter row.
 */
export const WhatTheWireServes: Story = {
  render: () => (
    <div className="armada-screen">
      <TheListSixStatesOneRowShape
        heading="Active jobs"
        summary="6 jobs. 1 awaiting approval."
        rows={SIX.map((row) => ({
          ...row,
          tracks: undefined,
          // Track one is the workflow on every row, because the field that
          // would make it the branch is not on the summary. The third field is
          // `Not started` wherever no step has been entered — the queued row's
          // reason is the badge's verb, and saying it twice is saying it twice.
          fields: [
            { value: workflow },
            row.fields[1]!,
            row === awaitingApproval || row === queued
              ? { value: "Not started", quiet: true }
              : row.fields[2]!,
          ],
        }))}
      />
    </div>
  ),
};
