import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import type { JobFixture } from "../../../fixtures/fixture";
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
  rejected,
  retryingCheckFailure,
  review,
  reviewAtDelivery,
  running,
  runningAtGate,
  superseded,
  unreadable,
} from "../../../fixtures/build/index";
import { recorded } from "../../../fixtures/recorded";
import { JobDetailFrom } from "./JobDetail";

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

/** The failed Check's output, opened from the Checks chapter. */
export const CheckOutputOpen: Story = {
  name: "Check output open",
  render: drawing(escalatedGateFailure),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the output/ }));
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
