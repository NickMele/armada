import type { Meta, StoryObj } from "@storybook/react-vite";
import { ActivityLogSheet } from "../../compositions/ActivityLogSheet/ActivityLogSheet";
import { ActivityLog, type ActivityEntry } from "../../compositions/ActivityLog/ActivityLog";
import { JobDiffSheet, type JobDiffFile } from "../../compositions/JobDiffSheet/JobDiffSheet";
import { UnifiedDiff, type DiffFile } from "../../compositions/UnifiedDiff/UnifiedDiff";
import type { RunTreeStep } from "../../compositions/RunTree/RunTree";
import type { StepChapter } from "../../compositions/StepStory/StepStory";
import { Button } from "../../primitives/Button/Button";
import { JobDetailWithASheet } from "./JobDetailTheLogAndTheDiffAsSheets";
import {
  BRIEF,
  CHAPTERS,
  ESCALATED_HEADING,
  HEADING,
  JOB,
  RUN_RUNNING,
  WHERE,
  WHOLE,
} from "../InsideAJobOneArrangementAtEveryState/fixtures";

/**
 * **The same Job, with one thing on top of it.** Every story here is the Bug
 * workflow the rest of job detail draws, at the moment a reader opened
 * something the panel cannot hold. Nothing under the layer has moved.
 */
const meta: Meta<typeof JobDetailWithASheet> = {
  title: "Screens/Job detail — the log and the diff as sheets",
  component: JobDetailWithASheet,
};
export default meta;

type Story = StoryObj<typeof JobDetailWithASheet>;

/** The acts that end or replace the Job, behind the layer. */
const JOB_ACTS = <Button variant="ghost">Kill</Button>;

const STEP_FIELDS = [
  { label: "Running for", value: "6m 11s", mono: true },
  { label: "Attempt", value: "1", mono: true },
];

/** The chapter whose sheet is open says so, and stops offering to open it. */
function opened(chapters: StepChapter[], id: string, meta: string): StepChapter[] {
  return chapters.map((chapter) =>
    chapter.id === id ? { ...chapter, summary: meta, act: undefined } : chapter,
  );
}

const PATCH: DiffFile[] = [
  {
    path: "packages/settings/src/selectors.ts",
    lines: [
      { kind: "hunk", text: "@@ -14,6 +14,9 @@ import { createSelector } from 'reselect'" },
      { kind: "context", text: " import type { SettingsState } from './types'" },
      { kind: "added", text: "+import { selectColumnOrder } from './selectors/columns'" },
      { kind: "added", text: "+" },
      { kind: "context", text: " export const selectSettings = (s: RootState) => s.settings" },
      {
        kind: "removed",
        text: "-export const selectColumns = createSelector([selectSettings], (s) => s.columns)",
      },
      {
        kind: "added",
        text: "+export const selectColumns = createSelector([selectSettings], (s) => s.columns)",
      },
      { kind: "added", text: "+export const selectVisibleColumns = createSelector(" },
      { kind: "added", text: "+  [selectColumns, selectColumnOrder]," },
      {
        kind: "added",
        text: "+  (columns, order) => order.map((id) => columns[id]).filter(Boolean),",
      },
      { kind: "added", text: "+)" },
      { kind: "hunk", text: "@@ -48,7 +51,7 @@ export const selectDensity = …" },
      { kind: "context", text: " export function selectDensity(state: RootState) {" },
      { kind: "removed", text: "-  return state.settings.density ?? 'comfortable'" },
      { kind: "added", text: "+  return state.settings.density ?? DEFAULT_DENSITY" },
      { kind: "context", text: " }" },
    ],
  },
  {
    path: "packages/settings/src/reducer.ts",
    lines: [{ kind: "hunk", text: "@@ -30,12 +30,9 @@ export function settings(state, action) {" }],
  },
  {
    path: "packages/settings/test/useColumnSelectors.test.ts",
    lines: [{ kind: "hunk", text: "@@ -0,0 +1,21 @@" }],
  },
];

/** The rail's own reading: the counts, and the step that wrote each file. */
const RAIL: JobDiffFile[] = [
  { path: "packages/settings/src/selectors.ts", added: 61, removed: 4, step: "Fix" },
  { path: "packages/settings/src/reducer.ts", added: 12, removed: 27, step: "Fix" },
  {
    path: "packages/settings/test/useColumnSelectors.test.ts",
    added: 21,
    removed: 0,
    step: "Reproduction",
  },
];

/**
 * The log, open on a running Job. **The reading is held and the tail is not
 * followed**, and *Jump to now* carries the count of what arrived while it was
 * held — a stream that scrolls itself cannot be read, and one that silently
 * stops arriving cannot be trusted.
 */
export const JobDetailLogOpen: Story = {
  render: () => (
    <div className="armada-screen">
      <JobDetailWithASheet
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={RUN_RUNNING}
        runElapsed="11m 03s"
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Fix",
          fields: STEP_FIELDS,
          chapters: opened(CHAPTERS, "log", "1676 entries · open"),
        }}
        sheet={
          <ActivityLogSheet
            open
            step="Fix"
            jobId={JOB}
            total={1676}
            live
            heldAt="14:31:58"
            arrived={31}
          >
            <ActivityLog entries={WHOLE} openId="3" />
          </ActivityLogSheet>
        }
      />
    </div>
  ),
};

/**
 * The diff, open at the step that wrote. **The patch is the Job's** — Fleet
 * commits once at the end — so the header names the branch and the rail is
 * where each file names the step that produced it.
 */
