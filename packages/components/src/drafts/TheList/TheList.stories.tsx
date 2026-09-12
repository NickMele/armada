import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, CircleDot, Cpu, Power, UserCheck, X } from "lucide-react";
import { BoardControls } from "../../compositions/BoardControls/BoardControls";
import { Button } from "../../primitives/Button/Button";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { StepBar } from "../../compositions/StepBar/StepBar";
import { useState } from "react";
import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import type { JobRowField, JobRowStackedProps } from "../../compositions/JobRowStacked/JobRowStacked";
import { TheListSixStatesOneRowShape } from "./TheList";

/**
 * Journey · Monitor Active Work. Six Job states, one row shape, in the
 * order Fleet supplies: the one row that needs a person first, the rest
 * newest work first.
 *
 * One story per state, and then the six together: the list is what a
 * person looks at, so it stays; the six are what a row is, and a row is
 * what changes.
 */

/**
 * The badge on the awaiting-approval row reads Needs approval, which is
 * what `enum-verbs.toml` holds — the drawing writes "Awaiting approval".
 * A status label is never written by hand, so the registry wins on the
 * word and the drawing wins on everything else. Reported.
 *
 * The queued row's glyph is `cpu`, not `clock`, and its verb is the
 * reason's too: the registry's rule replaces both where a reason is
 * present, and this row's reason is `waiting_on_resources`. It read
 * "Queued" beside the cpu glyph until the resource became a real one —
 * Fleet bounds how many drones it runs at once, and a Job past the
 * bound is held at `queued` for that reason and no other.
 */

/**
 * Every row carries origin now, the Board's requirement, and was true
 * of one row here: #218 gave `Job row (stacked)` its sixth track and
 * left this file alone, so the four rows that had run stopped at spend
 * and origin was drawn at the gate only. The two gate rows keep their
 * own five-track list, sharing named track properties rather than
 * repeating widths.
 * The five origin sentences here are literals, and Bridge draws nothing
 * in that track on a real row: `origin` is on `JobSummary`, and
 * `enum-verbs.toml` carries a row for each of its five values,
 * `sub_dispatched` included, as `Sub-dispatched by
 * {dispatched_by.job_id}` rather than a word (#234). No generator emits
 * a map carrying any of them into Bridge, so these fixtures are the
 * drawing rather than proof the track is filled.
 */
