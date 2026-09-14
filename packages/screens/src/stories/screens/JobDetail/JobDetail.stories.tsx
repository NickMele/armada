import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, waitFor, within } from "storybook/test";

import type { JobSummary, ServerState } from "@armada/protocol";
import { DockQuestions } from "@armada/components";
import { takenNotice, type Taken } from "../../../freeze";
import { TakenNotice } from "../../../Taken";
import { dockQuestionsOf } from "../../../dock-questions";
import type { Outstanding } from "../../../outstanding";
import type { JobFixture } from "../../../fixtures/fixture";
import { propsFor } from "../../../fixtures/props";
import {
  awaitingApproval,
  awaitingAttestation,
  awaitingRepair,
  completedFailed,
  completedSuccess,
  escalatedBlockedByPolicy,
  escalatedEvidenceSuspect,
  escalatedGateFailure,
  escalatedInterrupted,
  escalatedLoopCap,
  escalatedNoReport,
  escalatedSilent,
  gateChecksStreaming,
  killed,
  piloted,
  preparing,
  queued,
  reading,
  rejected,
  retryingCheckFailure,
  review,
  reviewAtDelivery,
  running,
  runningAtGate,
  runningWaitingOnACommand,
  superseded,
  unreadable,
} from "../../../fixtures/build/index";
import { JOB_ID, repository, spend, watchedRead } from "../../../fixtures/build/base";
import { WAITING_CALL } from "../../../fixtures/build/running";
import { recorded } from "../../../fixtures/recorded";
import { JobDetailFrom } from "./JobDetail";
import {
  awaitingApprovalPlanPending,
  PLAN_PARTWAY,
  PLAN_WITH_A_DROPPED_TASK,
  withPlan,
} from "./plan-fixtures";

/**
 * Job detail in every state a Job can be in, drawn by the app's own screen from
 * wire data.
 *
 * A recorded story is a real Job, captured off a Fleet by
 * `scripts/record-job.mjs` and replayed through the fold Bridge's main process
 * runs. A built story is composed in code from the wire types, for states a
 * real Job rarely sits in long enough to catch. The type checker keeps those
 * honest: a field Fleet stops sending stops compiling here.
 *
 * Every region is live, so a story can be clicked through like the app. A press
 * that would change the Job does nothing, because there is no Fleet behind it.
 */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

/** A story that draws one fixture, with no controls. A whole Job is not an arg. */
function drawing(fixture: () => JobFixture): Story["render"] {
  return () => <JobDetailFrom fixture={fixture()} />;
}

/** A real Job that landed. Its worktree was reclaimed, so there is no diff to read. */
export const DoneRecorded: Story = {
  name: "Done (recorded)",
  render: drawing(() => recorded("done-worktree-given-back")),
};

/** The recorded Job at the narrowest window. The title and the job id stack, and the acts are one control. */
export const DoneRecordedNarrow: Story = {
  name: "Done (recorded), narrow window",
  render: () => (
    <JobDetailFrom fixture={recorded("done-worktree-given-back")} width="var(--window-floor)" />
  ),
};

/** The header's one control with its menu open: the act that leads on its face, the rest behind it. */
export const HeaderActionsOpen: Story = {
  name: "Header actions open",
  render: drawing(() => recorded("done-worktree-given-back")),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(
      await canvas.findByRole("button", { name: "Everything else this job can do" }),
    );
    await expect(await within(document.body).findByRole("menuitem", { name: /record/i })).toBeVisible();
  },
};

/** Midway through Fix, before its Check has run. */
export const Running: Story = { name: "Running", render: drawing(running) };

/** The Plan region, partway done — one segment past, one working, one open. */
export const PlanPartwayDone: Story = {
  name: "Plan, partway done",
  // `whereOpen` defaults to `false` in `propsFor` — Fleet's own preference,
  // closed until it says otherwise, `#927`.
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Plan")).toBeVisible();
    await expect(canvas.getByText("1 of 3")).toBeVisible();
    await expect(canvas.getByText("T1")).toBeVisible();
    await expect(canvas.getByText("Re-point the reducer's own import at it")).toBeVisible();
    const where = canvas.getByRole("button", { expanded: false, name: /Where things are/i });
    await expect(within(where).getByText("fix/settings-split-selectors")).toBeVisible();
  },
};

/** The same moment, `whereOpen` set directly — the preference's own terms. */
export const PlanPartwayDoneWhereOpen: Story = {
  name: "Plan, partway done, Where things are open",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} on={{ whereOpen: true }} />,
};

/** A dropped task stays on the list, struck through, with its reason. */
export const PlanWithADroppedTask: Story = {
  name: "Plan, with a dropped task",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_WITH_A_DROPPED_TASK)} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The existing integration test already exercises this path.")).toBeVisible();
    await expect(canvas.getByText("1 of 3")).toBeVisible();
    // Already dropped, so dropping it again is not offered — #897.
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const droppedRow = found.closest("li");
    if (droppedRow === null) throw new Error("the dropped row was not found");
    await expect(within(droppedRow).queryByRole("button", { name: "Drop…" })).toBeNull();
  },
};

/** A workflow with no plan step draws no Plan region. */
export const NoPlan: Story = {
  name: "No plan on the workflow",
  render: () => <JobDetailFrom fixture={running()} />,
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Plan")).toBeNull();
  },
};

