import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { Lock, ShieldCheck, ShieldMinus, ShieldX } from "lucide-react";
import { WorkflowRail, type WorkflowRailStep } from "./WorkflowRail";

/**
 * One story per rail state the contract names: a running Job, a stopped step,
 * a failed step, a rail with no labels, an ungated step and a hard
 * prerequisite.
 *
 * The step names are the M1 drawing's four — nouns naming the artifact, which
 * is the settled rule for a step's name on every surface.
 */
const meta: Meta<typeof WorkflowRail> = {
  title: "Compositions/Workflow rail",
  component: WorkflowRail,
};
export default meta;

type Story = StoryObj<typeof WorkflowRail>;

/**
 * The evidence row on an ungated step wants the plain page-with-a-check
 * outline, and `file-check` has no entry in `packages/icons/icons.toml`. The
 * row renders a channel short rather than reaching for an unregistered glyph.
 * Reported.
 */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

const running: WorkflowRailStep[] = [
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
      { command: "build · cargo build --workspace", result: "not reached", icon: ShieldMinus, iconLabel: "Not reached" },
      { command: "diff_nonempty", result: "not reached", icon: ShieldMinus, iconLabel: "Not reached" },
    ],
  },
  {
    id: "verify",
    label: "Run tests",
    activity: "not_started",
    status: "not started",
    gates: [
      { command: "test · cargo test --workspace", result: "not reached", icon: ShieldMinus, iconLabel: "Not reached" },
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

/**
 * A running Job. The rail is the most specific running mark on job detail, so
 * the current step's dot pulses and the header's Running badge goes static.
 * Two of the four steps carry no Check and say so.
 */
export const Running: Story = {
  args: { steps: running, pulsing: true },
};

/**
 * The same rail with the pulse elsewhere. A rail rendered where a more
 * specific mark is present takes no motion — one per screen, on the thing
 * being read.
 */
export const RunningPulseElsewhere: Story = {
  args: { steps: running, pulsing: false },
};

/**
 * A failed step: hued `x`, `--step-failed-bg` beneath it, and the outcome in
 * words at the trailing edge. The gate rows stay neutral — the step's state is
 * hued, the Check's exit code is measured.
 */
export const Failed: Story = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced", evidence: { icon: NO_GLYPH_IN_REGISTRY, label: "evidence · 13:58" } },
      {
        id: "implement",
        label: "Implement",
        activity: "advanced",
        status: "advanced",
        gates: [
          { command: "build · cargo build --workspace", result: "exit 0", icon: ShieldCheck, iconLabel: "Passed" },
          { command: "diff_nonempty", result: "passed", icon: ShieldCheck, iconLabel: "Passed" },
        ],
      },
      {
        id: "verify",
        label: "Run tests",
        activity: "failed",
        status: "failed a check",
        gates: [{ command: "test · cargo test --workspace", result: "exit 1", icon: ShieldX, iconLabel: "Failed" }],
      },
      { id: "handoff", label: "Summarise", activity: "not_started", status: "not started" },
    ],
  },
};

/**
 * A stopped step: retries spent, which is not retrying and not waiting on you.
 * `flag` stays `--fg-default` because `--step-stopped-bg` already carries the
 * warning, and a hued flag would say it twice.
 */
export const Stopped: Story = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
      {
        id: "implement",
        label: "Implement",
        activity: "stopped",
        status: "retries spent",
        gates: [{ command: "build · cargo build --workspace", result: "exit 101", icon: ShieldX, iconLabel: "Failed" }],
      },
      { id: "verify", label: "Run tests", activity: "not_started", status: "not started" },
    ],
  },
};

/**
 * A step waiting on a person, and one retrying. Neither ends the Job, so
 * neither takes a surface — `retrying` takes no hue at all.
 */
export const WaitingAndRetrying: Story = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
      { id: "review", label: "Review the diff", activity: "awaiting_human", status: "waiting on you", current: true },
      { id: "fix", label: "Fix", activity: "retrying", status: "retrying · attempt 2" },
    ],
  },
};

/**
 * A killed step takes no hue. It is a human decision rather than a system
 * failure, and it must not read as an error — which is exactly what separates
 * it from the failed row above.
 */
export const Killed: Story = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
      { id: "implement", label: "Implement", activity: "killed", status: "killed · 4m 09s" },
      { id: "verify", label: "Run tests", activity: "not_started", status: "not started" },
    ],
  },
};

/**
 * A hard prerequisite: `lock` at the row's trailing edge, `--fg-muted`, label
 * only, with no row action behind it — the way past a locked step is Pilot. It
 * is drawn here to place it; **Hard prerequisite lock is its own row in
 * `components.toml`** and is not built in this pass.
 */
export const HardPrerequisite: Story = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
      {
        id: "verify",
        label: "Run tests",
        activity: "not_started",
        status: "not started",
        trailing: (
          <span
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: "var(--space-1)",
              flex: "none",
              color: "var(--fg-muted)",
              fontSize: "var(--text-2xs)",
            }}
          >
            <Lock size={12} strokeWidth={2} aria-hidden />
            Cannot be skipped
          </span>
        ),
      },
    ],
  },
};

/**
 * The same rail with `step_id` in place of a label: honest, and useless to
 * scan. Four schema identifiers on the surface a person watches. This is what
 * renders where a workflow's `label` values are not written — see
 * `[workflow-step-human-label]`.
 */
export const LabelsMissing: Story = {
  args: {
    steps: [
      { id: "plan", label: "plan", labelIsAnIdentifier: true, activity: "advanced", status: "advanced" },
      { id: "implement", label: "implement", labelIsAnIdentifier: true, activity: "running", status: "running", current: true },
      { id: "verify", label: "verify", labelIsAnIdentifier: true, activity: "not_started", status: "not started" },
      { id: "handoff", label: "handoff", labelIsAnIdentifier: true, activity: "not_started", status: "not started" },
    ],
    pulsing: true,
  },
};

/**
 * One step. A rail of one is still a rail — it answers where the work got to,
 * and a workflow of one step has an answer.
 */
export const OneStep: Story = {
  args: {
    steps: [{ id: "fix", label: "Fix", activity: "running", status: "running · 1m 40s", current: true }],
    pulsing: true,
  },
};
