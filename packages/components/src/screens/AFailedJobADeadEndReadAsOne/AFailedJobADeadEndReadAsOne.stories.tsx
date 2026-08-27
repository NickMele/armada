import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { CircleCheck, CircleX, File, Folder, GitBranch, OctagonAlert, Power, ShieldCheck, ShieldMinus, ShieldX, X } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import type { WorkflowRailStep } from "../../compositions/WorkflowRail/WorkflowRail";
import { AFailedJobADeadEndReadAsOne } from "./AFailedJobADeadEndReadAsOne";

/**
 * Journey · Read a failed Job. The screen states four things in order — what
 * failed, that the job is over, where the branch is, and where the log is.
 *
 * The header is `Job detail header actions`, the same component the running job
 * renders: a badge, a title, a job id and a run of facts. What changes with the
 * state is the field run and the trailing action — a terminal job carries none,
 * because the acts on a dead end are about its branch and its log and sit
 * beside those below.
 */
const meta: Meta<typeof AFailedJobADeadEndReadAsOne> = {
  title: "Screens/A failed job — a dead end, read as one",
  component: AFailedJobADeadEndReadAsOne,
};
export default meta;

type Story = StoryObj<typeof AFailedJobADeadEndReadAsOne>;

/* `file` has no entry in `packages/icons/icons.toml`, so the log row renders a
   channel short rather than reaching for an unregistered glyph. Reported. */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

const steps: WorkflowRailStep[] = [
  {
    id: "plan",
    label: "Plan the change",
    activity: "advanced",
    status: "advanced",
    // The drawing draws no row under Plan the change here. The rail always
    // draws one. Reported.
    evidence: { icon: NO_GLYPH_IN_REGISTRY, iconLabel: "Evidence", label: "" },
  },
  {
    id: "implement",
    label: "Implement",
    activity: "advanced",
    status: "advanced",
    gates: [
      {
        command: "build · cargo build --workspace",
        result: "exit 0",
        icon: ShieldCheck,
        iconLabel: "Passed",
      },
      // The drawing draws `shield-minus` on this row, whose registry entry
      // means "not reached", beside the result "passed". A glyph is never
      // written by hand against the registry, so the row takes `shield-check`.
      // Reported as a slip in the drawing.
      {
        command: "diff_nonempty",
        result: "passed",
        icon: ShieldCheck,
        iconLabel: "Passed",
      },
    ],
  },
  {
    id: "verify",
    label: "Run tests",
    activity: "failed",
    status: "failed a check",
    gates: [
      {
        command: "test · cargo test --workspace",
        result: "exit 1",
        icon: ShieldX,
        iconLabel: "Failed",
      },
    ],
  },
  {
    id: "handoff",
    label: "Summarise",
    activity: "not_started",
    status: "not started",
    evidence: { icon: NO_GLYPH_IN_REGISTRY, iconLabel: "Evidence", label: "" },
  },
];

const tail = [
  "running 84 tests",
  "test manifest::cache::reads_once ... FAILED",
  "test manifest::cache::invalidates_on_write ... FAILED",
  "",
  "failures:",
  "",
  "---- manifest::cache::reads_once stdout ----",
  "assertion `left == right` failed",
  "  left: 2",
  " right: 1",
  "   at core/manifest/src/cache.rs:214",
  "",
  "test result: FAILED. 82 passed; 2 failed",
].join("\n");

const heading = {
  status: "completed-failed",
  statusIcon: X,
  statusLabel: "Failed",
  headline: "Cache the manifest read",
  jobId: "job_91ab",
  fields: [
    // A step name is a label, so it stays sans beside its mono siblings, and
    // the two halves are one fact joined by a comma.
    { label: "Stopped at", value: "Run tests" },
    { label: "step", value: "3 of 4", mono: true, continues: true },
    { label: "Ran", value: "22m 41s", mono: true },
    { label: "Spend, estimated", value: "~$2.10", mono: true },
    { label: "Dispatched by you" },
  ],
};

