import type { LucideIcon } from "lucide-react";
import type {
  Finding,
  JobExamined,
  JobResources as Held,
  Look,
} from "@armada/protocol";
import { ESCALATION_REASON, JOB_STATUS } from "../../generated/vocabulary";
import { badgeOf } from "../badge";
import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { ActivityLog, type ActivityEntry } from "../../compositions/ActivityLog/ActivityLog";
import {
  ChangedFiles,
  changedFilesSummary,
  type ChangedFile,
} from "../../compositions/ChangedFiles/ChangedFiles";
import type { JobBriefProps } from "../../compositions/JobBrief/JobBrief";
import type { JobLogReferenceRow } from "../../compositions/JobLogReference/JobLogReference";
import type { RunTreeStep } from "../../compositions/RunTree/RunTree";
import type { StepChapter } from "../../compositions/StepStory/StepStory";

/**
 * The drawing's own Job: Bug, linear, seven steps, escalated at Regression
 * check with its retries spent. One fixture set, so every story below is the
 * same Job at a different moment — which is the claim the screen makes, and
 * six unrelated fixtures could not test it.
 *
 * **The header verb comes from the enum→verb map**, now emitted into this
 * package at `src/generated/vocabulary.ts` by the generator that writes
 * Bridge's copy. The sentence that stood here said a story had no generated
 * module to read, and that is what let `Needs you` be typed into a badge.
 */

/**
 * The transcript row's glyph has no entry in `packages/icons/icons.toml`. It
 * no longer costs the row anything: the region draws a label column now and the
 * word `Transcript` is what the glyph was standing in for. Kept because it is
 * still the honest way to say a glyph is missing. Reported.
 */
export const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

export const JOB = "job_2d90bb";
const WORKTREE = `.armada/worktrees/${JOB}`;
const DRONE = "01M10B1V2A0011VRS6RA2SKPQ7";

/**
 * Where things are. A path opens where it lives; an identifier copies.
 *
 * **The drawing's seven rows, and no glyphs.** `icon` and `iconLabel` were the
 * shape `JobLogReference` needed, where a glyph stood in for a label the region
 * had no column for. The column is drawn now, so `iconLabel` is the label and
 * the glyph is gone — the trailing mark says what the row *does*, which is the
 * only thing on the row a word would be slower than.
 */
export const WHERE: JobLogReferenceRow[] = [
  { iconLabel: "Worktree", value: WORKTREE, copyValue: WORKTREE },
  {
    iconLabel: "Branch",
    value: "fix/settings-split-selectors",
    copyValue: "fix/settings-split-selectors",
  },
  { iconLabel: "Manifest", value: "armada.yml", copyValue: "armada.yml" },
  {
    iconLabel: "Workflow",
    value: "bug",
    copyValue: "bug",
    meta: "as it was at 14:20",
  },
  {
    iconLabel: "Job log",
    value: `.armada/logs/${JOB}.jsonl`,
    copyValue: `.armada/logs/${JOB}.jsonl`,
    separated: true,
  },
  {
    iconLabel: "Transcript",
    value: ".armada/transcripts/01M10B1V2A.jsonl",
    copyValue: ".armada/transcripts/01M10B1V2A.jsonl",
  },
  { iconLabel: "Drone", value: DRONE, copyValue: DRONE },
];

/**
 * The brief the panel opens with — one line, on the panel's own surface.
 *
 * **`only: "facts"` and no label**, which is what the surface passes: the
 * region is called `Brief` and the sentence follows it, so a second heading
 * over one line is the sub-heading this screen removed. The criteria are what
 * the Judge stage of the phase strip opens to, with each one's verdict beside
 * it, which is one place rather than two.
 */
export const BRIEF: JobBriefProps = {
  facts:
    "The selectors cannot be tested without constructing the whole store, which makes every " +
    "settings test an integration test.",
  criteria: [],
  only: "facts",
  factsLabel: null,
};

