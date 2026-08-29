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
  { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced",
    evidence: { icon: EVIDENCE, iconLabel: "Evidence", label: "evidence · 09:14" } },
  { id: "implement", label: "Implement", activity: "running", status: "running · 6m 12s", current: true,
    gates: [
      { command: "build · cargo build --workspace", result: "not reached", icon: ShieldMinus, iconLabel: "Not reached" },
      { command: "diff_nonempty", result: "not reached", icon: ShieldMinus, iconLabel: "Not reached" },
    ],
    declarations: [{ label: "judge · 2 criteria", result: "not reached" }, { label: "advance_gate · auto_if_judge_passes" }] },
  { id: "verify", label: "Run tests", activity: "not_started", status: "not started",
    gates: [{ command: "test · cargo test --workspace", result: "not reached", icon: ShieldMinus, iconLabel: "Not reached" }],
    declarations: [{ label: "judge · 1 criterion · gaming check", result: "not reached" }, { label: "advance_gate · auto_if_judge_passes" }] },
  { id: "handoff", label: "Summarise", activity: "not_started", status: "not started",
    declarations: [{ label: "advance_gate · human_always" }] },
];

/**
 * A running Job — the `feature` workflow, as Fleet serves it. The rail is the
 * most specific running mark on job detail, so the current step's dot pulses
 * and the header's Running badge goes static.
 *
 * **Every tier a step declares is on it, including the two that have not run.**
 * `implement` and `tests` each hide a Judge behind their mechanical Checks, and
 * `handoff` gates on a person and nothing else — the step the Job halts on, and
 * the one that read "no check on this step" until its declaration crossed.
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
 * The same workflow at proposal time, every result stripped off it. Nothing has
 * run, so a mark here would draw a verdict where there is only a declaration.
 */
