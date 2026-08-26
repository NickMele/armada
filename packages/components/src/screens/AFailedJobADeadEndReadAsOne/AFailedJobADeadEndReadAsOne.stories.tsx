import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { Folder, GitBranch, ShieldCheck, ShieldX, X } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { JobDetailHeaderActions } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { JobLogReference } from "../../compositions/JobLogReference/JobLogReference";
import { WorkflowRail, type WorkflowRailStep } from "../../compositions/WorkflowRail/WorkflowRail";

/**
 * Journey · Read a failed Job. The screen states four things in order — what
 * failed, that the job is over, where the branch is, and where the log is.
 *
 * The header is `Job detail header actions`, the same component the running job
 * renders. The drawing tags it only there, but the block is the same one — a
 * badge, a title, a job id and a run of facts — and a second hand-built copy
 * could only drift from it. What changes with the state is the field run and
 * the trailing action: a failed job reports where it stopped and carries no
 * action here at all, because the acts on a dead end are about its branch and
 * its log and sit beside those below.
 */
const meta: Meta = {
  title: "Screens/A failed job — a dead end, read as one",
};
export default meta;

type Story = StoryObj;

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

export const FailedJob: Story = {
  render: () => (
    <div className="armada-screen">
      <div className="armada-screen__detail">
        <JobDetailHeaderActions
          status="completed-failed"
          statusIcon={X}
          statusLabel="Failed"
          headline="Cache the manifest read"
          jobId="job_91ab"
          fields={[
            // A step name is a label, so it stays sans beside its mono
            // siblings, and the two halves are one fact joined by a comma.
            { label: "Stopped at", value: "Run tests" },
            { label: "step", value: "3 of 4", mono: true, continues: true },
            { label: "Ran", value: "22m 41s", mono: true },
            { label: "Spend, estimated", value: "~$2.10", mono: true },
            { label: "Dispatched by you" },
          ]}
        />

        <div className="armada-screen__sunken">
          <span className="armada-screen__eyebrow">Why this stopped</span>
          <p className="armada-screen__why">
            The test check exited 1 at Run tests, on 2 assertions in{" "}
            <span className="armada-screen__mono">core/manifest</span>. The job is over.
            Nothing runs from here without you.
          </p>
        </div>

        <div className="armada-screen__split" data-wide>
          <div className="armada-screen__col">
            <span className="armada-screen__eyebrow">What ran</span>
            <WorkflowRail steps={steps} />
          </div>

          <div className="armada-screen__col" data-loose>
            <div className="armada-screen__col">
              <div className="armada-screen__head-row">
                <span className="armada-screen__eyebrow">Check output</span>
                <span className="armada-screen__tag">exit 1 · 4.2s · tail 12 lines</span>
              </div>
              <pre className="armada-screen__output">{tail}</pre>
            </div>

            <div className="armada-screen__col">
              <span className="armada-screen__eyebrow">Where the work is</span>
              <JobLogReference
                rows={[
                  {
                    icon: GitBranch,
                    iconLabel: "Branch",
                    value: "feat/manifest-cache",
                    copyValue: "feat/manifest-cache",
                    meta: "2 files +48 −11",
                  },
                  // `folder` means "workspace" in the registry. A worktree is
                  // not a workspace, and the registry has no row for one.
                  // Reported.
                  {
                    icon: Folder,
                    iconLabel: "Worktree",
                    value: "~/.armada/worktrees/job_91ab",
                  },
                  {
                    icon: NO_GLYPH_IN_REGISTRY,
                    iconLabel: "Log",
                    value: ".armada/logs/job_91ab.jsonl",
                    copyValue: ".armada/logs/job_91ab.jsonl",
                    meta: "318 lines · 4 error",
                    separated: true,
                  },
                ]}
              >
                The worktree and the branch are left in place. Armada will not touch either.
                The log holds Fleet, the drone and Bridge in one order, keyed on this job.
              </JobLogReference>
              <div className="armada-screen__actions">
                <Button>Open the log</Button>
                <Button>Open the worktree</Button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  ),
};