/** Before the first Drone turn, nothing is recorded yet — same absence. */
export const BeforeThePlanStepHasRecordedOne: Story = {
  name: "Before the plan step has recorded one",
  render: () => <JobDetailFrom fixture={preparing()} />,
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Plan")).toBeNull();
  },
};

/**
 * A step declares `plan_recorded` and has not run yet — a Bug Job at the
 * approval gate, its plan step still ahead. The quiet placeholder, and no
 * task bar, figure, approach or `Add task` until a plan exists. `#1007`.
 */
export const PlanPending: Story = {
  name: "Plan, before it's recorded",
  render: () => <JobDetailFrom fixture={awaitingApprovalPlanPending()} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Plan")).toBeVisible();
    await expect(canvas.getByText("No plan yet — Plan the change records it.")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Add task" })).toBeNull();
  },
};

/** The eyebrow act, open — a one-line title and an optional detail. `#897`. */
export const PlanAddTaskOpen: Story = {
  name: "Plan, Add task open",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Add task" }));
    const dialog = within(document.body).getByRole("dialog");
    await expect(within(dialog).getByLabelText("Title")).toBeVisible();
    await expect(within(dialog).getByLabelText("Detail — optional")).toBeVisible();
    await expect(within(dialog).getByRole("button", { name: "Add task" })).toBeDisabled();
  },
};

/** `Drop…` is offered only on `open` or `working`, never `done`. `#897`. */
export const PlanDropOnRow: Story = {
  name: "Plan, Drop… on a row",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas }) => {
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const row = found.closest("li");
    if (row === null) throw new Error("the task row was not found");
    await expect(within(row).getByRole("button", { name: "Drop…" })).toBeInTheDocument();
    const done = await canvas.findByText("Extract selectColumnOrder into its own module");
    const doneRow = done.closest("li");
    if (doneRow === null) throw new Error("the done row was not found");
    await expect(within(doneRow).queryByRole("button", { name: "Drop…" })).toBeNull();
  },
};

/** An empty reason cannot be sent, and the field says why. `#897`. */
export const PlanDropReasonEmpty: Story = {
  name: "Plan, Drop reason empty",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const row = found.closest("li");
    if (row === null) throw new Error("the task row was not found");
    await userEvent.click(within(row).getByRole("button", { name: "Drop…" }));
    // Pristine: a hint, not an error — nothing has been tried yet.
    await expect(within(row).getByText("A reason is needed.")).toHaveAttribute("data-tone", "muted");
    await expect(within(row).getByRole("button", { name: "Drop" })).toBeDisabled();
  },
};

/**
 * The same field, once a submit has been tried empty — the only way there,
 * since Drop itself stays disabled and unclickable while blank. `#897`.
 */
export const PlanDropReasonAttemptedEmpty: Story = {
  name: "Plan, Drop reason tried empty",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const row = found.closest("li");
    if (row === null) throw new Error("the task row was not found");
    await userEvent.click(within(row).getByRole("button", { name: "Drop…" }));
    await userEvent.click(within(row).getByLabelText("Reason"));
    await userEvent.keyboard("{Enter}");
    await expect(within(row).getByText("A reason is needed.")).toHaveAttribute("data-tone", "error");
  },
};

/** Nothing is connected, so a send refuses and the dialog stays open with what was typed. `#897`. */
export const PlanDropRefused: Story = {
  name: "Plan, Drop refused",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const row = found.closest("li");
    if (row === null) throw new Error("the task row was not found");
    await userEvent.click(within(row).getByRole("button", { name: "Drop…" }));
    await userEvent.type(within(row).getByLabelText("Reason"), "Already covered elsewhere.");
    await userEvent.click(within(row).getByRole("button", { name: "Drop" }));
    await expect(
      await within(row).findByText("Fleet is not connected. Nothing was sent."),
    ).toBeVisible();
    await expect(within(row).getByLabelText("Reason")).toHaveValue("Already covered elsewhere.");
  },
};

/** Nothing is connected, so a real send refuses and the dialog stays open with what was typed. `#897`. */
export const PlanAddTaskRefused: Story = {
  name: "Plan, Add task refused",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Add task" }));
    const dialog = within(document.body).getByRole("dialog");
    await userEvent.type(within(dialog).getByLabelText("Title"), "Add a regression test");
    await userEvent.click(within(dialog).getByRole("button", { name: "Add task" }));
    await expect(
      await within(dialog).findByText("Fleet is not connected. Nothing was sent."),
    ).toBeVisible();
    await expect(within(dialog).getByLabelText("Title")).toHaveValue("Add a regression test");
  },
};

/** What the panel's choice sends. A module's spy, cleared by the play that reads it. */
const setWhenBlocked = fn();

/**
 * The running Job, changed: a model chosen for its later steps and one command
 * allowed for it. What the header counts, and what the panel lists.
 */
function runningWithSettings(): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  return {
    ...fixture,
    watched: watchedRead({
      ...fixture.watched.detail,
      model_override: "opus",
      allowed_commands: [
        {
          run: "pnpm add -D reselect@5.1.1",
          reach: "job",
          allowed_at: "2026-09-10T14:29:40Z",
          by: "human",
        },
      ],
      // Fleet's own table, since protocol 13.5 — covers every job against
      // this repository, so the panel draws it read-only. #836's own case:
      // `gh issue view`, always-allowed from Job 7 while filing #834.
      repository_allowed_commands: [
        {
          run: "gh issue view",
          reach: "repository",
          allowed_at: "2026-09-13T09:41:00Z",
          by: "human",
        },
      ],
    }),
  };
}