export const HEADING = {
  ...badgeOf("running", JOB_STATUS),
  headline: "Split the settings reducer so the selectors can be tested alone",
  jobId: JOB,
  fields: [
    { label: "Workflow", value: "Bug" },
    {
      label: "Branch",
      value: "fix/settings-split-selectors",
      mono: true,
      copyValue: "fix/settings-split-selectors",
    },
    { label: "Elapsed", value: "11m 03s", mono: true },
    { label: "Spend, estimated", value: "~$1.80", mono: true },
    { label: "Dispatched by you" },
  ],
};

/**
 * An escalated Job's header, which reads by the reason it escalated.
 * **`escalated` carries `verb: null, icon: null` deliberately** — the
 * vocabulary refusing to render the status so the reason renders instead,
 * because nobody says a Job escalated at step 3. So the reason is the argument
 * and the badge is `ESCALATION_REASON`'s answer: `gate_failure` reads *Stopped
 * at the gate*, `evidence_suspect` reads *Evidence disputed*, and neither
 * string exists here. It replaces `Needs you`, which is `who_is_acting ==
 * Person` — true of three other statuses, so a correct Board filter and a badge
 * that could not name the status it sat on. #294.
 */
export const escalatedHeading = (reason: string) => ({
  ...HEADING,
  ...badgeOf(reason, ESCALATION_REASON),
});

/** The Judge refused with the retries spent: the gate is what stopped it. */
export const ESCALATED_HEADING = escalatedHeading("gate_failure");

export const FAILED_HEADING = { ...HEADING, ...badgeOf("completed_failed", JOB_STATUS) };

/**
 * `Waiting on you` stood here beside the wire spelling `awaiting_review`, so
 * the badge asked for a hue token that does not exist and said a thing three
 * other statuses say too. The badge names the status; the decision block below
 * it is where the screen asks something of you. Same defect as #294.
 */
export const WAITING_HEADING = { ...HEADING, ...badgeOf("awaiting_review", JOB_STATUS) };

/** The first three steps, which are the same at every state below. */
const BEHIND: RunTreeStep[] = [
  {
    id: "repro",
    label: "Reproduction",
    activity: "advanced",
    status: "advanced",
    elapsed: "1m 12s",
    facts: [
      {
        label: "Produced",
        paths: [{ directory: "packages/settings/test/", basename: "useColumnSelectors.test.ts" }],
      },
      { label: "Cleared", value: "test", named: "passed" },
    ],
  },
  {
    id: "root_cause",
    label: "Root cause",
    activity: "advanced",
    status: "advanced",
    elapsed: "3m 40s",
    facts: [
      { label: "Attempt 1", value: "refused", named: "refused" },
      { label: "Attempt 2", value: "advanced", named: "advanced" },
      {
        label: "Produced",
        paths: [{ directory: `.armada/artifacts/${JOB}/`, basename: "root_cause.md" }],
      },
    ],
  },
];

/** The three steps ahead, which nothing has reached at any state below. */
const AHEAD: RunTreeStep[] = [
  {
    id: "consumers",
    label: "Check the consumers still compile",
    activity: "not_started",
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing.",
  },
  {
    id: "land",
    label: "Land",
    activity: "not_started",
    locked: true,
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing.",
  },
];

/**
 * The run before anything has started. **Every step `not_started`**, which is
 * the state the Job that prompted #437 sat in for six minutes while Fleet cut
 * its worktree and installed its dependencies — and the reason those lines
 * belong to no step.
 */
export const RUN_NOT_STARTED: RunTreeStep[] = [
  {
    id: "repro",
    label: "Reproduction",
    activity: "not_started",
    current: true,
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing.",
  },
  {
    id: "root_cause",
    label: "Root cause",
    activity: "not_started",
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing.",
  },
  {
    id: "fix",
    label: "Fix",
    activity: "not_started",
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing.",
  },
];