export const JobDetailDiffOpen: Story = {
  render: () => (
    <div className="armada-screen">
      <JobDetailWithASheet
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={RUN_RUNNING}
        runElapsed="11m 03s"
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Fix",
          fields: STEP_FIELDS,
          chapters: opened(CHAPTERS, "produced", "3 files · open"),
        }}
        sheet={
          <JobDiffSheet
            open
            branch="fix/settings-split-selectors"
            files={RAIL}
            selected={RAIL[0]!.path}
            openedAt="Fix"
          >
            <UnifiedDiff files={PATCH} emptyNote="This drone has not changed anything yet." />
          </JobDiffSheet>
        }
      />
    </div>
  ),
};

/**
 * At `--window-floor`. **The sheet goes flush to both edges and drops its
 * radius** — 62% of 768px is 476px, and the 292px left over shows nothing
 * usable. Still a layer and not a route: nothing navigated, and `Esc` still
 * closes it. Close goes icon-only with its binding in the tooltip, and the
 * filters drop into the strip. Row heights, type and the 4px grid do not move.
 */
export const JobDetailLogOpenAtFloor: Story = {
  render: () => (
    <div className="armada-screen" style={{ width: "var(--window-floor)" }}>
      <JobDetailWithASheet
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={RUN_RUNNING}
        runElapsed="11m 03s"
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Fix",
          fields: STEP_FIELDS,
          chapters: opened(CHAPTERS, "log", "1676 entries · open"),
        }}
        sheet={
          <ActivityLogSheet
            open
            floor
            step="Fix"
            total={1676}
            live
            heldAt="14:31:58"
            arrived={31}
          >
            <ActivityLog entries={WHOLE} openId="3" />
          </ActivityLogSheet>
        }
      />
    </div>
  ),
};

/**
 * The stream at the moment the Judge refused. **The entries are the ones the
 * decision was taken on** — the submission, the Checks that passed, the refusal
 * and its record — because a log opened on an escalation that shows the Drone's
 * first ten minutes is a log nobody can decide from.
 */
const ESCALATED_ENTRIES: ActivityEntry[] = [
  {
    id: "e1",
    at: "14:44:02",
    actor: "drone",
    summary: "Bash",
    subject: "cargo nextest run -p armada-settings",
    output: ["    Starting 2034 tests across 41 binaries", "     Summary 2034 tests run: 2034 passed"].join(
      "\n",
    ),
    ran: "exit 0 · 1m 12s · in .armada/worktrees/job_2d90bb",
  },
  { id: "e2", at: "14:45:18", actor: "drone", summary: "Submitted for verification — attempt 3." },
  {
    id: "e3",
    at: "14:45:22",
    actor: "fleet",
    summary: "Checks passed —",
    subject: "build, test",
    output: ["build   cargo build --workspace --locked   exit 0", "test    cargo nextest run --workspace   exit 0"].join(
      "\n",
    ),
    ran: "2 of 2 Checks · 2m 04s",
  },
  {
    id: "e4",
    at: "14:47:09",
    actor: "fleet",
    summary: "Judge refused —",
    subject: "addresses the cause named in root_cause.md",
    named: "refused",
    output: [
      "The change widens the catch in loadSettings so the malformed row no longer throws.",
      "root_cause.md names the writer that produces the row. The reader is now tolerant of it",
      "and the writer is unchanged.",
    ].join("\n"),
    ran: "judge/record#3 · criterion 2 of 2 · attempt 3 of 3",
  },
  {
    id: "e5",
    at: "14:47:11",
    actor: "fleet",
    summary: "Escalated — attempts spent, the Drone is alive and idle.",
  },
];

/** The run at the moment the Judge refused, with the log still open over it. */
const RUN_ESCALATED: RunTreeStep[] = [
  {
    id: "root_cause",
    label: "Root cause",
    activity: "advanced",
    status: "advanced",
    elapsed: "3m 40s",
    facts: [],
  },
  {
    id: "fix",
    label: "Fix",
    activity: "failed",
    status: "refused",
    elapsed: "21m 08s",
    current: true,
    facts: [{ label: "Held", value: "attempts spent · waiting on you" }],
  },
];

/**
 * It escalated while you were reading. **The notice states itself inside the
 * sheet and does not grow the act** — Pilot keeps the accent in the Job header,
 * behind the layer, because one primary per view and this view is the log.
 * *Show me* closes the sheet, which is what `Esc` already does, and lands focus
 * on the failed step in the rail rather than on the chapter line.
 *
 * **The live mark stops with the Job.** The header goes from *live* to *ended
 * 14:47:11* and the dot goes: nothing on the screen breathes once the Job is
 * not running.
 */
export const JobDetailEscalatedLogOpen: Story = {
  render: () => (
    <div className="armada-screen">
      <JobDetailWithASheet
        heading={{ ...ESCALATED_HEADING, actions: JOB_ACTS }}
        run={RUN_ESCALATED}
        runElapsed="21m 08s"
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Fix",
          fields: [{ label: "Took", value: "21m 08s", mono: true }],
          chapters: opened(CHAPTERS, "log", "1676 entries · open"),
        }}
        sheet={
          <ActivityLogSheet
            open
            step="Fix"
            jobId={JOB}
            total={1676}
            endedAt="14:47:11"
            escalation={{
              at: "14:47:11",
              because:
                "The suite passed and the Judge refused: the diff widens the catch block rather " +
                "than addressing the cause named in root_cause.md. Three attempts spent.",
            }}
          >
            <ActivityLog entries={ESCALATED_ENTRIES} openId="e4" />
          </ActivityLogSheet>
        }
      />
    </div>
  ),
};
