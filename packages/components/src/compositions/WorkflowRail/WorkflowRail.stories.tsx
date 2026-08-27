import type { Meta, StoryObj } from "@storybook/react-vite";
import { FileCheck, Lock, ShieldCheck, ShieldMinus, ShieldX } from "lucide-react";
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
 * The evidence row on an ungated step takes the plain page-with-a-check
 * outline. `[icons.file-check]` reserves it to a submission that landed and is
 * explicit that it is never a Check result — the check marks that evidence was
 * submitted, not that anything passed. These stories claimed the registry had
 * no row for it; it does.
 */
const EVIDENCE = FileCheck;

const running: WorkflowRailStep[] = [
  {
    id: "plan",
    label: "Plan the change",
    activity: "advanced",
    status: "advanced",
    evidence: { icon: EVIDENCE, iconLabel: "Evidence", label: "evidence · 09:14" },
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
    evidence: { icon: EVIDENCE, iconLabel: "Evidence", label: "" },
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
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced", evidence: { icon: EVIDENCE, label: "evidence · 13:58" } },
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

/**
 * The rail Bridge draws from `GET /jobs/:job_id`. **Nothing here is inferred**
 * — `state` is `job_steps.state` as Fleet recorded it, and the duration on each
 * row is `entered_at` to `updated_at` on that step.
 *
 * **The status word is `job_steps.state`'s own wire spelling.**
 * `enum-verbs.toml` carries no `step_state` rows, so the generated module emits
 * an empty map and the underscored value renders — the same fallback a Check
 * outcome took until its rows landed. A word chosen at the call site would be
 * the second vocabulary that rule exists to prevent. Reported.
 *
 * **The unstarted step shows no duration.** `entered_at` is stamped at Job
 * creation for every step of the frozen workflow, so a span on a step that has
 * not run measures how long the Job has been alive — and `0s` on a row reads as
 * a step that ran instantly. The running step measures to now instead, because
 * `updated_at` stopped moving when the step started.
 */
export const ServedSteps: Story = {
  args: {
    steps: [
      {
        id: "plan",
        label: "plan",
        labelIsAnIdentifier: true,
        activity: "advanced",
        status: "advanced",
        elapsed: "2m 14s",
        verdict: "passed",
        verdictNamed: "passed",
      },
      {
        id: "implement",
        label: "implement",
        labelIsAnIdentifier: true,
        activity: "running",
        status: "running",
        current: true,
        elapsed: "11m 03s",
      },
      {
        id: "verify",
        label: "verify",
        labelIsAnIdentifier: true,
        activity: "not_started",
        status: "not_started",
      },
      {
        id: "handoff",
        label: "handoff",
        labelIsAnIdentifier: true,
        activity: "not_started",
        status: "not_started",
      },
    ],
    pulsing: true,
  },
};

/**
 * The six `job_steps.state` values `crates/core-model/domain/step-states.toml`
 * declares, one row each, exactly as Bridge draws them from `GET /jobs/:job_id`.
 *
 * **Every word on every row is the wire's own.** The label is the `step_id`,
 * because neither `StepDetail` nor `WorkflowStep` carries the `label` the
 * WorkflowDef declares. The status is the state's underscored spelling, because
 * `enum-verbs.toml` has no `step_state` rows. The Check row says its name and
 * not its command, because `GET /manifests` serves check names and not their
 * `run` strings. Three wire gaps, drawn rather than papered over.
 *
 * `not reached` and its `shield-minus` come from the criterion Check
 * vocabulary: `check_outcome`'s five are what a Check that *ran* did, and it
 * declares no row for one the gate has not reached.
 */
export const EveryStepState: Story = {
  args: {
    steps: [
      {
        id: "not_started",
        label: "not_started",
        labelIsAnIdentifier: true,
        activity: "not_started",
        status: "not_started",
        gates: [{ command: "build", result: "not reached", icon: ShieldMinus, iconLabel: "not reached" }],
      },
      {
        id: "running",
        label: "running",
        labelIsAnIdentifier: true,
        activity: "running",
        status: "running",
        current: true,
        elapsed: "6m 12s",
        gates: [{ command: "test", result: "not reached", icon: ShieldMinus, iconLabel: "not reached" }],
      },
      {
        id: "awaiting_human",
        label: "awaiting_human",
        labelIsAnIdentifier: true,
        activity: "awaiting_human",
        status: "awaiting_human",
        elapsed: "1m 04s",
      },
      {
        id: "retrying",
        label: "retrying",
        labelIsAnIdentifier: true,
        activity: "retrying",
        status: "retrying",
        elapsed: "3m 41s",
        verdict: "failed · failed a check",
        verdictNamed: "failed",
      },
      {
        id: "advanced",
        label: "advanced",
        labelIsAnIdentifier: true,
        activity: "advanced",
        status: "advanced",
        elapsed: "2m 14s",
        verdict: "passed",
        verdictNamed: "passed",
        gates: [{ command: "fmt", result: "passed", icon: ShieldCheck, iconLabel: "passed" }],
      },
      {
        id: "stopped",
        label: "stopped",
        labelIsAnIdentifier: true,
        activity: "stopped",
        status: "stopped",
        elapsed: "12m 30s",
        gates: [
          { command: "build", result: "failed · exit 0 → exit 101", icon: ShieldX, iconLabel: "failed" },
        ],
      },
    ],
    pulsing: true,
  },
};

/**
 * A verdict and an activity on one row, on different axes.
 *
 * A step retrying after a refusal is `running` in activity and `failed` in
 * verdict at the same moment, which is why one column cannot say both. The
 * trigger beside the verdict takes its verb from the escalation vocabulary
 * where the registry has one.
 */
export const AVerdictBesideTheState: Story = {
  args: {
    steps: [
      {
        id: "plan",
        label: "plan",
        labelIsAnIdentifier: true,
        activity: "advanced",
        elapsed: "2m 14s",
        verdict: "passed",
        verdictNamed: "passed",
      },
      {
        id: "verify",
        label: "verify",
        labelIsAnIdentifier: true,
        activity: "retrying",
        current: true,
        elapsed: "6m 51s",
        verdict: "failed · failed a check",
        verdictNamed: "failed",
      },
      { id: "handoff", label: "handoff", labelIsAnIdentifier: true, activity: "not_started" },
    ],
  },
};

/**
 * The two different sentences a rail says where a Check would be.
 *
 * **`checks: []` is "this step is ungated" and an absent key is "Fleet cannot
 * say."** The first is the ordinary case — every step of the `bug` workflow
 * declares none — and the second means the Job named a workflow this Fleet does
 * not hold. Neither is a gap, because a blank row reads as a gate that failed
 * to render, and one sentence for both would tell a reader a step gates on
 * nothing when what is true is that nobody could answer.
 */
export const UngatedAndUnanswerable: Story = {
  args: {
    steps: [
      {
        id: "reproduce",
        label: "reproduce",
        labelIsAnIdentifier: true,
        activity: "advanced",
        elapsed: "1m 40s",
        evidence: { label: "" },
      },
      {
        id: "root_cause",
        label: "root_cause",
        labelIsAnIdentifier: true,
        activity: "running",
        current: true,
        elapsed: "4m 02s",
        // What Bridge passes when `checks` is absent from the served step.
        ungatedLabel: "Fleet cannot say what this step checks",
        evidence: { label: "" },
      },
    ],
    pulsing: true,
  },
};

/**
 * The five Check outcomes, rendered apart.
 *
 * **Only `passed` advances a step, and the other four are four different
 * things.** A hanging Check and a Check whose command is not installed need
 * opposite responses, so collapsing them into "failed" hides the one difference
 * a reader acts on. A pass carries nothing beside it because a pass measured
 * nothing — the outcome is the whole sentence.
 *
 * **Every result here is the wire's own spelling.**
 * `crates/core-model/domain/check-outcomes.toml` declares the five and
 * `enum-verbs.toml` carries no `check_outcome` rows, so there is no sanctioned
 * verb, glyph or hue for any of them. The gate rows render a glyph short rather
 * than borrowing one that means something else. Reported.
 */
export const TheFiveCheckOutcomes: Story = {
  args: {
    steps: [
      {
        id: "verify",
        label: "verify",
        labelIsAnIdentifier: true,
        activity: "stopped",
        current: true,
        elapsed: "3m 18s",
        verdict: "failed · failed a check",
        verdictNamed: "failed",
        gates: [
          { command: "fmt" , result: "passed" },
          { command: "build", result: "failed · exit 0 → exit 101" },
          { command: "test", result: "signalled · SIGKILL" },
          { command: "audit", result: "timed_out · 120s budget" },
          { command: "typecheck", result: "never_ran · tsc is not installed" },
        ],
      },
    ],
  },
};
