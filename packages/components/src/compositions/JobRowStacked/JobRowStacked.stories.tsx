import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, CircleDot, Clock, Cpu, Folder, GitBranch, OctagonAlert, Power, UserCheck, X } from "lucide-react";
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
 *
 * **Every row's field run now carries origin** — display-only, never a filter
 * axis (`docs/concepts/job-board.md`, Origin tagging). The five values:
 * dispatched by you, found by Fleet, drafted in Helm, workflow-triggered, and
 * sub-dispatched, which additionally names the parent Job. See issue 218.
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
 * The track list a Job at the gate takes: no branch, step or elapsed reading
 * yet, so a "created" timestamp and origin do the work the running row's
 * time and spend tracks do. Composed from this component's own track custom
 * properties, the same way `APPROVAL_TRACKS` in `TheListSixStatesOneRowShape`
 * composes its own — duplicated here rather than imported, because a
 * composition's story cannot reach into a screen, and because
 * `APPROVAL_TRACKS` sits outside this change's write scope. Reported.
 */
const GATE_TRACKS = [
  "var(--armada-track-origin)",
  "var(--armada-track-bar)",
  "var(--armada-track-step)",
  "var(--armada-track-time)",
  "var(--armada-track-provenance)",
].join(" ");

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
    tracks: GATE_TRACKS,
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
    tracks: GATE_TRACKS,
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
      { value: "Dispatched by you" },
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
      { value: "Found by Fleet" },
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
      { value: "Found by Fleet" },
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
      { value: "Workflow-triggered" },
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
      { value: "Drafted in Helm" },
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
      { value: "Dispatched by you" },
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
          { value: "Dispatched by you" },
        ]}
        action={open}
      />
    </div>
  ),
};

/**
 * A Convoy has no single workspace, so its row **names its first write target
 * and counts the rest** — `+2` where three Workspaces are declared. Every
 * other row names a place in that column, so this one does too, and the folder
 * glyph keeps meaning a workspace. A bare "3 workspaces" was drawn and
 * rejected: it puts a count where the column holds an identifier.
 *
 * **No chip and no hue.** A bordered pill is a Job state and nothing else, so
 * shape reads as plain text. The Board computes it from `write_targets` and
 * `atomic` — nothing on Job stores a shape.
 */
export const Convoy: Story = {
  args: {
    ...Running.args,
    headline: "Retire the poke path across the fleet",
    fields: [
      { value: "crates/fleet +2", mono: true, icon: Folder },
      { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "~$4.20", mono: true },
      { value: "Workflow-triggered" },
    ],
  } as never,
};

/**
 * **The one exception on the Board: a sub-dispatched Job that is out of
 * headroom.** Almost every sub-dispatched Job is already running before
 * anything could render it — this is the single reason one appears here,
 * `waiting_on_resources`, still `queued` (`docs/concepts/job-board.md`,
 * "A sub-dispatched Job is usually already running").
 *
 * **No Approve, no Dispatch** — the approval already happened at the parent
 * Job named in the origin field, so a decision control here would offer a
 * choice that has no content. **Kill stays available**, and the row is
 * dimmed, the same visually-distinct treatment a blocked Job gets. Without
 * the origin field naming `job_2d90bb` — the parent, running in the
 * `Running` story above — a read-only row with no primary action would look
 * arbitrary; that is the whole reason this issue put origin on the row
 * rather than only on job detail.
 */
export const SubDispatchedWaitingOnResources: Story = {
  args: {
    status: "not-started",
    statusIcon: Cpu,
    statusLabel: "Waiting on resources",
    headline: "Precompute embeddings for the batch import step",
    jobId: "job_9f21",
    dimmed: true,
    tracks: GATE_TRACKS,
    fields: [
      { label: "Workflow", value: "chore, 3 steps", quiet: true },
      { value: <StepBar total={3} current={1} label="Step 2 of 3, waiting" /> },
      { value: "Held for CPU headroom", emphasis: true },
      { value: "queued 09:41", quiet: true },
      { value: "Sub-dispatched by job_2d90bb" },
    ],
    action: (
      <SplitButton ground="card" variant="destructive" items={[]}>
        Kill
      </SplitButton>
    ),
  },
};
