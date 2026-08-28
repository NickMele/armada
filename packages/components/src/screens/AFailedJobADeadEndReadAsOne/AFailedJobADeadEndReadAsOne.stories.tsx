import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { CircleCheck, CircleX, File, Folder, GitBranch, OctagonAlert, Power, ShieldCheck, ShieldX, Unplug } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { AFailedJobADeadEndReadAsOne } from "./AFailedJobADeadEndReadAsOne";
import { heading, record, steps, tail } from "./fixtures";

/**
 * Journey · Read a failed Job. The screen answers, in order, what stopped it,
 * whether anything resumes it, what ran, where the work is, and what it left
 * behind.
 *
 * **A job that landed is the one nobody investigates.** This is the screen with
 * every question on it, so the record beneath it holds the moves, the turns,
 * the diff and the claims — the four things a person used to open a database, a
 * transcript on disk and the source for.
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

/**
 * **`completed_failed`, which is the state with no way back.** `restart_step`
 * and `redirect` both take an `escalated` job — `crates/fleet/src/adrift.rs`
 * refuses anything else as `NotResumable` — so the recourse line says that
 * rather than leaving a person to press a button and find out.
 *
 * The record beneath holds the four reads: the moves, the turns, the diff and
 * the claims. Only the open one is drawn, so a record nobody unfolded costs
 * nothing.
 */
export const FailedJob: Story = {
  render: function FailedJobStory() {
    const [section, setSection] = useState("moves");
    return (
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
        recourse="Nothing resumes this job. Redirect and restart both take a job a person is holding, which is an escalated one, and this job is failed. A redispatch mints a new job from the approval gate and carries none of the work over."
        record={record}
        recordValue={section}
        onRecordChange={setSection}
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
    );
  },
};

/**
 * A Job that stopped and asked, **with its drone still alive and idle**. Two
 * acts beside redispatch and kill: redirect, because the session, the
 * worktree and every step so far are still held, so a person's own words can
 * be injected as a new turn rather than starting over.
 *
 * The verb and the glyph are `escalation_reason.stalled`'s, which is what
 * Bridge reads from the generated vocabulary rather than writing here.
 *
 * **The dialog is the confirmation.** Bridge's own `JobDetail.tsx` composes
 * this same shape — a secondary button beside the split button, opening a
 * `Dialog` whose confirm is disabled until the field holds something, so a
 * blank instruction never reaches Fleet's own 422 for it.
 */
export const StoppedAndAsked: Story = {
  render: () => {
    const [open, setOpen] = useState(false);
    const [instruction, setInstruction] = useState("");
    return (
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
            actions: (
              <>
                <Button variant="secondary" onClick={() => setOpen(true)}>
                  Redirect drone
                </Button>
                <SplitButton
                  variant="destructive"
                  menuLabel="What else ends this job"
                  items={[{ label: "Kill drone, the job stays open" }]}
                >
                  Redispatch as a new job
                </SplitButton>
              </>
            ),
          }}
          why="The job stalled. Nothing runs from here without you."
          recourse="Redirect the drone. Its session, its worktree and every step so far are still held, so an instruction reaches it as a new turn at the step above. Fleet refuses a restart while a drone is alive, because a restart throws that session away. A redispatch mints a new job from the approval gate and carries none of the work over."
          steps={steps.map((step) => ({
            id: step.id,
            label: step.id,
            labelIsAnIdentifier: true,
            activity: step.activity,
            ungatedLabel: "Fleet serves no check result for this step",
            evidence: { label: "" },
          }))}
        />
        <Dialog
          open={open}
          tone="neutral"
          title="Redirect the drone on this job?"
          confirmLabel="Redirect drone"
          confirmDisabled={instruction.trim() === ""}
          onCancel={() => setOpen(false)}
          onConfirm={() => setOpen(false)}
        >
          <p>
            The instruction is sent to the drone as a new turn. The job stays at the same step,
            with the same session — nothing is spawned and nothing already done is thrown away.
          </p>
          <Textarea
            label="Instruction"
            rows={4}
            value={instruction}
            onChange={(event) => setInstruction(event.target.value)}
          />
        </Dialog>
      </div>
    );
  },
};

