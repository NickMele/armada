import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { Folder, GitBranch, ShieldCheck, ShieldX, X } from "lucide-react";
import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { JobLogReference } from "../../compositions/JobLogReference/JobLogReference";
import { WorkflowRail, type WorkflowRailStep } from "../../compositions/WorkflowRail/WorkflowRail";

/**
 * Journey · Read a failed Job. The screen states four things in order — what
 * failed, that the job is over, where the branch is, and where the log is.
 *
 * The header here carries no `data-component` in the drawing, so it is composed
 * from a Badge and text rather than named absent — unlike the running job's,
 * which the drawing names as `Job detail header actions`.
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
        <div className="armada-screen__ident">
          <div className="armada-screen__ident-line">
            <Badge status="completed-failed" icon={X}>
              Failed
            </Badge>
            <span className="armada-screen__title">Cache the manifest read</span>
            <span className="armada-screen__job-id">job_91ab</span>
          </div>
          <div className="armada-screen__meta">
            <span>
              Stopped at <span className="armada-screen__value" data-sans>Run tests</span>,
              step <span className="armada-screen__value">3 of 4</span>
            </span>
            <span>
              Ran <span className="armada-screen__value">22m 41s</span>
            </span>
            <span>
              Spend, estimated <span className="armada-screen__value">~$2.10</span>
            </span>
            <span>Dispatched by you</span>
          </div>
        </div>

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
            <span className="armada-screen__caption" data-muted>
              The failed step is hued and carries a surface, so the row that ended the Job
              stays findable while the Check output is read beside it. The gate rows stay
              neutral — the step&rsquo;s state is hued, a Check&rsquo;s exit code is
              measured. <span className="armada-screen__mono">Implement</span> carries two
              checks and both had to pass:{" "}
              <span className="armada-screen__mono">cargo build</span> succeeds on an empty
              diff, so the build alone would advance a drone that did nothing.
            </span>
          </div>

          <div className="armada-screen__col" data-loose>
            <div className="armada-screen__col">
              <div className="armada-screen__head-row">
                <span className="armada-screen__eyebrow">Check output</span>
                <span className="armada-screen__tag">exit 1 · 4.2s · tail 12 lines</span>
              </div>
              <pre className="armada-screen__output">{tail}</pre>
              <span className="armada-screen__caption" data-muted>
                Readable without opening a log file. Mono throughout, because every line of
                it is machine output, and the tail is bounded with the full capture one click
                away.
              </span>
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

      <p className="armada-screen__note">
        <span className="armada-screen__strong">No retry, no category, no suggestion.</span>{" "}
        All three controls take you to the work; none offers to do anything about it. The
        screen states four things in order — what failed, that the job is over, where the
        branch is, and where the log is — and the sentence saying nothing happens
        automatically is written out rather than left to be inferred from an absence of
        buttons.{" "}
        <span className="armada-screen__strong">
          The per-job log is named here because M1 is when everything is broken.
        </span>{" "}
        The Check output pane answers what the suite said; the log answers what Fleet, the
        drone and Bridge each did, joined on{" "}
        <span className="armada-screen__mono">job_id</span>. Showing the path and the error
        count means a person can reach it without knowing the sink layout, and it costs one
        row and one button rather than a viewer.
      </p>
    </div>
  ),
};
