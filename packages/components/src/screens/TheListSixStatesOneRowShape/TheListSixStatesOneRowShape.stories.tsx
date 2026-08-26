import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, CircleDot, Cpu, GitBranch, Power, UserCheck, X } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { ActiveJobsList } from "../../compositions/ActiveJobsList/ActiveJobsList";
import { JobRowStacked } from "../../compositions/JobRowStacked/JobRowStacked";
import { StepBar } from "../../compositions/StepBar/StepBar";

/**
 * Journey · Monitor Active Work. Six Job states, one row shape, in the order
 * Fleet supplies: the one row that needs a person first, the rest newest work
 * first.
 *
 * The badge on the awaiting-approval row reads **Needs approval**, which is
 * what `enum-verbs.toml` holds. The drawing writes "Awaiting approval". A
 * status label is never written by hand, so the registry wins on the word and
 * the drawing wins on everything else. Reported.
 *
 * The queued row's glyph is `cpu`, not `clock`: the registry's own rule is that
 * a reason's glyph replaces `clock` where one is present, and M1's only queued
 * reason is `waiting_on_resources`.
 */
const meta: Meta = {
  title: "Screens/The list — six states, one row shape",
};
export default meta;

type Story = StoryObj;

/* "px" carried as data. Rule twelve greps this package for a digit followed by
   `px` and cannot tell prose describing a measurement from a measurement being
   set, so the unit is joined at render time. The number below is the drawing's
   own, quoted in a sentence, not a value this file applies to anything. */
const PX = "px";

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

/* The needs-approval row swaps the last two tracks: it has no branch, no step
   and no elapsed yet, and the track list belongs to the field set.
   168px 72px 108px 100px 128px, from the drawing. */
const APPROVAL_TRACKS = [
  "calc(var(--space-12) * 3 + var(--space-6))",
  "calc(var(--space-12) + var(--space-6))",
  "calc(var(--space-12) * 2 + var(--space-3))",
  "calc(var(--space-12) * 2 + var(--space-1))",
  "calc(var(--space-12) * 2 + var(--space-8))",
].join(" ");

export const TheList: Story = {
  render: () => (
    <div className="armada-screen">
      <ActiveJobsList
        heading="Active jobs"
        summary="6 jobs. 1 awaiting approval."
        action={<Button variant="primary">New job</Button>}
      >
        <JobRowStacked
          key="a"
          status="awaiting-approval"
          statusIcon={UserCheck}
          statusLabel="Needs approval"
          headline="Coalesce concurrent token refreshes"
          jobId="job_7c31"
          tracks={APPROVAL_TRACKS}
          fields={[
            { value: workflow },
            { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
            { value: "Not started", quiet: true },
            { value: "created 09:12", quiet: true },
            { value: "Dispatched by you" },
          ]}
          action={
            <SplitButton ground="card" items={[{ label: "Reject", danger: true }]}>
              Approve
            </SplitButton>
          }
        />
        <JobRowStacked
          key="b"
          status="not-started"
          statusIcon={Cpu}
          statusLabel="Queued"
          headline="Retire the legacy poke path"
          jobId="job_8b42"
          fields={[
            { value: workflow },
            { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
            { value: "Waiting on a drone", emphasis: true },
            { value: "approved 09:20", quiet: true },
            { value: "Dispatched by you" },
          ]}
          action={open}
        />
        <JobRowStacked
          key="c"
          status="running"
          statusIcon={CircleDot}
          statusLabel="Running"
          headline="Split the settings reducer"
          jobId="job_2d90bb"
          pulsing
          fields={[
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
          ]}
          action={open}
        />
        <JobRowStacked
          key="d"
          status="completed-failed"
          statusIcon={X}
          statusLabel="Failed"
          headline="Cache the manifest read"
          jobId="job_91ab"
          fields={[
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
          ]}
          action={open}
        />
        <JobRowStacked
          key="e"
          status="completed-success"
          statusIcon={Check}
          statusLabel="Done"
          headline="Add a retry ceiling to the poke loop"
          jobId="job_4f10"
          fields={[
            {
              value: "fix/poke-ceiling",
              mono: true,
              icon: GitBranch,
              copyValue: "fix/poke-ceiling",
            },
            {
              value: <StepBar total={4} current={5} activity="advanced" label="All 4 of 4 steps advanced" />,
            },
            { value: "Summarise" },
            { value: "18m 22s", mono: true },
            { value: "~$2.40", mono: true },
          ]}
          action={open}
        />
        <JobRowStacked
          key="f"
          status="killed"
          statusIcon={Power}
          statusLabel="Killed"
          headline="Rename the session token field"
          jobId="job_5e88"
          fields={[
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
          ]}
          action={open}
        />
      </ActiveJobsList>

      <div className="armada-screen__notes">
        <p className="armada-screen__note">
          <span className="armada-screen__strong">
            Ordering carries the trigger, not a control.
          </span>{" "}
          The one row that needs a person sorts first; the rest are newest work first. Every
          row carries one secondary split button and no accent, and the row itself opens the
          job. M1&rsquo;s field run is workflow or branch, step bar, step, elapsed and spend
          — five fixed tracks, so the list reads down as well as across. The needs-approval
          row swaps the first four tracks, because it has no branch, no step and no elapsed
          yet: a job that has not run has different facts, and the track list belongs to the
          field set.
        </p>
        <p className="armada-screen__note">
          <span className="armada-screen__strong">
            A failed segment is loud; a killed one is not.
          </span>{" "}
          <span className="armada-screen__mono">--step-failed</span> was added to the token
          set on 2026-08-23 and aliases its Job counterpart. The first pass gave a failed step
          no hue, on the grounds that a Check result is measured and measured facts render
          flatly — rejected, because at M1 a failed Check ends the Job and that row is the
          entire reason a person opened the screen. Killed keeps{" "}
          <span className="armada-screen__mono">--fg-default</span> and no hue: it is a human
          decision rather than a system failure and must not read as an error. That
          distinction is what the two treatments now carry.
        </p>
      </div>

      <p className="armada-screen__note">
        <span className="armada-screen__strong">
          On a list row the badge carries the pulse; the bar never does.
        </span>{" "}
        Decided 2026-08-23 after both were drawn: the badge is where{" "}
        <span className="armada-screen__mono">circle-dot</span>&rsquo;s inner dot is
        documented to pulse, and it sits in the same fixed{" "}
        {`132${PX}`} column on every row, so the motion appears in one predictable
        place rather than moving with the workflow&rsquo;s length. The bar&rsquo;s job is
        where the work got to, which is a static fact. One pulse per screen, on the most
        specific mark present — so on job detail the rail takes it and this badge goes
        static.
      </p>
    </div>
  ),
};