export const AWorkflowBeforeItRuns: Story = {
  args: {
    steps: running.map(({ id, label, gates, declarations }) => ({
      id, label, activity: "not_started" as const,
      gates: gates?.map(({ command }) => ({ command })), declarations: declarations?.map(({ label: what }) => ({ label: what })),
    })),
  },
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
 * A step the gaming check flagged, which is not a step that merely stopped.
 *
 * **The trigger is not the finding.** `evidence_suspect` beside the verdict
 * says the evidence is not to be trusted; `StepDetail.flagged` says which shape
 * of gaming was found and where, and a rail drawing the first without the
 * second showed that something objected and never what it objected to.
 *
 * **Not a verdict, and not a failure.** No glyph — `circle-*` is the Judge's
 * family, `shield-*` the Checks', and `flag` is already the stopped step's own
 * mark one row above. No `--step-failed` either: `evidence_suspect` routes as
 * its own escalation rather than a gate failure, because resubmitting under the
 * same instructions would likely reproduce the gaming, so the block carries
 * weight the way the overruled note does.
 *
 * **The citation is on the rail, not a press away.** An uncited flag is
 * unactionable, and the person who can overrule this from the same screen
 * should not have to open the dialog to see what they are lifting. The patterns
 * render as `flag_if` spells them: no vocabulary carries a verb per gaming
 * pattern. Reported.
 */
export const EvidenceFlaggedByTheGamingCheck: Story = {
  args: {
    steps: [
      { id: "implement", label: "Implement", activity: "advanced", status: "advanced", elapsed: "6m 48s",
        verdict: "passed", verdictNamed: "passed",
        gates: [{ command: "build · cargo build --workspace", result: "passed", icon: ShieldCheck, iconLabel: "passed" }] },
      { id: "verify", label: "Run tests", activity: "stopped", status: "stopped", current: true, elapsed: "3m 07s",
        verdict: "failed · evidence disputed", verdictNamed: "failed",
        gates: [{ command: "test · cargo test --workspace", result: "passed", icon: ShieldCheck, iconLabel: "passed" }],
        declarations: [{ label: "judge · 2 criteria · gaming check" }, { label: "advance_gate · auto_if_judge_passes" }],
        flags: [
          { pattern: "check_config_edited", cited: "package.json · scripts.test now runs vitest run src/unit" },
          { pattern: "assertion_weakened", cited: "src/parse.test.ts:88 · toEqual replaced by toBeDefined" },
        ] },
      { id: "handoff", label: "Summarise", activity: "not_started", status: "not started" },
    ],
  },
};

/**
 * A flagged step that gates on nothing must not also say nothing checked it.
 *
 * A gaming check rides on a `judge_checks[]` entry, so a step carrying a flag
 * was looked at by definition — and "no check on this step" printed above what
 * a check found is the contradiction the declaration rows were added to end,
 * arriving a second time. So a flag counts towards the sum the sentence is
 * drawn from, exactly as a declaration does.
 */
export const AFlaggedStepIsNeverUngated: Story = {
  args: {
    steps: [
      { id: "root_cause", label: "root_cause", labelIsAnIdentifier: true, activity: "stopped", status: "stopped", current: true, elapsed: "8m 22s",
        evidence: { label: "" },
        flags: [{ pattern: "findings_generic", cited: "the report names no file and no line" }] },
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
      { id: "plan", label: "plan", labelIsAnIdentifier: true, activity: "advanced", status: "advanced", elapsed: "2m 14s", verdict: "passed", verdictNamed: "passed" },
      { id: "implement", label: "implement", labelIsAnIdentifier: true, activity: "running", status: "running", current: true, elapsed: "11m 03s" },
      { id: "verify", label: "verify", labelIsAnIdentifier: true, activity: "not_started", status: "not_started" },
      { id: "handoff", label: "handoff", labelIsAnIdentifier: true, activity: "not_started", status: "not_started" },
    ],
    pulsing: true,
  },
};

/**
 * The six `job_steps.state` values `crates/core-model/domain/step-states.toml`
 * declares, one row each, exactly as Bridge draws them from `GET /jobs/:job_id`.
 *
 * **Every word on every row is the wire's own.** The label is the `step_id`
 * because this workflow declares none and Fleet substitutes the id; the status
 * is the state's underscored spelling, because `enum-verbs.toml` has no
 * `step_state` rows; and the Check row says its name and not its command,
 * because `GET /manifests` serves names and not `run` strings. Two wire gaps
 * and one honest fallback, drawn rather than papered over.
 *
 * `not reached` and its `shield-minus` come from the criterion Check
 * vocabulary: `check_outcome`'s six are what the gate *decided* about a Check,
 * and it declares no row for one the gate has not reached. `skipped` is not
 * that row — a skipped Check was reached and deliberately not run.
 */
export const EveryStepState: Story = {
  args: {
    steps: [
      { id: "not_started", label: "not_started", labelIsAnIdentifier: true, activity: "not_started", status: "not_started",
        gates: [{ command: "build", result: "not reached", icon: ShieldMinus, iconLabel: "not reached" }] },
      { id: "running", label: "running", labelIsAnIdentifier: true, activity: "running", status: "running", current: true, elapsed: "6m 12s",
        gates: [{ command: "test", result: "not reached", icon: ShieldMinus, iconLabel: "not reached" }] },
      { id: "awaiting_human", label: "awaiting_human", labelIsAnIdentifier: true, activity: "awaiting_human", status: "awaiting_human", elapsed: "1m 04s" },
      { id: "retrying", label: "retrying", labelIsAnIdentifier: true, activity: "retrying", status: "retrying", elapsed: "3m 41s", verdict: "failed · failed a check", verdictNamed: "failed" },
      { id: "advanced", label: "advanced", labelIsAnIdentifier: true, activity: "advanced", status: "advanced", elapsed: "2m 14s", verdict: "passed", verdictNamed: "passed",
        gates: [{ command: "fmt", result: "passed", icon: ShieldCheck, iconLabel: "passed" }] },
      { id: "stopped", label: "stopped", labelIsAnIdentifier: true, activity: "stopped", status: "stopped", elapsed: "12m 30s",
        gates: [{ command: "build", result: "failed · exit 0 → exit 101", icon: ShieldX, iconLabel: "failed" }] },
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
 * A step a person overruled — and it must not read like one that passed.
 *
 * **Three facts on one row, and all three stay.** The step is `advanced`, the
 * verdict is still `failed` carrying the trigger the Judge refused on, and
 * `overridden` says a person put it there. `StepDetail.overridden` is served as
 * a field so that no surface has to notice `advanced` beside `failed` and work
 * the pair out for itself — the first rail that forgot would draw an overruled
 * Judge as a Judge that cleared the work.
 *
 * The refusal itself is untouched beneath the step: the criterion the Judge
 * refused still reads refused, with what it expected and what it saw. Overruling
 * is a person disagreeing on the record, not a verdict being erased.
 *
 * **Weight and not a second hue.** The verdict carries the row's one
 * `--step-failed`; the override sits beside it in `--fg-default` at medium, so
 * the pair reads as two facts rather than one louder failure. No glyph — the
 * `circle-*` family is the Judge's and `shield-*` is the Checks', and an
 * override is neither.
 */
export const OverruledByAPerson: Story = {
  args: {
    steps: [
      { id: "implement", label: "Implement", activity: "advanced", status: "advanced", elapsed: "6m 48s",
        verdict: "passed", verdictNamed: "passed",
        gates: [{ command: "build · cargo build --workspace", result: "passed", icon: ShieldCheck, iconLabel: "passed" }] },
      { id: "verify", label: "Run tests", activity: "advanced", status: "advanced", elapsed: "12m 30s",
        verdict: "failed · refused by the judge", verdictNamed: "failed", overridden: "overruled by a person",
        gates: [{ command: "test · cargo test --workspace", result: "passed", icon: ShieldCheck, iconLabel: "passed" }],
        declarations: [{ label: "judge · 2 criteria" }, { label: "advance_gate · auto_if_judge_passes" }],
        verdicts: [{ ordinal: 2, criterionId: "crit_7f2a", named: "not_met", verdict: "refused",
          text: "The regression is covered by a test that fails without the fix.",
          expected: "a test that fails on the parent commit",
          produced: "a test asserting the new behaviour only",
          consequence: "a reader cannot tell the fix from the assertion" }] },
      { id: "handoff", label: "Summarise", activity: "running", status: "running · 0m 04s", current: true,
        declarations: [{ label: "advance_gate · human_always" }] },
    ],
    pulsing: true,
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
 * The six Check outcomes, rendered apart.
 *
 * **Two advance a step and only one of them is a pass.** Four of the six are
 * four different failures — a hanging Check and a Check whose command is not
 * installed need opposite responses, so collapsing them into "failed" hides the
 * one difference a reader acts on. A pass carries nothing beside it because a
 * pass measured nothing.
 *
 * **`skipped` is the sixth and it is the deliberate one.** The Check declares
 * which paths it covers in `checks.<name>.when` and this step changed none of
 * them, so the gate did not run it. It advanced the step and it verified
 * nothing, which is why it is neither `passed` nor one of the four — a rail
 * that drew it green would be claiming a Check that never ran.
 *
 * **Every result here is the wire's own spelling.**
 * `crates/core-model/domain/check-outcomes.toml` declares the six and
 * `enum-verbs.toml` now carries a `check_outcome` row for each, so the verb and
 * the glyph are the registry's. The results below are the raw wire words, which
 * is what this story is for: the vocabulary and the value are drawn apart.
 */
export const TheSixCheckOutcomes: Story = {
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
          { command: "storybook", result: "skipped · no changed file is under packages/**" },
        ],
      },
    ],
  },
};

/**
 * **A failed Check names the file it wrote.** Bridge does not read the
 * filesystem, so it names the file and copies it on click — no `copy` glyph, a
 * toast confirms, and it stays neutral like the exit code beside it. A Check
 * that never ran wrote nothing and names no file.
 */
export const AFailedCheckNamesItsOutput: Story = {
  args: {
    steps: [
      {
        id: "verify", label: "Run tests", activity: "failed", status: "failed a check", current: true,
        gates: [
          { command: "test · cargo test --workspace", result: "failed · exit 0 → exit 101", icon: ShieldX, iconLabel: "Failed", outputPath: ".armada/jobs/job_2d90bb/checks/test.log" },
          { command: "diff_nonempty", result: "never started", icon: ShieldMinus, iconLabel: "Never started" },
        ],
      },
    ],
  },
};

/**
 * The four Jobs that are over, and the one stop that is not a dead end.
 *
 * **The step reads `running` on the wire in three of these four.**
 * `job-statuses.toml` freezes the step machine at every terminal status and
 * declares no step state for it: the step keeps what it had, and the Job being
 * terminal is what says everything is over. So the rail reads a step's activity
 * against its Job's status — a rendering rule Bridge holds in `frozen.ts`, not
 * a seventh state, and nothing here reaches the wire.
 *
 * **`escalated` is the one that is not terminal.** A redirect or a restart
 * resumes exactly its step, so it keeps `flag` and `--step-stopped-bg` while
 * the dead end takes hued `x` and `--step-failed-bg`; drawing the two alike
 * would invite a control that cannot work. No row carries a duration that
 * moves, and a frozen row's word is the Job's own verb — `enum-verbs.toml` has
 * no `step_state` rows, and the Job's status holds the fact that it is over.
 */
export const FourJobsOverAndOneResumable: StoryObj = {
  render: () => {
    const done: WorkflowRailStep = { id: "implement", label: "Implement", activity: "advanced", status: "advanced", elapsed: "6m 48s" };
    const failing = { command: "test · cargo test --workspace", result: "failed · exit 0 → exit 101", icon: ShieldX, iconLabel: "failed", outputPath: ".armada/jobs/job_91ab/checks/test.log" };
    // The Job's status, what it means for the step, and the step as `railOf` builds it there.
    const rails: [string, string, WorkflowRailStep][] = [
      ["escalated", "over, and resumable — a redirect resumes exactly this step",
        { id: "verify", label: "Run tests", activity: "stopped", status: "stopped", current: true, elapsed: "12m 30s", gates: [failing] }],
      ["completed_failed", "over, and a dead end. Nothing resumes it",
        { id: "verify", label: "Run tests", activity: "failed", status: "failed", current: true, elapsed: "12m 30s", gates: [failing] }],
      ["killed", "frozen where it stood, and unhued — an operator act carries no verdict",
        { id: "verify", label: "Run tests", activity: "killed", status: "killed", current: true, elapsed: "4m 09s",
          gates: [{ command: "test · cargo test --workspace", result: "not reached", icon: ShieldMinus, iconLabel: "not reached" }] }],
      ["completed_success", "frozen, every step advanced. The Job's status says so",
        { id: "verify", label: "Run tests", activity: "advanced", status: "done", current: true, elapsed: "9m 51s",
          gates: [{ command: "test · cargo test --workspace", result: "passed", icon: ShieldCheck, iconLabel: "passed" }] }],
    ];
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-6)" }}>
        {rails.map(([status, reads, step]) => (
          <div key={status} style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
            <span style={{ fontSize: "var(--text-2xs)", color: "var(--fg-subtle)" }}>
              <span style={{ fontFamily: "var(--font-mono)" }}>{status}</span>{` — ${reads}`}
            </span>
            <WorkflowRail steps={[done, step]} />
          </div>
        ))}
      </div>
    );
  },
};

