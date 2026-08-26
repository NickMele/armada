import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, CircleDot, Clock, GitBranch, OctagonAlert, Power, UserCheck, X } from "lucide-react";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { StepBar } from "../StepBar/StepBar";
import { JobRowStacked } from "./JobRowStacked";

/**
 * One story per Job state the M1 drawing puts in the list, plus the three row
 * states the focus model names — focused, selected, dimmed — and the row at
 * the 720px floor.
 *
 * **The labels come from the enum→verb map**, `crates/core-model/domain/
 * enum-verbs.toml`, sentence-cased. They are written here because that map is
 * not generated into TypeScript yet, and nowhere that ships.
 */
const meta: Meta<typeof JobRowStacked> = {
  title: "Compositions/Job row (stacked)",
  component: JobRowStacked,
};
export default meta;

type Story = StoryObj<typeof JobRowStacked>;

const open = (
  <SplitButton
    ground="card"
    items={[
      { label: "Copy job id", shortcut: "⌘C" },
      { label: "Kill", shortcut: "x", danger: true },
    ]}
  >
    Open
  </SplitButton>
);

/**
 * A Job that has not run. `needs approval` is the verb the enum→verb map
 * carries — the badge means a person must act, not that time is passing.
 *
 * **The field set is different, so the track list is.** No branch, no step and
 * no elapsed yet: a job that has not run has different facts, and the track
 * list belongs to the field set rather than to the row.
 */