/**
 * **The header's way in, and the panel it opens.** The count on the button is
 * the model and the allow; the panel says what each setting does and when a
 * change to it takes. Nothing on the screen behind it restarts.
 *
 * **What goes is this Job's id and the wire's word.** The panel's own story
 * proves a choice names `ask_me`; this proves the screen hands it on against the
 * Job on screen, which is the half a composition cannot show. Broken once by
 * sending the Job's handle rather than its id.
 */
export const JobSettingsOpen: Story = {
  name: "Job settings open",
  render: () => (
    <JobDetailFrom fixture={runningWithSettings()} on={{ onSetWhenBlocked: setWhenBlocked }} />
  ),
  play: async ({ canvas, userEvent }) => {
    setWhenBlocked.mockClear();
    await userEvent.click(await canvas.findByRole("button", { name: /^Job settings/ }));
    const panel = within(await canvas.findByRole("dialog", { name: "Job settings" }));
    await userEvent.click(panel.getByRole("radio", { name: "Ask me first" }));
    await expect(setWhenBlocked).toHaveBeenCalledWith(JOB_ID, "ask_me");

    // The repository-wide row: read-only, and it carries no Remove — that
    // reaches every job against this repository, which is the Manifest
    // screen's act rather than this job's own settings.
    await expect(panel.getByText("gh issue view")).toBeVisible();
    await expect(panel.queryByRole("button", { name: "Remove gh issue view" })).toBeNull();
  },
};

/** What a waiting command's answer sends. A module's spy, cleared by the play that reads it. */
const answerCommand = fn();

/**
 * A Job at Ask me first whose Drone reached for a command it was not given, and
 * is waiting on a person — still `running`.
 * The command is what is asked; the answers are the three Fleet offered, each
 * saying what it commits to.
 *
 * **What goes is the call and the answer's wire name**, never the words on
 * the control. A label is copy, and Fleet takes `allow_for_job`.
 */
export const WaitingOnACommand: Story = {
  name: "Waiting on a command",
  render: () => (
    <JobDetailFrom fixture={runningWaitingOnACommand()} on={{ onAnswerCommand: answerCommand }} />
  ),
  play: async ({ canvas, userEvent }) => {
    answerCommand.mockClear();
    await userEvent.click(await canvas.findByRole("radio", { name: "Allow for this job" }));
    await userEvent.click(canvas.getByRole("button", { name: "Send this answer" }));
    // No note: an allow tells the drone everything it acts on, and the field is
    // never drawn under it.
    await expect(answerCommand).toHaveBeenCalledWith(
      JOB_ID,
      WAITING_CALL,
      "allow_for_job",
      undefined,
      undefined,
    );
  },
};

/**
 * The same command, always allowed — with the rule Fleet suggested, cut short
 * of the version pin.
 *
 * **What goes is the rule picked, never the whole command.** `pnpm` is the
 * candidate a person did not choose, and `pnpm add` is what Fleet declares.
 */
export const AlwaysAllowingWithARule: Story = {
  name: "Always allowing, with a rule",
  render: () => (
    <JobDetailFrom fixture={runningWaitingOnACommand()} on={{ onAnswerCommand: answerCommand }} />
  ),
  play: async ({ canvas, userEvent }) => {
    answerCommand.mockClear();
    await userEvent.click(
      await canvas.findByRole("radio", { name: "Always allow in this repository" }),
    );
    await expect(canvas.getByRole("radio", { name: "pnpm add" })).toBeChecked();

    await userEvent.click(canvas.getByRole("button", { name: "Send this answer" }));
    await expect(answerCommand).toHaveBeenCalledWith(
      JOB_ID,
      WAITING_CALL,
      "always_allow",
      undefined,
      "pnpm add",
    );
  },
};

/**
 * The same command, refused with a reason.
 *
 * **What goes is the call, the wire's word and the person's own words** — one
 * act, because the reason exists at the moment somebody presses reject and a
 * second surface for it is a second chance to lose it.
 */
export const RejectingWithAReason: Story = {
  name: "Rejecting a command, with a reason",
  render: () => (
    <JobDetailFrom fixture={runningWaitingOnACommand()} on={{ onAnswerCommand: answerCommand }} />
  ),
  play: async ({ canvas, userEvent }) => {
    answerCommand.mockClear();
    await userEvent.click(await canvas.findByRole("radio", { name: "Reject" }));
    await userEvent.type(
      canvas.getByLabelText("Note (optional)"),
      "we are not taking that dependency",
    );
    await userEvent.click(canvas.getByRole("button", { name: "Send this answer" }));
    await expect(answerCommand).toHaveBeenCalledWith(
      JOB_ID,
      WAITING_CALL,
      "reject",
      "we are not taking that dependency",
      undefined,
    );
  },
};

/** What the reading asks for, and what comes back. A module's spy, cleared by its play. */
const explainCommand = fn(async () => ({
  ok: true as const,
  explained: {
    explanation:
      "It adds reselect 5.1.1 to this repository as a development dependency and writes the lockfile. It reaches the network.",
    model: "haiku",
  },
}));

/**
 * Reading the command before answering it.
 *
 * **It decides nothing**, which is the second half of this play: the three
 * answers are live after the reading lands, exactly as they were before anybody
 * pressed. The model is named, because a reading is a claim and not a fact.
 */
