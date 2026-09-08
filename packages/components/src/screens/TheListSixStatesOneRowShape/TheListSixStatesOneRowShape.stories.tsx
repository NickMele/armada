import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, CircleDot, Cpu, GitBranch, Power, UserCheck, X } from "lucide-react";
import { BoardControls } from "../../compositions/BoardControls/BoardControls";
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
 * The queued row's glyph is `cpu`, not `clock`, **and its verb is the reason's
 * too**: the registry's rule replaces both where a reason is present, and this
 * row's reason is `waiting_on_resources`. It read "Queued" beside the cpu
 * glyph until the resource became a real one — Fleet bounds how many drones it
 * runs at once, and a Job past the bound is held at `queued` for that reason
 * and no other.
 *
 * **Every row carries origin now, which is the Board's requirement and was
 * true of one row here.** #218 gave `Job row (stacked)` its sixth track and
 * left this file alone; the four rows that had run stopped at spend, so origin
 * was drawn at the gate and nowhere else. The two gate rows keep their own
 * five-track list — a Job with no worktree has a timestamp where a running row
 * has elapsed, and no spend at all — and both lists compose the same named
 * properties rather than repeating widths.
 *
 * **The five origin sentences here are literals, and Bridge draws nothing in
 * that track on a real row.** `origin` is on `JobSummary` and `enum-verbs.toml`
 * carries a row for each of its five values, `sub_dispatched` included — that
 * one as the form `Sub-dispatched by {dispatched_by.job_id}` rather than a
 * word, which #234 settled before it closed. What no generator emits is a map
 * carrying any of them into Bridge, and `JobSummary` carries no
 * `dispatched_by` for the form to interpolate, so these fixtures are the
 * drawing rather than proof the track is filled.
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
  // **Review, not Approve.** The drawing gave this row an Approve control and
  // flagged it as a departure from the settled rule that approval is a second
  // act from detail; it was settled 2026-08-31 in favour of the rule. Review is
  // the word an `awaiting_review` row already carries and means the same thing
  // in both places — go read this — because in both places the act is on
  // detail. Nothing on the Board approves. The dispatch proposal is the one
  // surface that does, because there what the gate approves is on screen
  // already — settled 2026-09-08, and it changes nothing here.
  action: <SplitButton ground="card" items={menu}>Review</SplitButton>,
};