export const FailedJob: Story = {
  render: () => (
    <div className="armada-screen">
      <AFailedJobADeadEndReadAsOne
        heading={heading}
        why={
          <>
            The test check exited 1 at Run tests, on 2 assertions in{" "}
            <span className="armada-screen__mono">core/manifest</span>. The job is over. Nothing
            runs from here without you.
          </>
        }
        steps={steps}
        output={{ tail, meta: "exit 1 · 4.2s · tail 12 lines" }}
        work={{
          brief: {
            criteria: [
              { text: "The manifest is read once per dispatch, not once per step.", source: "check" },
              { text: "A changed armada.yml is picked up without restarting Fleet.", source: "judge" },
            ],
            facts: "`config::manifest` is the only reader. The cache key is the absolute path.",
          },
          rows: [
            {
              icon: GitBranch,
              iconLabel: "Branch",
              value: "feat/manifest-cache",
              copyValue: "feat/manifest-cache",
              meta: "2 files +48 −11",
            },
            // `folder` means "workspace" in the registry. A worktree is not a
            // workspace, and the registry has no row for one. Reported.
            {
              icon: Folder,
              iconLabel: "Worktree",
              value: "/repos/armada/.armada/worktrees/job_91ab",
              copyValue: "/repos/armada/.armada/worktrees/job_91ab",
            },
            {
              icon: File,
              iconLabel: "Log",
              value: "/repos/armada/.armada/logs/job_91ab.jsonl",
              copyValue: "/repos/armada/.armada/logs/job_91ab.jsonl",
              separated: true,
            },
            // No registered glyph means a transcript, so the mark keeps its
            // column and renders empty rather than borrowing one. Reported.
            {
              iconLabel: "Transcript",
              value: "/repos/armada/.armada/transcripts/",
              copyValue: "/repos/armada/.armada/transcripts/",
              meta: "named by a drone id nothing serves",
            },
          ],
          note: "The worktree and the branch are left in place. Armada will not touch either. The log holds Fleet, the drone and Bridge in one order, keyed on this job.",
          actions: (
            <>
              <Button>Open the log</Button>
              <Button>Open the worktree</Button>
            </>
          ),
        }}
      />
    </div>
  ),
};

/**
 * A Job that stopped and asked. **Not terminal** — the one act on it is a
 * redispatch, and it is about the Job rather than its branch, so it goes in
 * the header where `Kill` goes on a running Job.
 *
 * The verb and the glyph are `escalation_reason.stalled`'s, which is what
 * Bridge reads from the generated vocabulary rather than writing here.
 */
export const StoppedAndAsked: Story = {
  render: () => (
    <div className="armada-screen">
      <AFailedJobADeadEndReadAsOne
        heading={{
          status: "escalated",
          statusIcon: OctagonAlert,
          statusLabel: "stalled",
          headline: "Cache the manifest read",
          jobId: "job_91ab",
          fields: [
            { label: "Stopped at", value: "verify" },
            { label: "step", value: "3 of 4", mono: true, continues: true },
            { label: "Model", value: "sonnet", mono: true },
          ],
          actions: <Button>Redispatch</Button>,
        }}
        why="The job stalled. Nothing runs from here without you."
        steps={steps.map((step) => ({
          id: step.id,
          label: step.id,
          labelIsAnIdentifier: true,
          activity: step.activity,
          ungatedLabel: "Fleet serves no check result for this step",
          evidence: { label: "" },
        }))}
      />
    </div>
  ),
};

/**
 * **A Judge refused a criterion, and the whole screen says so.** The step ran,
 * its Check passed, and the work is not what was asked for — the citation
 * beneath the step is the only thing on the screen that says which criterion
 * and why, and it is what a person triages on.
 *
 * **The band at the top reads "failed a check", and that is wrong here.** A
 * refusal escalates on `gate_failure` — `crates/fleet/src/gate.rs` picks it
 * deliberately — and `enum-verbs.toml` gives that trigger a Check's verb and a
 * Check's `shield-x` glyph, from a time when only a Check could fire it. A
 * status label is never written by hand, so the registry's word renders and the
 * disagreement is a finding rather than something worked around here. The rail
 * is what carries the truth in the meantime.
 */