export const ReadingACommand: Story = {
  name: "Reading a command before answering",
  render: () => (
    <JobDetailFrom fixture={runningWaitingOnACommand()} on={{ onExplainCommand: explainCommand }} />
  ),
  play: async ({ canvas, userEvent }) => {
    explainCommand.mockClear();
    await userEvent.click(
      await canvas.findByRole("button", { name: "Help me understand this command" }),
    );
    await expect(explainCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL);

    const reading = within(await canvas.findByRole("status", { name: "What this command does" }));
    await expect(reading.getByText(/adds reselect 5\.1\.1/)).toBeVisible();
    await expect(reading.getByText("haiku")).toBeVisible();
    await expect(canvas.getByRole("radio", { name: "Reject" })).toBeEnabled();
  },
};

/**
 * Before the first Drone turn, while the worktree is cut and the repository's
 * preparation commands run. Fleet's own log is all there is to read.
 */
export const Preparing: Story = { name: "Preparing", render: drawing(preparing) };

/** A Check failed on the first attempt, and the Drone is on its second. */
export const CheckFailedRetrying: Story = {
  name: "Check failed, retrying",
  render: drawing(retryingCheckFailure),
};

/**
 * One Check has reported and the next is queued behind it, with the Judge after
 * both. The wire has no outcome for a Check still running, so this is what in
 * flight looks like.
 */
export const AtTheGate: Story = { name: "At the gate", render: drawing(runningAtGate) };

/** Every Check passed, and the Judge refused two criteria. */
export const JudgeRefused: Story = { name: "Judge refused", render: drawing(escalatedEvidenceSuspect) };

/** Every Check passed and the Judge met every criterion. A person decides next. */
export const Review: Story = { name: "Review", render: drawing(review) };

/** What answering a judge question sends. A module's spy, cleared by the play that reads it. */
const answerJudge = fn();

/**
 * The same gate as `Review`, holding on a judge question instead of a clean
 * pass. The question outranks the rest of the slot: no merge answer, no
 * checks list, just the criterion and the three presses.
 */
function reviewAtAQuestion(): JobFixture {
  const fixture = review();
  if (fixture.watched.state !== "read") return fixture;
  return {
    ...fixture,
    watched: watchedRead({
      ...fixture.watched.detail,
      judge_question: {
        step_id: "regression_verify",
        criterion_id: "c1",
        question: "Does the fix address the cause the note names?",
        expected: "packages/settings/src/selectors.ts imports no store type",
        produced: "The module still imports RootState directly, behind a re-export",
        consequence: "the regression this step exists to catch can still reach the selectors",
        asked_at: "2026-09-10T14:29:40Z",
      },
    }),
  };
}

export const JudgeQuestionAtGate: Story = {
  name: "Judge question at the gate",
  render: () => (
    <JobDetailFrom fixture={reviewAtAQuestion()} on={{ onAnswerJudge: answerJudge }} />
  ),
  play: async ({ canvas, userEvent }) => {
    answerJudge.mockClear();
    await userEvent.click(
      await canvas.findByRole("button", { name: "Disagree, just this step" }),
    );
    await expect(answerJudge).toHaveBeenCalledWith(JOB_ID, "2026-09-10T14:29:40Z", "disagree_once", undefined);
  },
};

/**
 * The review moment on a Job whose names are as long as real ones get, with the
 * spend and turns a long run reaches. The header a person reported on 11 Sep
 * 2026: the facts wrap into an orphaned line, and the blocked-command setting
 * reads as the screen's main button with nothing saying what pressing it does.
 */
function reviewWithLongNames(): JobFixture {
  const fixture = reviewAtDelivery();
  if (fixture.watched.state !== "read") return fixture;
  const handle = "3-support-attaching-screenshots-and-file-sea";
  const renamed = {
    ...fixture.job,
    title: "Support attaching screenshots and file search in job context",
    handle,
    branch: `armada/${handle}`,
  };
  const whole = fixture.watched.detail;
  return {
    ...fixture,
    job: renamed,
    watched: watchedRead({
      ...whole,
      job: renamed,
      branch: renamed.branch,
      spend: spend({
        cost_micros: 29_630_000,
        cost_cap_micros: 60_000_000,
        unpriced: 2,
        turns: 580,
        turn_cap: 1000,
      }),
      ...(whole.delivery === undefined
        ? {}
        : {
            delivery: {
              ...whole.delivery,
              pull_request: "https://forge.example/armada/armada/pull/660",
            },
          }),
    }),
  };
}

/** The review header with real-length names, as reported. Kept to iterate the header against. */
export const ReviewLongNames: Story = {
  name: "Review, long names",
  render: drawing(reviewWithLongNames),
};

/** At review with a pull request open and comments on it. Merge becomes a fourth answer. */
export const ReviewWithPullRequest: Story = {
  name: "Review, pull request open",
  render: drawing(reviewAtDelivery),
};

/**
 * Merge pressed. Cancel holds focus and Enter presses whatever holds focus, so
 * Enter here cancels. A merge Bridge cannot take back never goes out by default.
 */
export const MergeConfirmation: Story = {
  name: "Merge confirmation",
  render: drawing(reviewAtDelivery),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /^Merge/ }));
    const layer = within(await canvas.findByRole("dialog"));
    await expect(layer.getByRole("button", { name: "Cancel" })).toHaveFocus();
  },
};

