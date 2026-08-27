import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { CircleDot, File, Folder, GitBranch, ShieldMinus } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import type { WorkflowRailStep } from "../../compositions/WorkflowRail/WorkflowRail";
import { ARunningJob } from "./ARunningJob";

/**
 * Four steps, two of them ungated. No primary action exists on this screen.
 *
 * **The pulse is on the rail, so the header badge is static.** One pulse per
 * screen, on the most specific mark present.
 *
 * The header verb comes from the enum→verb map. It is written into the fixture
 * because a story has no generated module to read; Bridge reads one.
 */
const meta: Meta<typeof ARunningJob> = {
  title: "Screens/A running job",
  component: ARunningJob,
};
export default meta;

type Story = StoryObj<typeof ARunningJob>;

/* The evidence row on an ungated step wants the page-with-a-check outline, and
   `file-check` has no entry in `packages/icons/icons.toml`. The row renders a
   channel short rather than reaching for an unregistered glyph. Reported. */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

const steps: WorkflowRailStep[] = [
  {
    id: "plan",
    label: "Plan the change",
    activity: "advanced",
    status: "advanced",
    evidence: { icon: NO_GLYPH_IN_REGISTRY, iconLabel: "Evidence", label: "evidence · 09:14" },
  },
  {
    id: "implement",
    label: "Implement",
    activity: "running",
    status: "running · 6m 12s",
    current: true,
    gates: [
      {
        command: "build · cargo build --workspace",
        result: "not reached",
        icon: ShieldMinus,
        iconLabel: "Not reached",
      },
      {
        command: "diff_nonempty",
        result: "not reached",
        icon: ShieldMinus,
        iconLabel: "Not reached",
      },
    ],
  },
  {
    id: "verify",
    label: "Run tests",
    activity: "not_started",
    status: "not started",
    gates: [
      {
        command: "test · cargo test --workspace",
        result: "not reached",
        icon: ShieldMinus,
        iconLabel: "Not reached",
      },
    ],
  },
  {
    id: "handoff",
    label: "Summarise",
    activity: "not_started",
    status: "not started",
    // The drawing draws no row under Summarise. The rail always draws one,
    // because a blank would read as a gate that failed to render. Reported.
    evidence: { icon: NO_GLYPH_IN_REGISTRY, iconLabel: "Evidence", label: "" },
  },
];

/**
 * Where the work is. **Every path is derived from the job id and the
 * repository, and no path carries a count** — the drawing shows "142 lines · 0
 * error" and nothing counts either, so the row names the file and stops.
 *
 * The transcript has no registered glyph, so its mark keeps its column and
 * renders empty rather than borrowing one that means something else.
 */
const WORK_ROWS = [
  {
    icon: Folder,
    iconLabel: "Worktree",
    value: "/repos/armada/.armada/worktrees/job_2d90bb",
    copyValue: "/repos/armada/.armada/worktrees/job_2d90bb",
  },
  {
    icon: GitBranch,
    iconLabel: "Branch",
    value: "fix/settings-split",
    copyValue: "fix/settings-split",
  },
  {
    icon: File,
    iconLabel: "Log",
    value: "/repos/armada/.armada/logs/job_2d90bb.jsonl",
    copyValue: "/repos/armada/.armada/logs/job_2d90bb.jsonl",
    separated: true,
  },
  {
    iconLabel: "Transcript",
    value: "/repos/armada/.armada/transcripts/",
    copyValue: "/repos/armada/.armada/transcripts/",
    meta: "named by a drone id nothing serves",
  },
];

const heading = {
  status: "running",
  statusIcon: CircleDot,
  statusLabel: "Running",
  headline: "Split the settings reducer",
  jobId: "job_2d90bb",
  fields: [
    { label: "Step", value: "2 of 4", mono: true },
    { label: "Branch", value: "fix/settings-split", mono: true, copyValue: "fix/settings-split" },
    { label: "Elapsed", value: "11m 03s", mono: true },
    { label: "Spend, estimated", value: "~$1.80", mono: true },
    { label: "Dispatched by you" },
  ],
  actions: <Button variant="destructive">Kill</Button>,
};