/**
 * The same stall, **with the drone gone** instead. Redirect no longer
 * applies — there is no session left to inject a turn into — so restart
 * takes its place: a fresh drone on the surviving worktree, at the step
 * that stopped. Never offered beside redirect on the same job, because
 * which of the two applies is decided by the drone's presence rather than
 * by the person.
 *
 * Neutral tone, not destructive — nothing here ends the job.
 */
export const StoppedWithNoDrone: Story = {
  render: () => {
    const [confirming, setConfirming] = useState(false);
    return (
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
            actions: (
              <>
                <Button variant="secondary" onClick={() => setConfirming(true)}>
                  Restart step
                </Button>
                <SplitButton
                  variant="destructive"
                  menuLabel="What else ends this job"
                  items={[{ label: "Kill job, it ends here", danger: true }]}
                >
                  Redispatch as a new job
                </SplitButton>
              </>
            ),
          }}
          why="The job stalled. Its drone is gone. Nothing runs from here without you."
          recourse="Restart the step. The drone is gone, so a fresh one takes over the worktree at the step above, resolving its toolset, model and environment again. Fleet refuses this where the worktree is no longer on disk, and Bridge does not read the filesystem, so that answer comes on the press. A redispatch mints a new job from the approval gate and carries none of the work over."
          steps={steps.map((step) => ({
            id: step.id,
            label: step.id,
            labelIsAnIdentifier: true,
            activity: step.activity,
            ungatedLabel: "Fleet serves no check result for this step",
            evidence: { label: "" },
          }))}
        />
        <Dialog
          open={confirming}
          tone="neutral"
          title="Restart this step?"
          confirmLabel="Restart step"
          onCancel={() => setConfirming(false)}
          onConfirm={() => setConfirming(false)}
        >
          A fresh drone takes over on the same worktree, at the step the last one stopped at. The
          toolset, model and environment are resolved again from scratch, so a widened scope can
          only narrow — and where the worktree itself is gone, Fleet refuses this and names a
          redispatch instead.
        </Dialog>
      </div>
    );
  },
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
          actions: (
            <>
              <Button variant="secondary">Restart step</Button>
              <Button>Redispatch as a new job</Button>
            </>
          ),
        }}
        why="failed a check · owes c2"
        recourse="Restart the step. The drone is gone, so a fresh one takes over the worktree at the step above, resolving its toolset, model and environment again. Fleet refuses this where the worktree is no longer on disk, and Bridge does not read the filesystem, so that answer comes on the press. A redispatch mints a new job from the approval gate and carries none of the work over."
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
        recourse="Nothing resumes this job. Redirect and restart both take a job a person is holding, which is an escalated one, and this job is killed. A redispatch mints a new job from the approval gate and carries none of the work over."
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

/**
 * **Escalated, and no step of it stopped.** A job-level trigger — `interrupted`,
 * `resource_exhausted`, `dependency_failed` — names no step, so neither resume
 * act has anywhere to land and `crates/fleet/src/adrift.rs` refuses both as
 * `NoStepStopped`. The header carries no redirect and no restart, and the
 * recourse line says why rather than leaving the absence to read as an
 * oversight.
 *
 * This is the fourth of the four answers the recourse line gives. The other
 * three are on the stories above: redirect, restart, and a job whose status is
 * not `escalated` at all.
 */
export const EscalatedWithNoStepToResume: Story = {
  render: () => (
    <div className="armada-screen">
      <AFailedJobADeadEndReadAsOne
        heading={{
          status: "escalated",
          statusIcon: Unplug,
          statusLabel: "interrupted",
          headline: "Cache the manifest read",
          jobId: "job_91ab",
          fields: [
            { label: "Ran", value: "3m 12s", mono: true },
            { label: "Model", value: "sonnet", mono: true },
          ],
          actions: <Button>Redispatch as a new job</Button>,
        }}
        why="interrupted"
        recourse="Nothing resumes this job. It escalated without stopping a step, so redirect and restart have no step to land on. A redispatch mints a new job from the approval gate and carries none of the work over."
        steps={steps.map((step) => ({
          id: step.id,
          label: step.label,
          activity: "not_started" as const,
          status: "not started",
        }))}
        outputAbsent="Each check names its output file on its own row. Nothing serves the contents."
        workAbsent="Nothing serves this Job's paths, its branch or its brief."
      />
    </div>
  ),
};