/** Two comments picked to send to a Drone, which makes Send live. */
export const CommentsPicked: Story = {
  name: "Comments picked",
  render: drawing(reviewAtDelivery),
  play: async ({ canvas, userEvent }) => {
    const remarks = within(
      await canvas.findByRole("region", { name: "Comments on the pull request" }),
    );
    const picks = remarks.getAllByRole("checkbox");
    const send = remarks.getByRole("button", { name: "Send to a drone" });
    await expect(send).toBeDisabled();
    await userEvent.click(picks[0]!);
    await userEvent.click(picks[1]!);
    await expect(send).toBeEnabled();
  },
};

/** A Check failed on its last attempt, and the Job stopped at the gate. */
export const StoppedAtTheGate: Story = {
  name: "Stopped at the gate",
  render: drawing(escalatedGateFailure),
};

/** A step spent its retries and holds for a person to repair it. */
export const NeedsRepair: Story = { name: "Needs repair", render: drawing(awaitingRepair) };

/** Waiting for room to run. Nothing has started. */
export const Queued: Story = { name: "Queued", render: drawing(queued) };

/** Proposed, and waiting for a person to approve it. */
export const NeedsApproval: Story = { name: "Needs approval", render: drawing(awaitingApproval) };

/** Waiting for a person to attest to what the work did. */
export const NeedsAttestation: Story = {
  name: "Needs attestation",
  render: drawing(awaitingAttestation),
};

/** A person is driving the Drone directly. */
export const Piloted: Story = { name: "Piloted", render: drawing(piloted) };

/** The Drone reached for something the policy refuses. */
export const BlockedByPolicy: Story = {
  name: "Blocked by policy",
  render: drawing(escalatedBlockedByPolicy),
};

/** Fleet stopped during a running step, and the Drone was gone when it came back. */
export const Interrupted: Story = { name: "Interrupted", render: drawing(escalatedInterrupted) };

/** The Drone stopped writing for longer than a step allows. */
export const Silent: Story = { name: "Silent", render: drawing(escalatedSilent) };

/** The step looped more times than its cap allows. */
export const LoopCap: Story = { name: "Loop cap", render: drawing(escalatedLoopCap) };

/** The Drone ended without submitting a report. */
export const NoReport: Story = { name: "No report", render: drawing(escalatedNoReport) };

/** Fleet would not answer for this Job's detail. Each region says what it could not read. */
export const FleetUnreachable: Story = { name: "Fleet unreachable", render: drawing(unreadable) };

/**
 * This Job's own detail asked for and not back yet. **What the Board already
 * holds draws at once** — the run's step names, the open step's name, every row
 * of where things are — and only what the read answers waits.
 */
export const StillReading: Story = {
  name: "Still reading",
  // Open, so its rows draw against what the Board already held — `whereOpen`
  // is Fleet's own preference and a story sets it directly, `#927`.
  render: () => <JobDetailFrom fixture={reading()} on={{ whereOpen: true }} />,
  play: async ({ canvas }) => {
    const run = canvas.getByRole("status", { name: "Reading the run" });
    await expect(within(run).getByText("Reproduction")).toBeVisible();
    await expect(canvas.getByText("Branch")).toBeVisible();
    await expect(canvas.queryByText("Reading this job.")).toBeNull();
  },
};

/** Landed. */
export const Landed: Story = { name: "Landed", render: drawing(completedSuccess) };

/** A held or escalated Job that a person closed as failed. */
export const Failed: Story = { name: "Failed", render: drawing(completedFailed) };

/** A person declined the work. That is a decision, and it is drawn as one. */
export const Rejected: Story = { name: "Rejected", render: drawing(rejected) };

/** A person stopped it. That is a decision, and it is drawn as one. */
export const Killed: Story = { name: "Killed", render: drawing(killed) };

/** Replaced by a redispatch that carries the work on as a new Job. */
export const Superseded: Story = { name: "Superseded", render: drawing(superseded) };

/**
 * The log, opened from its chapter's own control, the way a person opens it.
 * The control leaves the chapter once its sheet is open, which is what this
 * checks.
 */
export const LogOpen: Story = {
  name: "Log open",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
    await expect(canvas.queryByRole("button", { name: /Open the log/ })).toBeNull();
  },
};

/** The Job's patch, opened from the Produced chapter. */
export const DiffOpen: Story = {
  name: "Diff open",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the diff/ }));
  },
};

/** The log open at the narrowest window Bridge lays out for. */
export const LogOpenNarrow: Story = {
  name: "Log open, narrow window",
  render: () => <JobDetailFrom fixture={running()} width="var(--window-floor)" />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
  },
};

/** The log open on a Job a failed Check stopped. */
export const LogOpenStopped: Story = {
  name: "Log open, stopped",
  render: drawing(escalatedGateFailure),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
  },
};

/** The failed Check's output, opened from the header act — the editor, not the sheet. */
export const CheckOutputOpen: Story = {
  name: "Check output open",
  render: drawing(escalatedGateFailure),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the output/ }));
  },
};

// #1021 — a Check's log had no end and was drawn under the rows anyway, which
// pushed every later chapter off the screen. Both stories below are the fix's
// own definition of done: a press, live or kept, opens the sheet; `Esc`
// returns to the Checks chapter; nothing is ever drawn inline.

/**
 * A kept Check's row, pressed. **The sheet, not the chapter, is what grows.**
 * Nothing between the rows and the next chapter gets any taller.
 */