const queued: JobRowStackedProps = {
  status: "not-started",
  statusIcon: Cpu,
  // The reason supplies the verb as well as the glyph. This carried cpu with
  // "Queued" beside it, which is half the registry's rule, and the field
  // beneath said "Waiting on a drone" — the reason a second time, and named
  // for the one drone Fleet used to run. The resource is the concurrency cap.
  statusLabel: "Waiting on resources",
  headline: "Retire the legacy poke path",
  jobId: "job_8b42",
  tracks: APPROVAL_TRACKS,
  fields: [
    { value: workflow },
    { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
    { value: "Not started", quiet: true },
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
    { value: "Dispatched by you" },
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
    { value: "Found by Fleet" },
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
    { value: "Drafted in Helm" },
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
    { value: "Workflow-triggered" },
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
 * The Board with its controls and its keyboard model — sections 1 and 3 of the
 * drawing, which the six rows above were reproduced without.
 *
 * **The count states both numbers.** `1 job needs you. 6 on the Board.` The
 * first is the number a person is deciding whether to act on and the second is
 * what it is a fraction of; either alone is a number with nothing to compare
 * it against. The drawing's own fixture reads `4 jobs need you. 15 on the
 * Board.` — the shape is the sentence, not the numerals.
 *
 * **Five tabs, and their counts are of what the search matched.** With no
 * search that is the whole board, which is why they read as the board here.
 *
 * **The cursor's row carries its key and no other row does.** The chip holds
 * its width on every row, so nothing moves as the cursor travels; what changes
 * is whether it is drawn.
 *
 * The keys are the contract's contextual tier — `docs/contracts/design-system.md`,
 * Keyboard and command palette — and none of them is decided here. What the
 * Board answers of it:
 *
 * | Key | Does |
 * |---|---|
 * | `/` | Search the list. `Esc` clears it and hands the cursor back |
 * | `j` `k` `↓` `↑` | Move the cursor; the accent left edge follows it |
 * | `Enter` `o` | Open the focused job's detail. One act, two names |
 * | `r` `t` `d` | Review, Attest, Redirect — only where the row carries that verb |
 * | `x` | Kill, and it confirms |
 * | `1`–`5` | Set the state filter, in tab order |
 * | `n` | New job, the one key that acts on nothing on screen |
 *
 * **There is no Approve key and no Approve control**, and `a` was deleted from
 * the map on 2026-08-31 for the reason this row shows: nothing on a list
 * approves. The row at the gate carries Review — see
 * `docs/concepts/job-board.md`.
 */
export const TheBoard: Story = {
  render: () => (
    <div className="armada-screen">
      <TheListSixStatesOneRowShape
        heading="Active jobs"
        summary="1 job needs you. 6 on the Board."
        action={<Button variant="primary">New job</Button>}
        controls={
          <BoardControls
            query=""
            onQuery={() => {}}
            searchKey="/"
            sorts={[
              { id: "critical_first", label: "Critical first" },
              { id: "oldest_first", label: "Oldest first" },
            ]}
            sort="critical_first"
            onSort={() => {}}
            tabs={[
              { id: "all", label: "All", count: 6, shortcut: "1" },
              { id: "needs-you", label: "Needs you", count: 1, shortcut: "2" },
              { id: "running", label: "Running", count: 1, shortcut: "3" },
              { id: "queued", label: "Queued", count: 1, shortcut: "4" },
              { id: "finished", label: "Finished", count: 3, shortcut: "5" },
            ]}
            tab="all"
            onTab={() => {}}
          />
        }
        rows={SIX.map((row, i) => ({
          ...row,
          actionKey: KEYS[i],
          focused: i === 0 || undefined,
        }))}
      />
    </div>
  ),
};

/**
 * Which key each of the six rows answers to, in the order they are drawn.
 *
 * **One control per row, so at most one key ever applies.** The gate row
 * carries Review and answers `r`; the other five carry Open and answer `o`.
 * Every other verb key no-ops on every one of these rows rather than acting on
 * the wrong verb.
 *
 * `t` and `d` reach nothing here, and that is the fixture rather than the map:
 * none of the six is at `awaiting_attestation` and none is being piloted, so
 * neither Attest nor Redirect is a control any of these rows carries.
 */
const KEYS = ["r", "o", "o", "o", "o", "o"];

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
 * **The same six rows, carrying only what Fleet serves a list.** Four of the
 * drawing's five fields survive the trip now, and this story is here so what
 * does not is something you can see rather than something you have to be told.
 *
 * `branch` and `created_at` are on `JobSummary`, so track one makes the
 * drawing's switch — the branch the moment a worktree exists, the workflow
 * until then — and elapsed is measured from creation to now.
 *
 * | Field | Why it is not here |
 * |---|---|
 * | Spend | Measured nowhere — not on the wire, not in the store, not computed |
 * | `Dispatched by you` | `origin` is on `JobSummary` and `enum-verbs.toml` carries all five rows, but no generator emits them — the wanted list in `apps/desktop/codegen/vocabulary.mjs` does not name `origin`, so nothing carries the words across and Bridge would have to retype them. Adding it is not the whole fix: `JobSummary` carries no `dispatched_by`, so `sub_dispatched`'s form has no slot to fill |
 * | Elapsed on a Job that is over | `JobSummary` carries no instant the Job stopped at, and a terminal elapsed running to now would read as still working |
 *
 * The step is its `step_id`, in mono: `StepDetail` carries a label, but a list
 * row holds `JobSummary` and the summary carries only the id. The name is one
 * click away, on the rail.
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
          // Track one is the branch where the row has one and the workflow
          // where it does not; the step is the id it was dispatched on; and
          // elapsed is only on a Job still working. Spend is dropped, never
          // drawn empty.
          fields: [
            row === awaitingApproval || row === queued ? { value: workflow } : row.fields[0]!,
            row.fields[1]!,
            row === awaitingApproval || row === queued
              ? { value: "Not started", quiet: true }
              : { ...row.fields[2]!, mono: true },
            ...(row === awaitingApproval || row === queued || row === running
              ? [{ value: STILL_RUNNING[SIX.indexOf(row)] ?? "", mono: true, quiet: true }]
              : []),
          ],
        }))}
      />
    </div>
  ),
};

/**
 * **The same six Jobs, in the table view.** Not a second component: these are
 * the rows above with their facts named and reordered, and the arrangement is
 * one prop on the list.
 *
 * **The header is what buys the height back.** A card labels a fact by where it
 * sits in a run a person has to learn; a table names it once at the top for the
 * whole Board. So this row is `--h-row-table` at 52px against the card's 84px,
 * and it says more rather than less, because every column has a word over it.
 *
 * **Three columns, not four.** `Dispatched by` is the fourth fact the Board
 * wants and the one it cannot draw. `enum-verbs.toml` carries every `origin`
 * row and `sub_dispatched`'s form, both settled when #234 closed; what is left
 * is that no map reaches Bridge, and that `JobSummary` carries no
 * `dispatched_by` for that form to interpolate. A named column with nothing
 * under it reads as a value that failed to load, which is the argument that
 * already keeps spend off the row.
 *
 * **Progress is one cell, where the card spends two tracks on it.** A column
 * called Progress answering in two places would need two names. The bar keeps
 * its 72px and the step sits beside it.
 *
 * The branch is not here either. On a card it shares track one with the
 * workflow and the row draws whichever it has; a named column cannot do that,
 * so the column says Workflow and carries the workflow.
 */
export const TheBoardAsATable: Story = {
  render: () => (
    <div className="armada-screen">
      <TheListSixStatesOneRowShape
        heading="Active jobs"
        summary="1 job needs you. 6 on the Board."
        action={<Button variant="primary">New job</Button>}
        view="table"
        columns={["Workflow", "Progress", "Run time"]}
        controls={
          <BoardControls
            query=""
            onQuery={() => {}}
            searchKey="/"
            view="table"
            onView={() => {}}
            sorts={[
              { id: "critical_first", label: "Critical first" },
              { id: "oldest_first", label: "Oldest first" },
            ]}
            sort="critical_first"
            onSort={() => {}}
            tabs={[
              { id: "all", label: "All", count: 6, shortcut: "1" },
              { id: "needs-you", label: "Needs you", count: 1, shortcut: "2" },
              { id: "running", label: "Running", count: 1, shortcut: "3" },
              { id: "queued", label: "Queued", count: 1, shortcut: "4" },
              { id: "finished", label: "Finished", count: 3, shortcut: "5" },
            ]}
            tab="all"
            onTab={() => {}}
          />
        }
        rows={SIX.map((row, i) => ({
          ...row,
          // `tracks` is the card's standalone fallback and means nothing here:
          // inside a list the columns are the list's.
          tracks: undefined,
          fields: asCells(row),
          actionKey: KEYS[i],
          focused: i === 0 || undefined,
        }))}
      />
    </div>
  ),
};

/**
 * A card's field run, read as the three cells the table names.
 *
 * **The fixtures are the card's**, and reshaping them here rather than writing
 * a second set is the point: two sets authored separately drift, and the whole
 * claim of the toggle is that it is one row's facts in two arrangements.
 */
function asCells(row: JobRowStackedProps): JobRowStackedProps["fields"] {
  const [origin, bar, step, ...rest] = row.fields;
  // **Only a mono field, and no fallback to the last one.** The gate row's run
  // ends `created 09:12` then `Dispatched by you`, and taking the last of them
  // put a person's name under Run time. A Job that has not started has no run
  // time, and the column says so with a dash rather than with whatever was
  // nearest.
  const elapsed = rest.find((field) => field.mono === true);
  return [
    { label: "Workflow", value: origin?.value, mono: origin?.mono, icon: origin?.icon },
    {
      label: "Progress",
      value: (
        <>
          {bar?.value}
          <span className="armada-row-step">{step?.value}</span>
        </>
      ),
    },
    {
      label: "Run time",
      value: elapsed?.value ?? "—",
      mono: true,
      quiet: elapsed === undefined || undefined,
    },
  ];
}

/**
 * How long each row has been alive, for the three that are not over. Written
 * here because a story is a fixture; the app measures it from `created_at`.
 */
const STILL_RUNNING: Record<number, string> = { 0: "1h 04m", 1: "38m 12s", 2: "11m 03s" };