/**
 * What Fleet has done to the Job itself, before any step is running.
 *
 * **Every one of these names Fleet**, and every one opens to what it carried.
 * A grey line of prose in a column of openable entries is a second-class
 * citizen in a stream whose whole claim is that it has one grammar — which is
 * what the log-file-as-text rendering would have made these.
 */
export const FLEET_PREPARING: ActivityEntry[] = [
  {
    id: "f1",
    at: "09:14:02",
    actor: "fleet",
    summary: "Worktree cut",
    output: "branch  armada/job_2d90bb\nat      .armada/worktrees/job_2d90bb",
  },
  {
    id: "f2",
    at: "09:14:02",
    actor: "fleet",
    summary: "Preparation began",
    output: "commands  3",
  },
  {
    id: "f3",
    at: "09:16:47",
    actor: "fleet",
    summary: "A preparation command failed",
    named: "failed",
    output: "command   pnpm install --frozen-lockfile\nexit      1",
  },
];

/** The run while the Drone is working on Fix. */
export const RUN_RUNNING: RunTreeStep[] = [
  ...BEHIND,
  {
    id: "fix",
    label: "Fix",
    activity: "running",
    status: "running",
    elapsed: "6m 11s",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Produced", value: "3 files · +94 −31" },
      { label: "Checks", value: "not run" },
      { label: "Judge", value: "2 criteria" },
    ],
  },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "not_started",
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing.",
  },
  ...AHEAD,
];

/** The run once Regression check has cleared everything mechanical. */
export const RUN_WAITING: RunTreeStep[] = [
  ...BEHIND,
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "awaiting_human",
    status: "waiting on you",
    elapsed: "2m 04s",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Checks", value: "2 of 2 passed", named: "passed" },
      { label: "Judge", value: "2 of 2 met", named: "passed" },
      { label: "Waiting", value: "on you · 2m 04s" },
    ],
  },
  ...AHEAD,
];

/** The run while a failed Check is being repaired by the Drone that caused it. */
export const RUN_REPAIRING: RunTreeStep[] = [
  ...BEHIND,
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "retrying",
    status: "retrying",
    elapsed: "1m 09s",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Attempt 1", value: "test failed · exit 101", named: "failed" },
      { label: "Attempt 2", value: "running" },
      { label: "Checks", value: "1 of 2 failed", named: "failed" },
    ],
  },
  ...AHEAD,
];

/** The run once three attempts at the same failure are spent. */
export const RUN_STOPPED: RunTreeStep[] = [
  ...BEHIND,
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "stopped",
    status: "retries spent",
    elapsed: "6m 40s",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Attempt 1", value: "same failure", named: "failed" },
      { label: "Attempt 2", value: "same failure", named: "failed" },
      { label: "Attempt 3", value: "same failure", named: "failed" },
      { label: "Held", value: "retries spent · waiting on you" },
    ],
  },
  ...AHEAD,
];

/** The run where the Check ended the Job. Nothing below it ever ran. */
export const RUN_FAILED: RunTreeStep[] = [
  ...BEHIND,
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "failed",
    status: "failed",
    elapsed: "2m 51s",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Checks", value: "test failed · exit 101", named: "failed" },
      { label: "Judge", value: "not reached" },
      { label: "Job", value: "completed_failed", named: "failed" },
    ],
  },
];