export const CheckOutputRowOpensSheet: Story = {
  name: "Check output row opens the sheet, kept",
  render: drawing(escalatedGateFailure),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByRole("dialog")).toBeNull();
    // `aria-pressed` is `CheckRuns`' own row control, so this is the one that
    // opens the sheet rather than the run tree's gate row, which names the
    // same file for copying into a shell and carries no pressed state at all.
    await userEvent.click(
      await canvas.findByRole("button", {
        name: "regression_verify.3.cargo_nextest.log",
        pressed: false,
      }),
    );
    const body = within(document.body);
    const dialog = within(await body.findByRole("dialog", { name: "Console output" }));
    await expect(dialog.findByText(/visible_manifests_memoises/)).resolves.toBeVisible();
    // The one place the file's own name and the pressed Check agree.
    await expect(dialog.findByText("cargo_nextest — output")).resolves.toBeVisible();

    // The second exit, same as the log and diff sheets.
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(body.queryByRole("dialog")).toBeNull());
  },
};

/**
 * A running Check's row, pressed while the gate is still writing it. The
 * sheet opens on the same press and asks main to follow the log — nothing
 * about "which Check is filling the chapter" survives from before #1021,
 * because nothing fills the chapter any more.
 */
export const CheckOutputRowOpensSheetLive: Story = {
  name: "Check output row opens the sheet, live",
  render: () => (
    <JobDetailFrom fixture={gateChecksStreaming()} on={{ onFollowCheckOutput: fn() }} />
  ),
  play: async ({ canvas, userEvent }) => {
    // No recorded run for this Check yet, so there is no second control on
    // the screen naming the same file — one button, found by its text.
    await userEvent.click(await canvas.findByText("regression_verify.1.cargo_nextest.live.log"));
    const body = within(document.body);
    const dialog = within(await body.findByRole("dialog", { name: "Console output" }));
    // No `followed` state was wired for this story, so main has not answered
    // yet — which is itself the proof the sheet asked, rather than drawing
    // whatever the chapter already had.
    await expect(dialog.findByText(/Opening this Check.s log/)).resolves.toBeVisible();
  },
};

/** Pulse, read in full: the Details control on its title line opens the sheet. */
export const FullReadingOpen: Story = {
  name: "Full reading open",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /^Details/ }));
  },
};

// Journey 9's run sheet — #624. `r` opens it, one sheet replaces another, and
// a server's link never navigates.

/**
 * This repository's own `armada.yml` — Setup, Checks and Commands as they are
 * declared at the root of this repository — read as the Job's frozen
 * Manifest. A story with an empty rail draws what shipping the wire and
 * never loading it into a fixture looks like on screen, so this is the real
 * file rather than a name invented for the story.
 */
const ARMADA_RUN_SHEET_READ = {
  state: "read" as const,
  jobId: JOB_ID,
  sheet: {
    job_id: JOB_ID,
    setup: [
      { name: "bootstrap", run: "pnpm install --frozen-lockfile", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "browsers", run: "pnpm -C packages/components exec playwright install chromium --only-shell", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
    ],
    checks: [
      { name: "build", run: "cargo build --workspace --locked", narrows: true, narrow_run: "cargo build --locked -p screens", requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "test", run: "cargo nextest run --workspace --exclude acceptance", narrows: true, narrow_run: "cargo nextest run -p screens", requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "typecheck", run: "pnpm typecheck", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "bridge_build", run: "pnpm -C apps/desktop build", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "storybook", run: "pnpm -C packages/components build-storybook", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "bridge_test", run: "pnpm bridge-test", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "format", run: "cargo fmt --all --check", narrows: true, narrow_run: "rustfmt --check --edition 2021 packages/screens/src/JobDetail.tsx", requires: ["fmt"], expect_exit_code: 0, destructive: false, frozen: true },
    ],
    commands: [
      { name: "fmt", run: "cargo fmt --all", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "gate", run: "cargo xtask verify-foundations", narrows: false, requires: [], expect_exit_code: 0, destructive: true, frozen: true },
    ],
    manifest_edited_at: "2026-09-08T16:40:00Z",
    worktree_on_disk: true,
    worktree_differs: false,
    drone_working: true,
  },
};

/** `r` opens the run sheet, nothing selected — Setup, Checks and Commands as
 * this repository's own `armada.yml` declares them. */
export const RunOpen: Story = {
  name: "Run sheet open",
  render: () => (
    <JobDetailFrom
      fixture={running()}
      on={{ rehearsal: { ...propsFor(running()).rehearsal, runSheet: ARMADA_RUN_SHEET_READ } }}
    />
  ),
  play: async ({ userEvent }) => {
    await userEvent.keyboard("r");
    const dialog = within(await within(document.body).findByRole("dialog", { name: "Run" }));
    await expect(dialog.findByText("bridge_test")).resolves.toBeVisible();
  },
};

/** Opening the run sheet while the log is open replaces it — one sheet at a time. */
export const RunReplacesLog: Story = {
  name: "Run sheet replaces the log",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
    await expect(within(document.body).findByRole("dialog", { name: "Activity log" })).resolves.toBeVisible();

    await userEvent.keyboard("r");
    await expect(within(document.body).findByRole("dialog", { name: "Run" })).resolves.toBeVisible();
    expect(within(document.body).queryByRole("dialog", { name: "Activity log" })).toBeNull();
  },
};