export const RunningJob: Story = {
  render: () => (
    <div className="armada-screen">
      <ARunningJob
        heading={heading}
        steps={steps}
        evidence={{
          icon: NO_GLYPH_IN_REGISTRY,
          iconLabel: "Evidence",
          step: "Plan the change",
          time: "09:14",
          claimed:
            "settings.rs is split into a reducer and a selector module, with no change in behaviour.",
          shownBy: "src/settings.rs → src/settings/reducer.rs, src/settings/selectors.rs",
          notClaimed:
            "Nothing about the settings UI, and no new tests — the existing suite is the only cover.",
        }}
        log={{
          brief: {
            criteria: [
              {
                text: "settings.rs is split into a reducer and a selector module.",
                source: "check",
              },
              { text: "No change in behaviour, and the existing suite still passes.", source: "judge" },
            ],
            facts: "The reducer is the only caller of `apply_defaults`. Keep the public signature.",
          },
          rows: WORK_ROWS,
          note: "The log is Fleet, the drone and Bridge in one order, keyed on this job. The transcript is named by a drone id nothing serves — the log above is the only record of it.",
          actions: (
            <Button ground="sunken" size="sm">
              Open the log
            </Button>
          ),
        }}
      />
    </div>
  ),
};

/**
 * What the rail says where a Check would be, and why it is not "no check on
 * this step".
 *
 * Those are two different sentences and **nothing on the wire tells them
 * apart**: `GET /workflows` serves `steps` as bare ids, and the served step on
 * `GET /jobs/:job_id` carries no checks either. Saying a step has no Check
 * where the truth is that Bridge cannot see one would be a guess printed as a
 * fact, so the copy names the gap instead.
 */
const UNGATED = "No operation serves this step's checks";

/**
 * The same screen with what `GET /jobs/:job_id` now carries. **The rail reads
 * served state rather than inferring it** — every row's mark is
 * `job_steps.state`, and the duration beside it is that step's `entered_at` to
 * `updated_at`. The unstarted rows carry no duration, because a span on a step
 * that has not run measures the Job's age rather than the step's.
 *
 * The header gains the branch, the whole-Job elapsed from `created_at`, and the
 * write targets. **Both kills are drawn**, because a running Job has a drone to
 * kill and a Job to end and those are two different acts.
 *
 * **Where the work is is drawn from what Bridge can know**: the worktree and
 * the log derived from the job id and the repository, the branch served. The
 * criteria are empty because Bridge's composer does not offer them, and the
 * region says so rather than leaving a labelled blank.
 */
export const AsBridgeDrawsItToday: Story = {
  render: () => (
    <div className="armada-screen">
      <ARunningJob
        heading={{
          status: "running",
          statusIcon: CircleDot,
          statusLabel: "running",
          headline: "Split the settings reducer",
          jobId: "job_2d90bb",
          fields: [
            { label: "Step", value: "2 of 4", mono: true },
            { label: "at", value: "implement", mono: true, continues: true },
            { label: "Elapsed", value: "11m 03s", mono: true },
            { label: "Branch", value: "fix/settings-split", mono: true, copyValue: "fix/settings-split" },
            { label: "Workflow", value: "bug" },
            { label: "Model", value: "sonnet", mono: true },
            { label: "Drone", value: "drn_7c21", mono: true, copyValue: "drn_7c21" },
            { label: "Writes", value: "src/settings/reducer.ts", mono: true },
          ],
          actions: (
            <>
              <Button variant="destructive">Kill drone</Button>
              <Button variant="destructive">Kill job</Button>
            </>
          ),
        }}
        steps={[
          {
            id: "plan",
            label: "plan",
            labelIsAnIdentifier: true,
            activity: "advanced",
            elapsed: "2m 14s",
            verdict: "passed",
            verdictNamed: "passed",
            ungatedLabel: UNGATED,
            evidence: { label: "" },
          },
          {
            id: "implement",
            label: "implement",
            labelIsAnIdentifier: true,
            activity: "running",
            current: true,
            elapsed: "8m 49s",
            ungatedLabel: UNGATED,
            evidence: { label: "" },
          },
          {
            id: "verify",
            label: "verify",
            labelIsAnIdentifier: true,
            activity: "not_started",
            ungatedLabel: UNGATED,
            evidence: { label: "" },
          },
          {
            id: "handoff",
            label: "handoff",
            labelIsAnIdentifier: true,
            activity: "not_started",
            ungatedLabel: UNGATED,
            evidence: { label: "" },
          },
        ]}
        log={{
          brief: {
            criteria: [],
            criteriaAbsent:
              "This job was proposed with no acceptance criteria, so nothing states what done means for it.",
            facts: "The reducer is the only caller of `apply_defaults`. Keep the public signature.",
          },
          rows: WORK_ROWS,
          note: "The worktree and the log are derived from the job id and the repository the manifest was read from. The branch is served.",
        }}
      />
    </div>
  ),
};
