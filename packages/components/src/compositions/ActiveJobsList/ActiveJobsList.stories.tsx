import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ReactElement } from "react";
import { cloneElement, useState } from "react";
import { Check, CircleDot, Cpu, GitBranch, Layers, OctagonAlert, Power, UserCheck, X } from "lucide-react";
import { expect } from "storybook/test";
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
 * Six states, one row shape. The running row is the only one here, so it
 * carries the loop. **Each row is washed in its own status hue** at
 * `--row-tint`, so the list sorts by state before a badge is read.
 *
 * **One, because one row is running here — not because only one can be.** This
 * paragraph used to say the pulse rode the status "focused or not, because
 * Fleet runs one drone at M1". Fleet runs several now, and the pulse is the
 * cursor's: this list is a plain list with no cursor in it, so the running row
 * takes the mark. `Two jobs running at once` is the same rule where it bites.
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
        handle="7-coalesce-token-refreshes"
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
        // `cpu`, not `clock`, and the verb goes with the glyph. `enum-verbs.toml`
        // gives job_status.queued the clock and then says a reason's verb *and*
        // glyph replace it where one is set; this row's reason is
        // waiting_on_resources, whose pair is cpu and "waiting on resources".
        // The label read "Queued" while the glyph read cpu, which is half the
        // rule applied — and the resource it waits on is now real: the
        // concurrency cap bounds how many drones Fleet runs at once.
        statusIcon={Cpu}
        statusLabel="Waiting on resources"
        headline="Retire the legacy poke path"
        jobId="job_8b42"
        handle="6-retire-the-legacy-poke-path"
        fields={[
          { value: WORKFLOW },
          { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
          // The step, like every other row's third field. It read "Waiting on a
          // drone", which said the reason a second time and said it wrong: the
          // Job is behind the cap rather than behind the one drone there used
          // to be. The badge carries the reason; a field does not repeat it.
          { value: "Not started", quiet: true },
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
        handle="12-split-the-settings-reducer"
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
        handle="14-cache-the-manifest-read"
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
        handle="21-retry-ceiling-on-poke-loop"
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
        handle="18-rename-session-token"
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
 * The same six, where the failed Job failed 12 seconds ago. Its row carries the stronger tint and
 * the note; every other row rests at `--row-tint`. Held still, so a screenshot is stable.
 */
export const OneRowMidDecay: Story = {
  name: "One row mid-decay",
  args: {
    ...SixStates.args,
    children: (SixStates.args?.children as ReactElement<JobRowStackedProps>[]).map((row) =>
      row.props.status === "completed-failed"
        ? cloneElement(row, { changed: { note: "Failed · 12s ago", remaining: 0.73 } })
        : row,
    ),
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
 *
 * **The running row does not pulse here and the approval row is where the
 * cursor is.** A cursor exists in this list, so the mark is its to carry;
 * arrow down twice and the pulse arrives with it. `Six states` is the same
 * rows with no cursor at all, and there the running row keeps it.
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
  /**
   * **The paragraph above says to read this with the keyboard rather than the
   * eye, so this is that reading.** Every claim it makes is invisible in a
   * drawing: a list of six tab stops and a list with one cursor render
   * identically, and the difference is the whole reason the role was changed.
   *
   * `tabindex` is read directly because it *is* the claim. It is not styling —
   * it is which of six rows Tab will find, and there is no rendered mark for
   * it.
   */
  play: async ({ canvas, userEvent }) => {
    const rows = canvas.getAllByRole("option");
    await expect(rows).toHaveLength(6);

    // One tab stop for the list, on the first row — where a list nobody has
    // touched should put it.
    await expect(rows.filter((row) => row.getAttribute("tabindex") === "0")).toEqual([rows[0]]);

    rows[0]!.focus();
    await userEvent.keyboard("{ArrowDown}{ArrowDown}");
    await expect(rows[2]).toHaveFocus();

    // And the row the cursor left gave up its tab stop, which is the half a
    // roving implementation forgets: two rows at `tabindex=0` is a list that
    // Tab enters twice.
    await expect(rows[0]).toHaveAttribute("tabindex", "-1");

    await userEvent.keyboard("{End}");
    await expect(rows[5]).toHaveFocus();

    // Clamped, not wrapped. A Board is scanned, and a list that jumps back to
    // the top loses the reader's place.
    await userEvent.keyboard("{ArrowDown}");
    await expect(rows[5]).toHaveFocus();

    await userEvent.keyboard("{Home}");
    await expect(rows[0]).toHaveFocus();
  },
};

/**
 * Two jobs running at once, which is a board this list could not hold until
 * Fleet's working slot became a roster of them.
 *
 * **Both rows read as running and one of them breathes.** Hue says which Jobs
 * are running, on both; the pulse says *still working*, and the Motion section
 * asks that of the row being read and no other. Two marks at
 * `--duration-pulse` is what that scope avoids, and it is what this
 * list drew for as long as the pulse followed the status — invisibly, because
 * a second running row was unreachable.
 *
 * The cursor starts on the first row, so the pulse starts there. Arrow down
 * onto the second running row and the mark moves with it: one screen, one
 * animated mark, wherever the eye is.
 *
 * **Nothing here says two is the ceiling.** The bound is
 * `settings.concurrency-cap`, resolved in Fleet's composition root, and no
 * route or event carries it — so a list cannot say "2 of 2" without inventing
 * the number. A Job held back by the cap says so on its own row instead, which
 * is the `waiting on resources` badge above.
 */
export const TwoRunning: Story = {
  args: {
    heading: "Active jobs",
    summary: "3 jobs.",
    selectable: true,
    label: "Active jobs",
    children: [
      <JobRowStacked
        key="a"
        status="running"
        statusIcon={CircleDot}
        statusLabel="Running"
        headline="Split the settings reducer"
        jobId="job_2d90bb"
        handle="12-split-the-settings-reducer"
        pulsing
        onOpen={() => {}}
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
        key="b"
        status="running"
        statusIcon={CircleDot}
        statusLabel="Running"
        headline="Coalesce concurrent token refreshes"
        jobId="job_7c31"
        handle="7-coalesce-token-refreshes"
        pulsing
        onOpen={() => {}}
        fields={[
          { value: "bug/token-refresh", mono: true, icon: GitBranch, copyValue: "bug/token-refresh" },
          { value: <StepBar total={4} current={1} activity="running" label="Step 1 of 4" /> },
          { value: "Plan", emphasis: true },
          { value: "2m 47s", mono: true },
          { value: "~$0.30", mono: true },
        ]}
        action={<SplitButton ground="card" items={menu}>Open</SplitButton>}
      />,
      <JobRowStacked
        key="c"
        status="not-started"
        statusIcon={Cpu}
        statusLabel="Waiting on resources"
        headline="Retire the legacy poke path"
        jobId="job_8b42"
        handle="6-retire-the-legacy-poke-path"
        onOpen={() => {}}
        fields={[
          { value: WORKFLOW },
          { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
          { value: "Not started", quiet: true },
          { value: "approved 09:20", quiet: true },
          { value: "Dispatched by you" },
        ]}
        action={<SplitButton ground="card" items={menu}>Open</SplitButton>}
      />,
    ],
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
      cloneElement((SixStates.args?.children as ReactElement<JobRowStackedProps>[])[0]!, {
        onOpen: () => {},
      }),
    ],
  },
  /**
   * The state the prose above calls the one a clamp gets wrong most easily —
   * and the one where getting it wrong is silent, because a cursor wrapping
   * onto the only row it could be on looks exactly like a cursor staying put.
   *
   * All four keys, because a clamp is four expressions and each can be wrong
   * on its own.
   */
  play: async ({ canvas, userEvent }) => {
    const row = canvas.getByRole("option");
    row.focus();

    await userEvent.keyboard("{ArrowDown}{ArrowUp}{Home}{End}");
    await expect(row).toHaveFocus();
    await expect(row).toHaveAttribute("tabindex", "0");
  },
};

/** The Board's table: long titles, and handles at Fleet's 44-character limit, which is what a day's work leaves. */
const TABLE_ROWS = [
  { status: "awaiting-approval", icon: UserCheck, label: "Needs approval", headline: "Board's Clear button should reclaim worktrees a Job left behind", handle: "1-board-s-clear-button-should-reclaim-worktr", act: "Approve" },
  { status: "running", icon: CircleDot, label: "Running", headline: "Refuse a merge press whose chosen comments were never posted", handle: "2-refuse-a-merge-press-whose-chosen-comments", act: "Open" },
  { status: "completed-failed", icon: X, label: "Failed", headline: "Cache the manifest read", handle: "14-cache-the-manifest-read", act: "Open" },
] as const;

/** Every drawn row opens a Job on the real Board (`selectable`), which is what lets a row's cursor reveal what its own columns gave up. */
function TableAt({ width }: { width: string }) {
  return (
    <div style={{ width }}>
      <ActiveJobsList heading="Job Board" summary="3 jobs." view="table" selectable label="Job Board" columns={["Workflow", "Progress", "Run time"]}>
        {TABLE_ROWS.map((row) => (
          <JobRowStacked
            key={row.handle}
            status={row.status}
            statusIcon={row.icon}
            statusLabel={row.label}
            headline={row.headline}
            jobId={row.handle}
            handle={row.handle}
            onOpen={() => {}}
            fields={[
              { label: "Workflow", icon: Layers, value: "Bug, 6 steps" },
              {
                label: "Progress",
                value: (
                  <>
                    <StepBar total={6} current={2} activity="running" label="Step 2 of 6" />
                    <span className="armada-row-step">regression_verify</span>
                  </>
                ),
              },
              { label: "Run time", value: "11h 00m", mono: true },
            ]}
            action={<SplitButton ground="card" items={menu}>{row.act}</SplitButton>}
          />
        ))}
      </ActiveJobsList>
    </div>
  );
}

/** Every row shows its action inside the frame, its handle whole, and a readable part of its title (#925). */
async function everyRowReads(canvasElement: HTMLElement) {
  const frame = canvasElement.querySelector(".armada-active-jobs__frame")!.getBoundingClientRect();
  const rows = [...canvasElement.querySelectorAll<HTMLElement>(".armada-job-row")];
  await expect(rows).toHaveLength(TABLE_ROWS.length);
  for (const row of rows) {
    const title = row.querySelector<HTMLElement>(".armada-job-row__title")!;
    const handle = row.querySelector<HTMLElement>(".armada-job-row__id")!;
    const headline = row.querySelector<HTMLElement>(".armada-job-row__headline")!.getBoundingClientRect();
    await expect(row.querySelector(".armada-job-row__action")!.getBoundingClientRect().right).toBeLessThanOrEqual(frame.right);
    // Eight ems of title is a phrase, where the title used to keep a letter or two.
    await expect(title.getBoundingClientRect().width).toBeGreaterThanOrEqual(parseFloat(getComputedStyle(title).fontSize) * 8);
    await expect(handle.scrollWidth).toBeLessThanOrEqual(handle.clientWidth);
    await expect(handle.getBoundingClientRect().right).toBeLessThanOrEqual(headline.right);
    await expect(handle.getBoundingClientRect().bottom).toBeLessThanOrEqual(headline.bottom);
  }
}

/** Reachable by mouse or keyboard — hover or focus floats it free of its cramped column (#984). */
async function workflowReveals(frame: DOMRect, row: HTMLElement) {
  const value = row.querySelector<HTMLElement>(".armada-job-row__field-value")!;
  await expect(value).toHaveTextContent("Bug, 6 steps");
  await expect(value.getBoundingClientRect().right).toBeLessThanOrEqual(frame.right);
}

/** At rest the name is in the accessibility tree but not painted — icon alone, never a clipped fragment (#984). */
async function workflowHiddenAtRest(row: HTMLElement) {
  const value = row.querySelector<HTMLElement>(".armada-job-row__field-value")!;
  await expect(value).toHaveTextContent("Bug, 6 steps");
  await expect(value.getBoundingClientRect().width).toBeLessThanOrEqual(1);
}

/** The table at the narrowest window, 768px less the rail: the facts give way, and the title, handle and action do not. */
export const TableAtTheWidthFloor: StoryObj = {
  render: () => <TableAt width="calc(var(--window-floor) - var(--sidebar-rail))" />,
  play: async ({ canvasElement, userEvent }) => {
    await everyRowReads(canvasElement);
    const frame = canvasElement.querySelector(".armada-active-jobs__frame")!.getBoundingClientRect();
    const rows = canvasElement.querySelectorAll<HTMLElement>('[role="option"]');
    await workflowHiddenAtRest(rows[2]!);
    await userEvent.hover(rows[0]!);
    await workflowReveals(frame, rows[0]!);
    await userEvent.unhover(rows[0]!);
    rows[1]!.focus();
    await workflowReveals(frame, rows[1]!);
  },
};

/** The table at the 1100px breakpoint, where the title used to give way to nothing beside a whole handle. */
export const TableAtTheBreakpoint: StoryObj = {
  render: () => <TableAt width="calc(var(--layout-breakpoint) - var(--sidebar-rail))" />,
  play: async ({ canvasElement }) => everyRowReads(canvasElement),
};

/** The badge, the title, the handle and the action all stay inside the frame — no ellipsis, and nothing off its right edge. */
async function escalatedRowStaysInFrame(canvasElement: HTMLElement) {
  const frame = canvasElement.querySelector(".armada-active-jobs__frame")!.getBoundingClientRect();
  const row = canvasElement.querySelector<HTMLElement>(".armada-job-row")!;
  const badge = row.querySelector<HTMLElement>(".armada-badge")!;
  await expect(badge).toHaveTextContent("A required command did not succeed");
  await expect(badge.getBoundingClientRect().right).toBeLessThanOrEqual(frame.right);
  const title = row.querySelector<HTMLElement>(".armada-job-row__title")!;
  await expect(title).toHaveTextContent("Reconcile orphaned drones on Fleet start");
  await expect(row.querySelector(".armada-job-row__action")!.getBoundingClientRect().right).toBeLessThanOrEqual(frame.right);
}

/** The registry's longest verb (34 chars), inside a real list at the 720px floor: stays whole, own line, inside the frame (#984, #914). */
export const EscalatedBadgeInTheTableAtTheWidthFloor: StoryObj = {
  render: () => (
    <div style={{ width: "calc(var(--window-floor) - var(--sidebar-rail))" }}>
      <ActiveJobsList
        heading="Job Board"
        summary="1 job."
        view="table"
        selectable
        label="Job Board"
        columns={["Workflow", "Progress", "Run time"]}
      >
        <JobRowStacked
          status="escalated"
          statusIcon={OctagonAlert}
          statusLabel="A required command did not succeed"
          headline="Reconcile orphaned drones on Fleet start"
          jobId="job_31c7"
          handle="31-reconcile-orphaned-drones-fleet-start"
          onOpen={() => {}}
          fields={[
            { label: "Workflow", icon: Layers, value: "Bug, 6 steps" },
            {
              label: "Progress",
              value: (
                <>
                  <StepBar total={6} current={2} activity="running" label="Step 2 of 6" />
                  <span className="armada-row-step">regression_verify</span>
                </>
              ),
            },
            { label: "Run time", value: "1h 12m", mono: true },
          ]}
          action={<SplitButton ground="card" items={menu}>Open</SplitButton>}
        />
      </ActiveJobsList>
    </div>
  ),
  play: async ({ canvasElement }) => escalatedRowStaysInFrame(canvasElement),
};

/**
 * A `panel` list's own fold, Overview 27's addition (#1091). **Collapses to
 * the head, never to nothing** — `Panel`'s own rule: the count stays put and
 * the rows leave, so a glance still answers how many without a press.
 *
 * Controlled here the way the app controls it: `open` and `onOpenChange` are
 * this story's own `useState`, standing in for the persisted preference
 * Overview keeps per panel.
 */
export const PanelFolded: StoryObj = {
  render: function Render() {
    const [open, setOpen] = useState(true);
    return (
      <ActiveJobsList
        variant="panel"
        heading="Running"
        count={1}
        open={open}
        onOpenChange={setOpen}
        selectable
        label="Running"
      >
        <JobRowStacked
          status="running"
          statusIcon={Cpu}
          statusLabel="Running"
          headline="Reconcile orphaned drones on Fleet start"
          jobId="job_31c7"
          handle="31-reconcile-orphaned-drones-fleet-start"
          onOpen={() => {}}
          fields={[{ value: WORKFLOW }]}
        />
      </ActiveJobsList>
    );
  },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("heading", { name: "Running" })).toBeVisible();
    await expect(canvas.getByRole("option")).toBeVisible();

    const toggle = canvas.getByRole("button", { name: "Collapse Running" });
    await userEvent.click(toggle);

    // Collapsed to the head: the count is still there, the row is not — and
    // it is gone rather than hidden, so a roving list finds nothing to rove.
    await expect(canvas.getByText("1")).toBeVisible();
    await expect(canvas.queryByRole("option")).toBeNull();
    await expect(canvas.getByRole("button", { name: "Expand Running" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );

    await userEvent.click(canvas.getByRole("button", { name: "Expand Running" }));
    await expect(canvas.getByRole("option")).toBeVisible();
  },
};