/** A server this Job started, serving. Its link hands the address to the
 * system browser rather than navigating — the design system's hard rule. */
const SERVING: ServerState = {
  id: "srv-storybook",
  name: "storybook",
  job_id: propsFor(running()).job.id,
  phase: "serving",
  serve: "pnpm storybook",
  ports: [{ name: "PORT", port: 41207 }],
  links: [{ url: "http://localhost:41207", name: "Storybook" }],
  started_by: "person",
  started_at: "2026-09-10T18:00:00Z",
  serving_since: "2026-09-10T18:00:05Z",
  stopped: false,
  log: "run/storybook.log",
};

const openServerLink = fn(async () => ({ ok: true }) as const);

/** The sheet's own reading of the Job's frozen Manifest, naming the Check
 * `escalatedGateFailure` failed — `cargo_nextest` — so selecting it has a row
 * to select. */
const RUN_SHEET_READ = {
  state: "read" as const,
  jobId: JOB_ID,
  sheet: {
    job_id: JOB_ID,
    setup: [],
    checks: [
      {
        name: "cargo_nextest",
        run: "cargo nextest run --workspace",
        narrows: false,
        requires: [],
        expect_exit_code: 0,
        destructive: false,
        frozen: true,
      },
    ],
    commands: [],
    worktree_on_disk: true,
    worktree_differs: false,
    drone_working: false,
  },
};

/** A refused Check's `Run it here` opens the run sheet with that Check selected. */
export const RunItHere: Story = {
  name: "Run it here selects the Check",
  render: () => (
    <JobDetailFrom
      fixture={escalatedGateFailure()}
      on={{ rehearsal: { ...propsFor(escalatedGateFailure()).rehearsal, runSheet: RUN_SHEET_READ } }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Run it here" }));
    const dialog = within(await within(document.body).findByRole("dialog", { name: "Run" }));
    await expect(dialog.findByRole("button", { current: true })).resolves.toHaveTextContent("cargo_nextest");
  },
};

/** `test` running, streaming its output as it prints. */
export const RunStreaming: Story = {
  name: "Running test, streaming",
  render: () => (
    <JobDetailFrom
      fixture={escalatedGateFailure()}
      on={{
        rehearsal: {
          ...propsFor(escalatedGateFailure()).rehearsal,
          runSheet: {
            ...RUN_SHEET_READ,
            sheet: {
              ...RUN_SHEET_READ.sheet,
              running: {
                id: "run-1",
                job_id: JOB_ID,
                name: "cargo_nextest",
                command: "cargo nextest run --workspace",
                narrowed: false,
                started_at: "2026-09-11T14:05:00Z",
              },
            },
          },
          runFollowed: {
            state: "following",
            jobId: JOB_ID,
            runId: "run-1",
            name: "cargo_nextest",
            path: ".armada/runs/run-1/output.log",
            fromLine: 1,
            lines: ["running 2034 tests", "test settings::selectors::visible_manifests_memoises ... FAIL"],
          },
        },
      }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Run it here" }));
    await expect(
      within(await within(document.body).findByRole("dialog", { name: "Run" })).findByText(/FAIL/),
    ).resolves.toBeVisible();
  },
};

export const ServingRow: Story = {
  name: "Serving row, link opens the system browser",
  // Open, so the Serving row it holds is drawn — `whereOpen` is Fleet's own
  // preference and a story sets it directly, `#927`.
  render: () => (
    <JobDetailFrom
      fixture={running()}
      on={{
        whereOpen: true,
        rehearsal: {
          ...propsFor(running()).rehearsal,
          servers: { servers: [SERVING] },
          onOpenServerLink: openServerLink,
        },
      }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    openServerLink.mockClear();
    const link = await canvas.findByRole("button", { name: "Storybook" });
    await userEvent.click(link);
    await expect(openServerLink).toHaveBeenCalledWith(SERVING.id, SERVING.links[0]!.url);
  },
};

/** A fixture with its row, and the detail's copy of it, changed alike. */
function withRow(fixture: JobFixture, over: Partial<JobSummary>): JobFixture {
  const job = { ...fixture.job, ...over };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  return { ...fixture, job, watched: watchedRead({ ...fixture.watched.detail, job }) };
}

/** Approved, and held because the repository it would land in is frozen. */
export const FrozenQueued: Story = {
  name: "Frozen, queued",
  render: drawing(() => withRow(queued(), { queued_reason: "frozen", frozen_by: ["armada"] })),
  play: async ({ canvas, canvasElement }) => {
    await expect(await canvas.findByText("Frozen")).toBeVisible();
    await waitFor(() => expect(canvasElement.textContent).toMatch(/Waits for\s*armada\s*to unfreeze/));
  },
};

/** At review in a frozen repository. The status is review's own, and the header says nothing lands. */
export const FrozenAtReview: Story = {
  name: "Frozen, at review",
  render: drawing(() => withRow(reviewAtDelivery(), { frozen_by: ["armada"] })),
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(canvasElement.textContent).toMatch(/Nothing lands until\s*armada\s*unfreezes/));
  },
};

/** Merge pressed and confirmed while frozen: taken, waiting, and never drawn as a refusal. */
function MergeTakenWhileFrozenDrawn() {
  const fixture = withRow(reviewAtDelivery(), { frozen_by: ["armada"] });
  const [taken, setTaken] = useState<Taken | null>(null);
  const notice = takenNotice(taken, fixture.job);
  return (
    <JobDetailFrom
      fixture={fixture}
      above={notice === null ? null : <TakenNotice {...notice} onDismiss={() => setTaken(null)} />}
      on={{ onMergePullRequest: (jobId) => setTaken({ jobId, act: "merge", from: fixture.job.status }) }}
    />
  );
}

export const MergeTakenWhileFrozen: Story = {
  name: "Merge taken while frozen",
  render: () => <MergeTakenWhileFrozenDrawn />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /^Merge/ }));
    const layer = within(await canvas.findByRole("dialog"));
    await userEvent.click(layer.getByRole("button", { name: "Merge and take the work" }));
    await expect(await canvas.findByText("Merge taken")).toBeVisible();
    await expect(canvas.getByText(/merges when the freeze lifts/)).toBeVisible();
  },
};