export const NeedsApproval: Story = {
  args: {
    status: "awaiting-approval",
    statusIcon: UserCheck,
    statusLabel: "Needs approval",
    headline: "Coalesce concurrent token refreshes",
    jobId: "job_7c31",
    fields: [
      { label: "Workflow", value: "bug, 4 steps", quiet: true },
      { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
      { value: "Not started", quiet: true },
      { value: "created 09:12", quiet: true },
      { value: "Dispatched by you" },
    ],
    action: (
      <SplitButton ground="card" items={[{ label: "Reject", danger: true }]}>
        Approve
      </SplitButton>
    ),
  },
};

/**
 * `queued` renders grey whatever its reason, and a reader correctly moves past
 * a grey row. Where a reason is set it supplies the headline verb and the
 * glyph; with none it reads queued.
 */
export const Queued: Story = {
  args: {
    status: "not-started",
    statusIcon: Clock,
    statusLabel: "Queued",
    headline: "Retire the legacy poke path",
    jobId: "job_8b42",
    fields: [
      { label: "Workflow", value: "bug, 4 steps", quiet: true },
      { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
      { value: "Waiting on a drone", emphasis: true },
      { value: "approved 09:20", quiet: true },
      { value: "Dispatched by you" },
    ],
    action: open,
  },
};

/**
 * The running row, as the list draws it: the badge pulses, and the row is not
 * focused. The pulse is the Job's state rather than the cursor's position, and
 * the step bar beside it stays still.
 */
export const Running: Story = {
  args: {
    pulsing: true,
    status: "running",
    statusIcon: CircleDot,
    statusLabel: "Running",
    headline: "Split the settings reducer",
    jobId: "job_2d90bb",
    fields: [
      { value: "fix/settings-split", mono: true, icon: GitBranch, copyValue: "fix/settings-split" },
      { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "~$1.80", mono: true },
    ],
    action: open,
  },
};

/**
 * The same row with the keyboard cursor on it: a 2px `--accent` left edge and
 * `--bg-hover`. Focus adds the edge and nothing else — the pulse was already
 * there.
 */
export const RunningFocused: Story = {
  args: { ...Running.args, focused: true } as never,
};

/** `--accent-muted` fill. Selected and focused are different states and coexist. */
export const Selected: Story = {
  args: { ...Running.args, selected: true } as never,
};

/**
 * De-emphasised: `--border-subtle` and `--fg-subtle`, never an `opacity`.
 * Dimming is a token.
 */
export const Dimmed: Story = {
  args: { ...Running.args, dimmed: true } as never,
};

/**
 * `escalated` renders its reason, never its own name — nobody says a Job
 * escalated at step 3. The verb and the glyph both come from the escalation
 * reason, which is why the Job status row in the map carries neither.
 */
export const EscalatedStalled: Story = {
  args: {
    status: "escalated",
    statusIcon: OctagonAlert,
    statusLabel: "Stalled",
    headline: "Job 12 stalled at step 3",
    jobId: "job_12",
    fields: [
      { value: "auth/session.rs", mono: true, icon: GitBranch, copyValue: "auth/session.rs" },
      { value: <StepBar total={5} current={3} activity="stopped" label="Step 3 of 5" /> },
      { value: "3 pokes", emphasis: true },
      { value: "12m", mono: true },
      { value: "~$1.80", mono: true },
    ],
    action: (
      <SplitButton ground="card" items={[{ label: "Kill", danger: true }]}>
        Pilot
      </SplitButton>
    ),
  },
};

/**
 * A second stall at the same step reads "2nd time" in the headline, and the
 * detail view surfaces the prior attempt. Presentation only — recurrence
 * changing behaviour is a separate decision.
 */
export const EscalatedSecondTime: Story = {
  args: {
    ...EscalatedStalled.args,
    headline: "Job 12 stalled at step 3, 2nd time",
  } as never,
};

/**
 * A failed segment is loud. At M1 a failed Check ends the Job, so this row is
 * the entire reason a person opened the screen.
 */
export const Failed: Story = {
  args: {
    status: "completed-failed",
    statusIcon: X,
    statusLabel: "Failed",
    headline: "Cache the manifest read",
    jobId: "job_91ab",
    fields: [
      { value: "feat/manifest-cache", mono: true, icon: GitBranch, copyValue: "feat/manifest-cache" },
      { value: <StepBar total={4} current={3} activity="failed" label="Step 3 of 4" /> },
      { value: "Run tests", emphasis: true },
      { value: "22m 41s", mono: true },
      { value: "~$2.10", mono: true },
    ],
    action: open,
  },
};

/**
 * A killed segment is not loud. Killing is a human decision rather than a
 * system failure and must not read as an error — the segment keeps
 * `--fg-default` and the badge takes the grey `killed` hue.
 */
export const Killed: Story = {
  args: {
    status: "killed",
    statusIcon: Power,
    statusLabel: "Killed",
    headline: "Rename the session token field",
    jobId: "job_5e88",
    fields: [
      { value: "feat/session-rename", mono: true, icon: GitBranch, copyValue: "feat/session-rename" },
      { value: <StepBar total={4} current={2} activity="killed" label="Step 2 of 4" /> },
      { value: "Implement", emphasis: true },
      { value: "4m 09s", mono: true },
      { value: "~$0.60", mono: true },
    ],
    action: open,
  },
};

/** A finished Job: every segment past, and no hue left for a current step. */
export const Done: Story = {
  args: {
    status: "completed-success",
    statusIcon: Check,
    statusLabel: "Done",
    headline: "Add a retry ceiling to the poke loop",
    jobId: "job_4f10",
    fields: [
      { value: "fix/poke-ceiling", mono: true, icon: GitBranch, copyValue: "fix/poke-ceiling" },
      { value: <StepBar total={4} current={5} activity="advanced" label="All 4 of 4 steps advanced" /> },
      { value: "Summarise" },
      { value: "18m 22s", mono: true },
      { value: "~$2.40", mono: true },
    ],
    action: open,
  },
};

/**
 * Spend follows the active billing mode, and the visible number is always the
 * number that gates dispatch. The track is sized for the wider of the two
 * strings rather than for whichever example came first.
 */
export const SpendAsQuota: Story = {
  args: {
    ...Running.args,
    fields: [
      { value: "fix/settings-split", mono: true, icon: GitBranch, copyValue: "fix/settings-split" },
      { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "68% quota", mono: true },
    ],
  } as never,
};

/**
 * At the 768px floor, with the rail at 48px: 720px of content. **The same
 * row.** Nothing reshapes and no field is dropped — secondary values truncate
 * with a tooltip carrying the full string, and the badge stays leading so
 * status is still the first thing caught.
 */
export const AtTheWidthFloor: StoryObj = {
  render: () => (
    <div style={{ width: "calc(var(--window-floor) - var(--sidebar-rail))" }}>
      <JobRowStacked
        status="running"
        statusIcon={CircleDot}
        statusLabel="Running"
        headline="Split the settings reducer so the selectors can be tested alone"
        jobId="job_2d90bb"
        fields={[
          { value: "fix/settings-split-selectors", mono: true, icon: GitBranch, copyValue: "fix/settings-split-selectors" },
          { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
          { value: "Implement", emphasis: true },
          { value: "11m 03s", mono: true },
          { value: "~$1.80", mono: true },
        ]}
        action={open}
      />
    </div>
  ),
};

/**
 * A Convoy has no single workspace, so its row says "3 workspaces" where every
 * other row names one — a count in a field that elsewhere holds an identifier.
 * Rendered as the open question describes it, unresolved. See
 * `[convoy-row-single-workspace]`.
 */
export const Convoy: Story = {
  args: {
    ...Running.args,
    headline: "Retire the poke path across the fleet",
    fields: [
      { value: "3 workspaces" },
      { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "~$4.20", mono: true },
    ],
  } as never,
};