const PREVIEW: ActivityEntry[] = [
  { id: "1", at: "14:22:07", actor: "armada", summary: "Go on to Implement." },
  {
    id: "2",
    at: "14:26:31",
    actor: "drone",
    summary: "Edit",
    subject: "packages/settings/src/selectors.ts",
    output: [
      "@@ -14,6 +14,9 @@",
      "+import { selectColumnOrder } from './selectors/columns'",
      "+",
      " export const selectSettings = (s: RootState) => s.settings",
    ].join("\n"),
    ran: `+3 −0 · in ${WORKTREE}`,
  },
  {
    id: "3",
    at: "14:29:40",
    actor: "drone",
    summary: "Bash",
    subject: "cargo build --workspace --locked",
    output: [
      "$ cargo build --workspace --locked",
      "   Compiling armada-settings v0.1.0 (packages/settings)",
      "   Compiling armada-fleet v0.1.0 (crates/fleet)",
      "    Finished `dev` profile [unoptimized] in 47.61s",
    ].join("\n"),
    ran: `exit 0 · 47.61s · in ${WORKTREE}`,
  },
  {
    id: "4",
    at: "14:30:28",
    actor: "fleet",
    summary: "Heartbeat — the Drone has been quiet for 48 seconds",
  },
  { id: "5", at: "14:31:58", actor: "drone", summary: "thinking" },
];

/** Every entry the step carried, which is what the log sheet draws. */
export const WHOLE: ActivityEntry[] = [
  PREVIEW[0]!,
  {
    id: "1b",
    at: "14:22:44",
    actor: "drone",
    summary:
      "Splitting the selector block into its own module so the tests can import it without the store.",
  },
  {
    id: "1c",
    at: "14:23:11",
    actor: "drone",
    summary: "Read",
    subject: "packages/settings/src/reducer.ts",
    output: [
      "import { createSlice } from '@reduxjs/toolkit'",
      "import type { SettingsState } from './types'",
      "",
      "const initialState: SettingsState = { columns: {}, density: 'comfortable' }",
    ].join("\n"),
    ran: `214 lines · in ${WORKTREE}`,
  },
  ...PREVIEW.slice(1),
];

/**
 * The three files the step produced, each with its own count — the drawing
 * lists them that way, and the chapter's header is the same reading summed.
 *
 * **Nothing on the wire fills `added` and `deleted`.** The seam carries the
 * names and never the bytes, by its own stated rule. Drawn here because the
 * drawing draws it; the surface cannot reach it yet, and that is reported
 * rather than papered over with a shorter fixture.
 */
export const PRODUCED_FILES: ChangedFile[] = [
  { path: "packages/settings/src/selectors.ts", change: "modified", added: 61, deleted: 4 },
  { path: "packages/settings/src/reducer.ts", change: "modified", added: 12, deleted: 27 },
  { path: "packages/settings/src/index.ts", change: "added", added: 21 },
];

const PRODUCED = (
  <ChangedFiles emptyNote="This drone has not changed anything yet." files={PRODUCED_FILES} />
);

/**
 * The story, in the order it happened. **Same three chapters at every state** —
 * what changes is which one is the reason you are here.
 */
/**
 * The affordance on a chapter whose content has no end.
 *
 * **It names its own destination and it is on the header line.** The log and
 * the diff open as a trailing sheet rather than in place — 1676 entries and a
 * whole patch are not longer versions of a preview — so there is no body for
 * the control to sit under, and the word is never *more*. Its binding is drawn
 * inline, which is Journey 4's stated departure from the contract: a missing
 * one is then visible, which is how the `open_log` gap was found.
 */
export function chapterAct(label: string, binding: string) {
  return (
    <Button variant="ghost" size="sm">
      {label}
      <Kbd>{binding}</Kbd>
    </Button>
  );
}

export const CHAPTERS: StepChapter[] = [
  {
    id: "instructions",
    ordinal: 1,
    title: "Drone instructions",
    summary: "14:22:07 · 2 criteria and what it was given",
    preview:
      "Move the selector block into its own module so the tests can import it without constructing " +
      "the store. Do not change reducer behaviour.",
  },
  {
    id: "log",
    ordinal: 2,
    title: "Activity log",
    // `live` is the running dot, not the word. A count says how many entries
    // there are and only the dot says they are still arriving.
    live: true,
    summary: "47 entries · every line opens",
    preview: <ActivityLog entries={PREVIEW} />,
    act: chapterAct("Open the log", "Enter"),
  },
  {
    id: "produced",
    ordinal: 3,
    title: "Produced",
    // Built from the list rather than typed beside it, so the header and the
    // rows cannot disagree about what the reading found.
    summary: changedFilesSummary(PRODUCED_FILES, true),
    preview: PRODUCED,
    act: chapterAct("Open the diff", "f"),
  },
];