// #937: the dock and this screen's own question band read the same outstanding question, so an
// answer taken on either clears both. Proved here, beside `arrivals.test.ts`'s proof that the
// event which clears the dock's copy is the same one that re-reads this screen — a person's own
// window is the account the issue was closed against, and `arrivals.test.ts` cannot show a window.

/** The Job's held command, read straight off the fixture rather than written out a second time —
 * that is the whole of what "the same outstanding question" means for this story. */
function commandOutstanding(fixture: JobFixture): Outstanding {
  const whole = fixture.watched.state === "read" ? fixture.watched.detail : undefined;
  const waiting = whole?.command_waiting;
  if (waiting === undefined) throw new Error("fixture carries no command_waiting to share");
  return { kind: "command", job_id: fixture.job.id, waiting };
}

const REPOSITORIES = [repository()];

/**
 * The band and the dock, side by side, sharing one `open` flag. Whichever side answers flips it,
 * and both sides are drawn off it — never two copies of whether the question still stands.
 */
function DockAgreesWithDetailDrawn({ answerFrom }: { answerFrom: "dock" | "detail" }) {
  const base = runningWaitingOnACommand();
  const outstanding = commandOutstanding(base);
  const [open, setOpen] = useState(true);
  const whole = base.watched.state === "read" ? base.watched.detail : undefined;
  const fixture: JobFixture =
    open || whole === undefined
      ? base
      : { ...base, watched: watchedRead({ ...whole, command_waiting: undefined }) };
  const cards = dockQuestionsOf(open ? [outstanding] : [], [fixture.job], REPOSITORIES, 0, {
    onAnswer: () => setOpen(false),
  });

  return (
    <div style={{ display: "flex", height: "100vh", width: "100%" }}>
      <div style={{ flex: 1, minWidth: 0 }}>
        <JobDetailFrom
          fixture={fixture}
          on={answerFrom === "detail" ? { onAnswerCommand: () => setOpen(false) } : {}}
        />
      </div>
      <div style={{ width: "var(--w-dock)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
        <DockQuestions questions={cards} />
      </div>
    </div>
  );
}

/** Pressing an answer in the dock's card clears the band on Job detail beside it. */
export const AnsweredInTheDockClearsJobDetail: Story = {
  name: "Answered in the dock, Job detail agrees",
  render: () => <DockAgreesWithDetailDrawn answerFrom="dock" />,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("region", { name: "A question from the drone" })).toBeVisible();
    const card = canvas.getByRole("article");
    await userEvent.click(within(card).getByRole("button", { name: "Reject" }));
    await expect(canvas.queryByRole("article")).toBeNull();
    await expect(canvas.queryByRole("region", { name: "A question from the drone" })).toBeNull();
  },
};

/** Pressing an answer in Job detail's own band clears the dock's card beside it. */
export const AnsweredOnJobDetailClearsTheDock: Story = {
  name: "Answered on Job detail, the dock agrees",
  render: () => <DockAgreesWithDetailDrawn answerFrom="detail" />,
  play: async ({ canvas, userEvent }) => {
    const band = within(canvas.getByRole("region", { name: "A question from the drone" }));
    await userEvent.click(band.getByRole("radio", { name: "Reject" }));
    await userEvent.click(band.getByRole("button", { name: "Send this answer" }));
    await expect(canvas.queryByRole("region", { name: "A question from the drone" })).toBeNull();
    await expect(canvas.queryByRole("article")).toBeNull();
  },
};

/** A refusal clears neither. Nothing here writes local state until an answer is known to have
 * taken, so a card Fleet sent back stands beside a band that never moved. */
export const AnswerRefusedStandsOnBoth: Story = {
  name: "A refused answer stands on both",
  render: () => {
    const base = runningWaitingOnACommand();
    const cards = dockQuestionsOf([commandOutstanding(base)], [base.job], REPOSITORIES, 0, {
      onAnswer: () => {},
      refusalFor: () => "Fleet is not connected. Nothing was sent.",
    });
    return (
      <div style={{ display: "flex", height: "100vh", width: "100%" }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <JobDetailFrom fixture={base} />
        </div>
        <div style={{ width: "var(--w-dock)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
          <DockQuestions questions={cards} />
        </div>
      </div>
    );
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(within(canvas.getByRole("article")).getByRole("button", { name: "Reject" }));
    await expect(canvas.getByRole("alert")).toHaveTextContent("Fleet is not connected. Nothing was sent.");
    await expect(canvas.getByRole("region", { name: "A question from the drone" })).toBeVisible();
    await expect(canvas.getByRole("article")).toBeVisible();
  },
};