export const AJudgeRefusedACriterion: Story = {
  render: () => (
    <div className="armada-screen">
      <AFailedJobADeadEndReadAsOne
        heading={{
          status: "escalated",
          statusIcon: ShieldX,
          statusLabel: "failed a check",
          headline: "Sign a revoked device out on refresh failure",
          jobId: "job_2d90bb",
          fields: [
            { label: "Stopped at", value: "Implement" },
            { label: "step", value: "2 of 4", mono: true, continues: true },
            { label: "Elapsed", value: "11m 03s", mono: true },
            { label: "Model", value: "sonnet", mono: true },
          ],
          actions: <Button>Redispatch as a new job</Button>,
        }}
        why="failed a check · owes c2"
        ranLabel="What ran"
        steps={[
          { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
          {
            id: "implement",
            label: "Implement",
            activity: "stopped",
            status: "stopped",
            current: true,
            gates: [
              {
                command: "build · cargo build --workspace",
                result: "passed",
                icon: ShieldCheck,
                iconLabel: "Passed",
                outputPath: ".armada/jobs/job_2d90bb/checks/build.log",
              },
            ],
            verdicts: [
              {
                ordinal: 1,
                criterionId: "c1",
                text: "Expired tokens refresh once rather than per request.",
                named: "met",
                verdict: "no objection",
                icon: CircleCheck,
              },
              {
                ordinal: 2,
                criterionId: "c2",
                text: "A failed refresh signs the session out.",
                named: "not_met",
                verdict: "refused",
                icon: CircleX,
                expected:
                  "A 401 from the refresh endpoint clears the session and returns the caller to sign-in.",
                produced:
                  "The refresh error is swallowed in `session.ts:212` and the stale token is retried on the next request.",
                consequence:
                  "A revoked device keeps a working-looking session until the next full reload, so signing a device out does not sign it out.",
              },
            ],
          },
          { id: "verify", label: "Run tests", activity: "not_started", status: "not started" },
          { id: "handoff", label: "Summarise", activity: "not_started", status: "not started" },
        ]}
        outputAbsent="Each check names its output file on its own row. Nothing serves the contents."
      />
    </div>
  ),
};

/**
 * **Killed while the step was running.** `job_steps.state` still says `running`
 * on the wire and that is correct — `job-statuses.toml` freezes the step
 * machine at `killed` and declares no step state for it, because the Job being
 * terminal is what says everything is over. The rail draws the frozen step
 * rather than the live one: `power`, **no hue**, and a duration that has
 * stopped.
 *
 * The exclusion is the point. A killed step must not read as a system failure,
 * so it takes neither the failed row's hue nor its surface — and it is not
 * `stopped` either, which would say a redirect or a restart resumes it.
 *
 * **"Why this stopped" is the step, not a stored reason.** `killed` stores
 * none, so without the step this region would say only that something ended.
 * The step's name, the Check that did not pass and the file it wrote are all
 * served, and Bridge names them in that order.
 */
export const KilledWhileTheStepWasRunning: Story = {
  render: () => (
    <div className="armada-screen">
      <AFailedJobADeadEndReadAsOne
        heading={{
          status: "killed",
          statusIcon: Power,
          statusLabel: "killed",
          headline: "Cache the manifest read",
          jobId: "job_91ab",
          fields: [
            { label: "Step", value: "3 of 4", mono: true },
            { label: "at", value: "verify", mono: true, continues: true },
            { label: "Elapsed", value: "22m 41s", mono: true },
            { label: "Model", value: "sonnet", mono: true },
          ],
        }}
        why={<>stopped at Run tests</>}
        steps={[
          { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced", elapsed: "2m 14s" },
          { id: "implement", label: "Implement", activity: "advanced", status: "advanced", elapsed: "6m 48s" },
          {
            id: "verify",
            label: "Run tests",
            // `running` on the wire, `killed` on the rail. The Job's status is
            // read, not a state Fleet does not have.
            activity: "killed",
            status: "killed",
            current: true,
            elapsed: "4m 09s",
            gates: [
              {
                command: "test · cargo test --workspace",
                result: "not reached",
                icon: ShieldMinus,
                iconLabel: "Not reached",
              },
            ],
          },
          { id: "handoff", label: "Summarise", activity: "not_started", status: "not_started" },
        ]}
        outputAbsent="Each check names its output file on its own row. Nothing serves the contents."
        workAbsent="Nothing serves this Job's paths, its branch or its brief."
      />
    </div>
  ),
};