/**
 * What a step declares, before any of it has run.
 *
 * **This is the tier a rail could not draw.** A step's Checks are mechanical, a
 * `judge_checks[]` entry is semantic, and `advance_gate` says which of them
 * actually lets the step past — so a step with three gates rendered as a step
 * with two, and `handoff`, which stops and waits for a person, rendered as a
 * step with none.
 *
 * **Counts, never questions.** `DeclaredJudge` carries how many criteria are
 * asked, how many judges answer and whether a gaming check rides along, and
 * carries no prompt at all: a question drawn here is a prompt in a screenshot.
 * `panel_size` is absent at one, so a value always means a panel.
 *
 * **`advance_gate` reads as the wire spells it.** `enum-verbs.toml` has no rows
 * for it, so `human_always` renders underscored — the fallback `step_state`
 * takes. Reported. `auto` draws no row: the Checks above are the whole gate, and
 * a row on every step would displace the sentence an ungated step owes.
 */
export const WhatAStepDeclares: Story = {
  args: {
    steps: [
      { id: "scope", label: "scope", labelIsAnIdentifier: true, activity: "advanced", status: "advanced", elapsed: "1m 12s", evidence: { label: "" } },
      { id: "implement", label: "implement", labelIsAnIdentifier: true, activity: "not_started", status: "not_started",
        declarations: [{ label: "judge · 2 criteria · panel of 3", result: "not reached" }, { label: "advance_gate · auto_if_judge_passes" }] },
      { id: "tests", label: "tests", labelIsAnIdentifier: true, activity: "not_started", status: "not_started",
        declarations: [{ label: "judge · gaming check", result: "not reached" }] },
      { id: "handoff", label: "handoff", labelIsAnIdentifier: true, activity: "not_started", status: "not_started",
        declarations: [{ label: "advance_gate · human_always" }] },
    ],
  },
};
