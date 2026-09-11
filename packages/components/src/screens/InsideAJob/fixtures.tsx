import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import type {
  Finding,
  JobExamined,
  JobResources as Held,
  Look,
} from "@armada/protocol";
import { ESCALATION_REASON, JOB_STATUS } from "../../generated/vocabulary";
import { badgeOf } from "../badge";
import type { JobBriefProps } from "../../compositions/JobBrief/JobBrief";
import type {
  HoldsFigures,
  HoldsLine,
} from "../../compositions/JobHoldsSummary/JobHoldsSummary";
import type { JobLogReferenceRow } from "../../compositions/JobLogReference/JobLogReference";
import { PhaseCard } from "../../compositions/PhaseCard/PhaseCard";
import type { RunTreeStep } from "../../compositions/RunTree/RunTree";

/**
 * The three chapters this screen draws — `CHAPTERS`, `REPAIR_CHAPTERS`, the
 * sectioned brief behind them and `chapterAct` — live in `chapters.tsx`
 * beside this file, so this file stays under the line limit. Import them
 * from there, not from here.
 */

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
export const WORKTREE = `.armada/worktrees/${JOB}`;
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
    value: `.armada/transcripts/${JOB}/01M10B1V2A.jsonl`,
    copyValue: `.armada/transcripts/${JOB}/01M10B1V2A.jsonl`,
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
  // **The run `factsOf` builds, in its own shapes.** Four of these five had
  // drifted from what Bridge draws: the workflow is a phrase and not a labelled
  // value — `Bug workflow`, never `Workflow Bug` — the branch is the one fact
  // here that reads as itself and takes no label, the spend is labelled `Spend`,
  // and `Dispatched by you` is on the Board's row and was never on this header.
  // `Turns` is the fifth and was missing: it is the ceiling that stops a Job
  // which passed every Check, and until it was drawn the figure a person
  // decides a raise against was on no surface.
  fields: [
    { value: "Bug", suffix: " workflow", copyValue: "bug" },
    {
      value: "fix/settings-split-selectors",
      mono: true,
      copyValue: "fix/settings-split-selectors",
    },
    { label: "Elapsed", value: "11m 03s", mono: true },
    { label: "Spend", value: "~$1.80", mono: true },
    { label: "Turns", value: "18 of 40", mono: true },
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

/**
 * The first three steps, which are the same at every state below.
 *
 * **Exported for the gate sheet next door**, which runs this same Job on to
 * its delivering step. Two spellings of the two steps behind every state is
 * exactly what one fixture file exists to prevent.
 */
export const BEHIND: RunTreeStep[] = [
  {
    id: "repro",
    label: "Reproduction",
    activity: "advanced",
    status: "advanced",
    elapsed: "1m 12s",
    startedAt: "14:17:15",
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
    startedAt: "14:18:27",
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
 * The commands behind `not run`, as the phase strip's own card draws them.
 *
 * **The rich hover, and it is `PhaseCard` rather than anything new.** `Checks ·
 * not run` raises exactly one question — *which commands* — and the strip
 * already answers it four inches away on the same screen. Two drawings of that
 * card is the defect; this is the one that exists, opened from the tree.
 *
 * `said` is left to default, so the sentence about what a Check is comes from
 * `phaseSaid` and no fixture retypes it.
 */
const CHECKS_NOT_RUN = (
  <PhaseCard
    kind="checks"
    name="Checks"
    state="ahead"
    stands="not run"
    floating
    rows={[
      { label: "cargo build --workspace --locked", mono: true, result: "not run" },
      { label: "cargo nextest run --workspace", mono: true, result: "not run" },
    ]}
  />
);

/** The run while the Drone is working on Fix. */
export const RUN_RUNNING: RunTreeStep[] = [
  ...BEHIND,
  {
    id: "fix",
    label: "Fix",
    activity: "running",
    status: "running",
    elapsed: "6m 11s",
    startedAt: "14:22:07",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Produced", value: "3 files · +94 −31" },
      { label: "Checks", value: "not run", card: CHECKS_NOT_RUN },
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
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", startedAt: "14:22:07", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "awaiting_human",
    status: "waiting on you",
    elapsed: "2m 04s",
    startedAt: "14:44:20",
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
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", startedAt: "14:22:07", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "retrying",
    status: "retrying",
    elapsed: "1m 09s",
    startedAt: "14:44:20",
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
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", startedAt: "14:22:07", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "stopped",
    status: "retries spent",
    elapsed: "6m 40s",
    startedAt: "14:44:20",
    current: true,
    factsOpen: true,
    facts: [
      // The failure is named once and the repeats say they are repeats. Three
      // rows all reading `same failure` never said what the failure was, and
      // `Held` was the surface's word for a state the row's own status already
      // spells — two pieces of jargon where one plain sentence was owed.
      // Each attempt keeps its own run log, so each fact opens a different
      // artifact — which is the whole reason the tree can be pressed here
      // rather than sending a reader to one Checks list further down.
      { label: "Attempt 1", value: "test failed · exit 101", named: "failed", opens: "chk-suite@1" },
      { label: "Attempt 2", value: "the same failure again", named: "failed", opens: "chk-suite@2" },
      { label: "Attempt 3", value: "the same failure again", named: "failed", opens: "chk-suite@3" },
      { label: "Stopped", value: "3 of 3 attempts spent · waiting on you" },
    ],
  },
  ...AHEAD,
];

/** The run where the Check ended the Job. Nothing below it ever ran. */
export const RUN_FAILED: RunTreeStep[] = [
  ...BEHIND,
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", startedAt: "14:22:07", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "failed",
    status: "failed",
    elapsed: "2m 51s",
    startedAt: "14:44:20",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Checks", value: "test failed · exit 101", named: "failed" },
      { label: "Judge", value: "not reached" },
      { label: "Job", value: "completed_failed", named: "failed" },
    ],
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

/**
 * The same readings as the summary draws them, under the run.
 *
 * **Values rather than a second reading.** The figures below are what the
 * `HOLDS_*` readings above come to once a caller has formatted them — the sizes
 * are `sized()` of the same byte counts, and the process counts are the length
 * of the same lists. The summary derives nothing, so a fixture that disagreed
 * with the reading it stands for would be a disagreement the app cannot have.
 */
export const TAIL_LIVE: HoldsLine[] = [
  { at: "14:31:58", actor: "Drone", said: "thinking" },
  { at: "14:30:28", actor: "Fleet", said: "Heartbeat — quiet 48s" },
];

/**
 * Armada's last two lines before any step is running, newest first — the tail
 * that used to have a region of its own above the run.
 *
 * **A worktree cut, a preparation command that failed, and no Drone after
 * it.** The whole reason the region existed is that these belong to no step,
 * and folding it in here kept that: they are still on the Job's column, and a
 * reader still sees the failure that explains the empty run below.
 */
export const TAIL_PREPARING: HoldsLine[] = [
  { at: "09:16:47", actor: "Fleet", said: "A preparation command failed", wrong: true },
  { at: "09:14:02", actor: "Fleet", said: "Preparation began" },
];

/** A Drone working: two processes and a checkout on disk. */
export const SUMMARY_RUNNING: HoldsFigures = {
  processes: 2,
  worktree: "healthy",
  size: "1.2 GiB",
};

/** At the approval gate: no process, and right to hold none. */
export const SUMMARY_AT_THE_GATE: HoldsFigures = {
  processes: 0,
  worktree: "on disk",
  size: "1.2 GiB",
};

/** The Drone that is alive and idle, in one figure. */
export const SUMMARY_IDLE: HoldsFigures = {
  processes: 1,
  worktree: "on disk",
  size: "1.4 GiB",
};

/** A Job that is over, and the checkout it is still holding. */
export const SUMMARY_AFTER_THE_END: HoldsFigures = {
  processes: 0,
  worktree: "on disk",
  size: "1.4 GiB",
};

/**
 * The wedged Job. **The worktree is fine and the process count is the fault** —
 * the examination is what says which, exactly as it does in the full reading.
 */
export const SUMMARY_WEDGED: HoldsFigures = {
  processes: 0,
  nothingRunningIsWrong: true,
  worktree: "healthy",
  size: "92.0 MiB",
};

/**
 * The ground a job detail composes on, in a story.
 *
 * **The screen fills the height it is given rather than growing past it**, so
 * a story has to give it one: `height: 100%` against an unbounded parent
 * resolves against nothing, the two columns stop scrolling, and a trailing
 * sheet measures the panel's content instead of the screen — which is the
 * scrolling-away this bounding was for. Every story in the file is this screen,
 * so every one of them takes it.
 *
 * The height is the one `ActivityLogSheet`'s stories already stage on. A second
 * number here would be a second answer to the same question.
 */
export function OnAScreen({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        position: "relative",
        height: "var(--palette-max-height)",
        background: "var(--bg-base)",
      }}
    >
      {children}
    </div>
  );
}