/** The stream on the step whose Check failed, with Fleet's own hand-back in it. */
export const REPAIR_CHAPTERS: StepChapter[] = [
  {
    id: "instructions",
    ordinal: 1,
    title: "Drone instructions",
    summary: "14:44:20",
    preview: "Run the regression suite and fix anything it turns up.",
  },
  {
    id: "log",
    ordinal: 2,
    title: "Activity log",
    summary: "88 entries · ended 14:47:11",
    act: chapterAct("Open the log", "Enter"),
    preview: (
      <ActivityLog
        entries={[
          {
            id: "r1",
            at: "14:46:02",
            actor: "drone",
            summary: "Bash",
            subject: "cargo nextest run --workspace",
          },
          {
            id: "r2",
            at: "14:47:09",
            actor: "fleet",
            summary: "Check failed — 3 of 2034 tests. Handed back to the Drone, attempt 2 of 3.",
            subject: "test",
            named: "failed",
            output: [
              "FAIL settings::selectors::visible_manifests_memoises",
              "  expected the same reference on repeat calls, got a new object",
              "and 2 more",
            ].join("\n"),
            ran: `exit 101 · 1m 22s · in ${WORKTREE}`,
          },
        ]}
        openId="r2"
      />
    ),
  },
  {
    id: "produced",
    ordinal: 3,
    title: "Produced",
    summary: "4 files · being repaired",
    preview:
      "The work is on fix/settings-split-selectors and the Drone is editing it now. Nothing was " +
      "thrown away and nothing was rolled back.",
    act: chapterAct("Open the diff", "f"),
  },
];

/**
 * What the Job holds on this machine, at each moment the run below is drawn at.
 *
 * **The same Job, so the panel can be read against the run beside it.** The
 * worktree path and the branch are `WHERE`'s, because that region sits four
 * inches below this one and two spellings of one branch on one screen is the
 * defect this fixture file exists to prevent.
 */
function holds(over: Partial<Held> = {}): Held {
  return {
    job_id: JOB,
    read_at: "2026-09-04T09:16:52.402Z",
    held: "running",
    processes: [],
    worktree: {
      path: WORKTREE,
      branch: "fix/settings-split-selectors",
      bytes: 1_288_490_188,
    },
    ...over,
  };
}

function examined(found: Finding, looks: Look[], reading: Held): JobExamined {
  return { job_id: JOB, looked_at: "2026-09-04T09:16:52.402Z", found, looks, resources: reading };
}

/**
 * A Drone working, and the two processes that says. **The one Fleet wrote down
 * leads**, and the build it started is descended from it — the same
 * `cargo build --workspace --locked` the step's activity log is showing, which
 * is the whole reason the reading is a list and not a pid.
 *
 * **Named by its command and never by its vendor.** A process is `node` or
 * `cargo` here because that is what `ps` answers; the gate's rule against a
 * vendor literal outside `crates/adapters` caught the first spelling of this.
 */
export const HOLDS_RUNNING: Held = holds({
  processes: [
    {
      pid: 41233,
      command: "node",
      cpu_percent: 8.2,
      memory_bytes: 402_653_184,
      running_for: "06:11",
      recorded: true,
    },
    {
      pid: 41287,
      command: "cargo",
      cpu_percent: 61.4,
      memory_bytes: 268_435_456,
      running_for: "00:12",
      recorded: false,
    },
  ],
  wrote_last_at: "2026-09-04T09:16:44.100Z",
});