const meta: Meta<typeof TheListSixStatesOneRowShape> = {
  title: "Drafts/The list",
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

/**
 * One Job, as the fixtures hold it — the facts, not a field run.
 *
 * **The six were six hand-written field arrays, and that is what made this file
 * hard to read.** Each arrangement wrote its own list, so a row could carry a
 * branch where the column said Workflow and nothing caught it: the table drew
 * four branches under WORKFLOW because it was handed track one, which on a card
 * is whichever of the two the Job has.
 *
 * Facts here, arrangements below. A Job has a workflow whether or not it has a
 * worktree, so a column named Workflow can always be filled.
 */
type Job = {
  status: string;
  statusIcon: LucideIcon;
  statusLabel: string;
  headline: string;
  jobId: string;
  /** The workflow and how many steps it has. Every Job has one. */
  workflow: string;
  /** Where the run got to, as the bar draws it. */
  bar: ReactNode;
  /** The step it is on, or `Not started`. */
  step: string;
  /** Absent on a Job that has not started, which is not the same as zero. */
  runTime?: string;
  /** Where the Job came from, one of `enum-verbs.toml`'s five origin rows. */
  origin: string;
  action: ReactNode;
};

/**
 * The four facts a row carries, named.
 *
 * **The name is data, and the arrangement decides how it is drawn** — stacked
 * over the value in small caps on a card, taken off the screen on a table where
 * the column header says it once. One list feeds both, which is what stops the
 * two from drifting.
 */
function factsOf(job: Job): JobRowField[] {
  return [
    { label: "Workflow", value: job.workflow },
    {
      label: "Progress",
      value: (
        <>
          {job.bar}
          <span className="armada-row-step">{job.step}</span>
        </>
      ),
    },
    {
      label: "Run time",
      // A dash, never a blank: a Job that has not started has no run time, and
      // an empty cell in a named column reads as one that failed to load.
      value: job.runTime ?? "\u2014",
      mono: true,
      quiet: job.runTime === undefined || undefined,
    },
    // **Origin, not "Dispatched by".** Only one of the five origin rows is a
    // dispatch: the others read `Found by Fleet`, `Drafted in Helm` and
    // `Workflow-triggered`, and none of them is something a person did. The
    // registry's verbs are whole sentences by design — `auto_detected`'s notes
    // say the row draws the string as written — so a label saying `Dispatched
    // by` over a value saying `Found by Fleet` contradicts itself on four rows
    // out of five.
    { label: "Origin", value: job.origin },
  ];
}

function rowOf(job: Job): JobRowStackedProps {
  return {
    status: job.status,
    statusIcon: job.statusIcon,
    statusLabel: job.statusLabel,
    headline: job.headline,
    jobId: job.jobId,
    fields: factsOf(job),
    action: job.action,
  };
}

const BAR_STEPS = 4;

const awaitingApproval: Job = {
  status: "awaiting-approval",
  statusIcon: UserCheck,
  statusLabel: "Needs approval",
  headline: "Coalesce concurrent token refreshes",
  jobId: "job_7c31",
  workflow: "bug, 4 steps",
  bar: <StepBar total={BAR_STEPS} current={0} label="Not started, 4 steps" />,
  step: "Not started",
  origin: "Dispatched by you",
  // **Review, not Approve.** The drawing gave this row an Approve control and
  // flagged it as a departure from the settled rule that approval is a second
  // act from detail; it was settled 2026-08-31 in favour of the rule. Review is
  // the word an `awaiting_review` row already carries and means the same thing
  // in both places — go read this — because in both places the act is on
  // detail. Nothing on the Board approves.
  action: (
    <SplitButton ground="card" items={menu}>
      Review
    </SplitButton>
  ),
};

const queued: Job = {
  status: "not-started",
  statusIcon: Cpu,
  // The reason supplies the verb as well as the glyph. This carried cpu with
  // "Queued" beside it, which is half the registry's rule, and the resource is
  // the concurrency cap rather than one drone.
  statusLabel: "Waiting on resources",
  headline: "Retire the legacy poke path",
  jobId: "job_8b42",
  workflow: "bug, 4 steps",
  bar: <StepBar total={BAR_STEPS} current={0} label="Not started, 4 steps" />,
  step: "Not started",
  origin: "Dispatched by you",
  action: open,
};

const running: Job = {
  status: "running",
  statusIcon: CircleDot,
  statusLabel: "Running",
  headline: "Split the settings reducer",
  jobId: "job_2d90bb",
  workflow: "feature, 4 steps",
  bar: <StepBar total={BAR_STEPS} current={2} activity="running" label="Step 2 of 4" />,
  step: "Implement",
  runTime: "11m 03s",
  origin: "Dispatched by you",
  action: open,
};

const failed: Job = {
  status: "completed-failed",
  statusIcon: X,
  statusLabel: "Failed",
  headline: "Cache the manifest read",
  jobId: "job_91ab",
  workflow: "bug, 4 steps",
  bar: <StepBar total={BAR_STEPS} current={3} activity="failed" label="Step 3 of 4" />,
  step: "Run tests",
  runTime: "22m 41s",
  origin: "Found by Fleet",
  action: open,
};

const done: Job = {
  status: "completed-success",
  statusIcon: Check,
  statusLabel: "Done",
  headline: "Add a retry ceiling to the poke loop",
  jobId: "job_4f10",
  workflow: "fix, 4 steps",
  bar: (
    <StepBar total={BAR_STEPS} current={5} activity="advanced" label="All 4 of 4 steps advanced" />
  ),
  step: "Summarise",
  runTime: "18m 22s",
  origin: "Drafted in Helm",
  action: open,
};

const killed: Job = {
  status: "killed",
  statusIcon: Power,
  statusLabel: "Killed",
  headline: "Rename the session token field",
  jobId: "job_5e88",
  workflow: "feature, 4 steps",
  bar: <StepBar total={BAR_STEPS} current={2} activity="killed" label="Step 2 of 4" />,
  step: "Implement",
  runTime: "4m 09s",
  origin: "Workflow-triggered",
  action: open,
};

const JOBS = [awaitingApproval, queued, running, failed, done, killed];
const SIX = JOBS.map(rowOf);

/** One row, framed, with no count sentence above it — a row is not a surface. */
function one(job: Job) {
  return (
    <div className="armada-screen">
      <TheListSixStatesOneRowShape label="Active jobs" rows={[rowOf(job)]} />
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
 * The Board with its controls and its keyboard model — sections 1 and 3
 * of the drawing, which the six rows above were reproduced without.
 *
 * The count states both numbers: `1 job needs you. 6 on the Board.` The
 * first is the number a person is deciding whether to act on and the
 * second is what it is a fraction of — the drawing's own fixture reads
 * `4 jobs need you. 15 on the Board.`, the shape is the sentence, not
 * the numerals.
 *
 * Five tabs, with counts of what the search matched — the whole board
 * with none. The cursor's row carries its key and no other row does:
 * the chip holds its width on every row, so nothing moves as the
 * cursor travels, only whether it is drawn.
 */

/**
 * The keys are the contract's contextual tier —
 * `docs/contracts/design-system.md`, Keyboard and command palette — and
 * none of them is decided here. What the Board answers of it:
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
 */

/**
 * There is no Approve key and no Approve control, and `a` was deleted
 * from the map on 2026-08-31 for the reason this row shows: nothing on
 * a list approves. The row at the gate carries Review — see
 * `docs/concepts/job-board.md`.
 */

/**
 * The Board, with a working switch. The toggle holds real state rather
 * than being drawn set, since the one thing worth checking about two
 * arrangements is that the same six rows survive moving between them —
 * a story that hard-coded `view` could not show that, and the switch
 * was missing from this story entirely while the table had its own.
 */
function Board({ start }: { start: "card" | "table" }) {
  const [view, setView] = useState<"card" | "table">(start);
  return (
    <div className="armada-screen">
      <TheListSixStatesOneRowShape
        heading="Active jobs"
        summary="1 job needs you. 6 on the Board."
        action={<Button variant="primary">New job</Button>}
        view={view}
        columns={COLUMNS}
        controls={
          <BoardControls
            query=""
            onQuery={() => {}}
            searchKey="/"
            view={view}
            onView={setView}
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
  );
}

/** What the table names its columns, in the order the rows supply their facts. */
const COLUMNS = ["Workflow", "Progress", "Run time", "Origin"];

export const TheBoard: Story = { render: () => <Board start="card" /> };

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
 * The same six rows with every fact Bridge cannot fill taken out. The
 * stories above are the drawing; this one is what a real Board can put
 * on the screen today, so the difference is something you look at
 * rather than something you have to be told. It was called `What the
 * wire serves`, which named the subject and not the question — it read
 * like a protocol dump next to five state stories. The point is the gap.
 *
 * | Taken out | Why Bridge cannot fill it |
 * |---|---|
 * | `Origin` | `origin` is on `JobSummary` and `enum-verbs.toml` carries all five rows, `sub_dispatched`'s form included. Nothing emits them: the wanted list in `apps/desktop/codegen/vocabulary.mjs` does not name `origin`. `JobSummary` also carries no `dispatched_by`, so that form has no slot to fill |
 * | Run time on a Job that is over | `JobSummary` carries no instant the Job stopped at, and an elapsed running to now would read as still working |
 */

/**
 * The step is its `step_id` rather than its name, in mono: `StepDetail`
 * carries a label and a list row holds `JobSummary`, which carries only
 * the id — the name is one click away, on the rail.
 *
 * Absent rather than blank, in every case: a named cell with nothing in
 * it reads as a value that failed to load, which is worse than a
 * shorter row.
 */
export const OnlyWhatBridgeCanFill: Story = {
  render: () => (
    <div className="armada-screen">
      <TheListSixStatesOneRowShape
        heading="Active jobs"
        summary="6 jobs. 1 awaiting approval."
        rows={JOBS.map((job) => ({
          ...rowOf(job),
          fields: factsOf({
            ...job,
            // The id, because that is what a summary carries.
            step: job.step === "Not started" ? "Not started" : job.jobId.replace("job_", "step_"),
            // Elapsed climbs only while the Job is still working.
            ...(job.status === "running" || job.status.startsWith("awaiting") || job.status === "not-started"
              ? {}
              : { runTime: undefined }),
          }).filter((field) => field.label !== "Origin"),
        }))}
      />
    </div>
  ),
};

/**
 * The same six Jobs, in the table view. Not a second component: these
 * are the rows above with their facts named and reordered, and the
 * arrangement is one prop on the list.
 *
 * The header is what buys the height back: a card labels a fact by
 * where it sits in a run a person has to learn, a table names it once
 * at the top for the whole Board. So this row is `--h-row-table` at
 * 52px against the card's 84px, and says more rather than less, since
 * every column has a word over it.
 */

/**
 * Three columns, not four: `Dispatched by` is the fourth fact the
 * Board wants and cannot draw. `enum-verbs.toml` carries every `origin`
 * row and `sub_dispatched`'s form, both settled when #234 closed; what
 * is left is that no map reaches Bridge, and `JobSummary` carries no
 * `dispatched_by` for that form to interpolate — a named column with
 * nothing under it reads as a value that failed to load.
 *
 * Progress is one cell, where the card spends two tracks on it — a
 * column called Progress answering in two places would need two names;
 * the bar keeps its 72px and the step sits beside it. The branch is not
 * here either: on a card it shares track one with the workflow and the
 * row draws whichever it has, but a named column cannot, so the column
 * says Workflow and carries the workflow.
 */
export const TheBoardAsATable: Story = { render: () => <Board start="table" /> };
