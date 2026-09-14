import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

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
  killed,
  piloted,
  preparing,
  queued,
  reading,
  rejected,
  retryingCheckFailure,
  review,
  reviewAtDelivery,
  runningAtGate,
  superseded,
  unreadable,
} from "../../../fixtures/build/index";
import { JOB_ID, spend, watchedRead } from "../../../fixtures/build/base";
import type { JobFixture } from "../../../fixtures/fixture";
import { JobDetailFrom } from "./JobDetail";
import { drawing } from "./story-helpers";

/** Job detail, split by group — #1044. Same `title` as the rest of this directory, so ids hold. */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

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