/**
 * The healthy answer, said once. **Three looks and not five**: `writing` and
 * `silence` can never report a fault, so an examination that included them
 * could not come back `working` — which is the component's own rule, and the
 * reason its `WorkingAndSaidSo` story asks the same three.
 */
export const EXAMINED_WORKING: JobExamined = examined(
  "working",
  [
    {
      asked: "process",
      found: "working",
      said: "the process Fleet recorded is running",
      fields: [{ name: "pid", value: "41233" }],
    },
    { asked: "worktree", found: "working", said: "the worktree is on disk" },
    { asked: "span", found: "working", said: "waiting for the step to finish" },
  ],
  HOLDS_RUNNING,
);

/**
 * A Job at its approval gate. **Holding nothing, and right to** — which is why
 * the absence is drawn quiet here and loud on `HOLDS_WEDGED` below, from the
 * same empty list. Nobody has pressed, because nothing looks wrong.
 */
export const HOLDS_AT_THE_GATE: Held = holds({
  held: "none",
  processes: [],
  worktree: { path: WORKTREE, branch: "fix/settings-split-selectors", bytes: 1_310_720_000 },
  wrote_last_at: "2026-09-04T09:14:48.000Z",
});

/**
 * The claim `Drone alive, idle` in the step's header, substantiated.
 *
 * **This is the figure that header field cannot carry.** `alive, idle` is a
 * summary of a process at 0.1% of a core that has been up for twenty-one
 * minutes, and the summary is worth exactly as much as whatever produced it —
 * so the reading it came from is on the screen beside it.
 */
export const HOLDS_IDLE: Held = holds({
  processes: [
    {
      pid: 41233,
      command: "node",
      cpu_percent: 0.1,
      memory_bytes: 536_870_912,
      running_for: "21:40",
      recorded: true,
    },
  ],
  worktree: { path: WORKTREE, branch: "fix/settings-split-selectors", bytes: 1_476_395_008 },
  wrote_last_at: "2026-09-04T09:10:12.000Z",
});

/**
 * A Job that is over, and the checkout it left behind.
 *
 * **The size is the reason this is drawn on a dead Job at all.** Nothing here
 * needs watching and the disk does: seventy-four worktrees once took 220 GB and
 * three agents died at zero bytes free, and every one of those Jobs was over.
 */
export const HOLDS_AFTER_THE_END: Held = holds({
  held: "none",
  processes: [],
  worktree: { path: WORKTREE, branch: "fix/settings-split-selectors", bytes: 1_476_395_008 },
  wrote_last_at: "2026-09-04T09:11:56.000Z",
});

/**
 * The state the whole panel was built for. **A Job that reads `running` and
 * holds no process**, with the run below it showing every step `not_started`
 * and Fleet's log stopped on a failed preparation command two minutes ago.
 *
 * Nothing else on this screen says the Job is dead. The badge says running, the
 * tree says not started — both true, and together they are what a wedged Job
 * looked like for six minutes on 4 Sep 2026 while somebody read them.
 */
export const HOLDS_WEDGED: Held = holds({
  held: "none",
  processes: [],
  worktree: { path: WORKTREE, branch: "fix/settings-split-selectors", bytes: 96_468_992 },
  wrote_last_at: "2026-09-04T09:16:47.000Z",
});

/** What asking found on the wedged Job. The headline is the finding. */
export const EXAMINED_WEDGED: JobExamined = examined(
  "not_working",
  [
    {
      asked: "process",
      found: "not_working",
      said: "this Job is running and Fleet recorded no process for it",
      fields: [{ name: "processes", value: "0" }],
    },
    {
      asked: "worktree",
      found: "working",
      said: "the worktree is on disk",
    },
    {
      asked: "writing",
      found: "cannot_tell",
      said: "nothing has been written to this Job's log lately, which settles nothing on its own",
      fields: [{ name: "seconds_ago", value: "165" }],
    },
  ],
  HOLDS_WEDGED,
);
