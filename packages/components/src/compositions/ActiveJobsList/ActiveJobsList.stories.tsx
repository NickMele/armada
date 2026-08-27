import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ReactElement } from "react";
import { cloneElement } from "react";
import { Check, CircleDot, Cpu, GitBranch, Power, UserCheck, X } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { JobRowStacked, type JobRowStackedProps } from "../JobRowStacked/JobRowStacked";
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

/* The first field of a job that has not run: the workflow's id in mono, then
   how many steps it has in sans. The drawing writes no "Workflow" label — the
   value names itself. */
const WORKFLOW = (
  <>
    <span style={{ fontFamily: "var(--font-mono)" }}>bug</span>, 4 steps
  </>
);

const menu = [
  { label: "Copy job id", shortcut: "⌘C" },
  { label: "Kill", shortcut: "x", danger: true },
];

/**
 * Six states, one row shape. The one running row carries the one pulse on the
 * screen, focused or not — there is only ever one, because Fleet runs one
 * drone at M1, and fourteen breathing dots is what the motion rules forbid.
 */
export const SixStates: Story = {
  args: {
    heading: "Active jobs",
    summary: "6 jobs. 1 awaiting approval.",
    action: <Button variant="primary">New job</Button>,
    children: [
      // "Needs approval" is what `enum-verbs.toml` holds for
      // `job_status.awaiting_approval`, and its note says the wording is
      // deliberate: the badge means a person must act, not that time is
      // passing. The M1 drawing writes "Awaiting approval". A status label is
      // never written by hand, so the registry wins here where the drawing
      // wins on arrangement. Reported.
      <JobRowStacked
        key="a"
        status="awaiting-approval"
        statusIcon={UserCheck}
        statusLabel="Needs approval"
        headline="Coalesce concurrent token refreshes"
        jobId="job_7c31"
        fields={[
          { value: WORKFLOW },
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
        // `cpu`, not `clock`. `enum-verbs.toml` gives job_status.queued the
        // clock and then says a reason's verb and glyph replace it where one is
        // set; M1's only queued reason is waiting_on_resources, whose glyph is
        // cpu. The drawing draws cpu for the same reason.
        statusIcon={Cpu}
        statusLabel="Queued"
        headline="Retire the legacy poke path"
        jobId="job_8b42"
        fields={[
          { value: WORKFLOW },
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

/**
 * Twice the width floor — 1536px, wider than the drawing and wider than the
 * window most of the time. **The field run reaches the right edge and nothing
 * truncates while there is room**, because the tracks are the list's and grow
 * with it; before subgrid they were five fixed lengths on each row and the run
 * stopped short whatever the window did.
 *
 * Read this beside `At the width floor`: the same six rows, the same fields,
 * the columns still lining up down the list at both ends.
 */
export const AtAWideWindow: StoryObj = {
  render: () => (
    <div style={{ width: "calc(var(--window-floor) * 2)" }}>
      <ActiveJobsList {...(SixStates.args as ActiveJobsListProps)} />
    </div>
  ),
};

/**
 * The list Bridge draws: every row opens a Job, so the frame is a listbox and
 * the rows are options. Tab reaches a row, Enter and Space open it, and the
 * open one carries `aria-selected` as well as the accent fill.
 *
 * **This is also the roving state.** Tab lands on one row and one only; Up and
 * Down move the cursor, Home and End go to the ends, and the row the cursor
 * leaves gives up its tab stop. Read it with the keyboard rather than the eye —
 * the difference from a list of six tab stops is invisible in a screenshot.
 *
 * Clamped rather than wrapped: Down on the last row stays there. A Board is
 * scanned, and a list that jumps back to the top loses the reader's place.
 */
export const Selectable: Story = {
  args: {
    ...SixStates.args,
    selectable: true,
    label: "Active jobs",
    children: (SixStates.args?.children as ReactElement<JobRowStackedProps>[]).map((row, i) =>
      cloneElement(row, { onOpen: () => {}, selected: i === 2 }),
    ),
  },
};

/**
 * The same listbox with one row. **The roving cursor has nowhere to go**, and
 * both arrows leave it where it is rather than wrapping onto itself — the
 * state a clamp gets wrong most easily.
 */
export const OneOption: Story = {
  args: {
    heading: "Active jobs",
    summary: "1 job. 1 awaiting approval.",
    selectable: true,
    label: "Active jobs",
    children: [
      cloneElement((SixStates.args?.children as ReactElement<JobRowStackedProps>[])[0], {
        onOpen: () => {},
      }),
    ],
  },
};
