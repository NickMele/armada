import type { Meta, StoryObj } from "@storybook/react-vite";

import type { JobFixture } from "../../../fixtures/fixture";
import {
  awaitingApproval,
  awaitingAttestation,
  awaitingRepair,
  completedFailed,
  completedSuccess,
  escalatedBlockedByPolicy,
  escalatedGateFailure,
  escalatedInterrupted,
  escalatedLoopCap,
  escalatedNoReport,
  escalatedSilent,
  killed,
  piloted,
  queued,
  rejected,
  review,
  running,
  superseded,
} from "../../../fixtures/build/index";
import { recorded } from "../../../fixtures/recorded";
import { JobDetailFrom } from "./JobDetail";

/**
 * Job detail, one story per state a Job can be in — drawn by the app's own
 * screen from wire data.
 *
 * **Two sources, and both are the wire's shape.** A *recorded* story is a real
 * Job, captured off a Fleet by `scripts/record-job.mjs` and replayed through the
 * fold Bridge's main process runs. A *built* one is composed in code from the
 * wire types, for the states a real Job rarely sits in long enough to catch —
 * the type checker is what keeps it honest, since a field Fleet stops sending
 * stops compiling here. Neither hand-builds the screen's props, which is what
 * the stories before these did and why they could not show a bug in deriving
 * them.
 *
 * **Every region is live.** The run, the chapters and the sheets hold their own
 * state, so a story can be clicked through the way the app can. A press that
 * would change the Job does nothing: a story is one moment, and there is no
 * Fleet behind it to act on.
 */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

/** A story that draws one fixture, with no controls — a whole Job is not an arg. */
function drawing(fixture: () => JobFixture): Story["render"] {
  return () => <JobDetailFrom fixture={fixture()} />;
}

/** A real Job that landed, whose worktree has since been reclaimed — so no diff can be read. */
export const RecordedDoneWithItsWorktreeGivenBack: Story = {
  name: "Recorded — done, and its worktree given back",
  render: drawing(() => recorded("done-worktree-given-back")),
};

/** Mid-way through Fix, its Check not yet run. The state most of a Job's life is spent in. */
export const Running: Story = { name: "Running", render: drawing(running) };

/** Every Check passed and the Judge met every criterion; the gate is a person. */
export const WaitingOnYourReview: Story = {
  name: "Waiting on your review",
  render: drawing(review),
};

/** A Check failed on the last attempt it had, and that ended the Job. */
export const ACheckFailedAndEndedIt: Story = {
  name: "A Check failed and ended it",
  render: drawing(escalatedGateFailure),
};

/** Waiting for room to run, with nothing started. */
export const Queued: Story = { name: "Queued", render: drawing(queued) };

/** Proposed, and waiting on a person to approve it before anything starts. */
export const WaitingForApproval: Story = {
  name: "Waiting for approval",
  render: drawing(awaitingApproval),
};

/** A step spent its retries and is holding for a person to repair it. */
export const OutOfAttempts: Story = { name: "Out of attempts", render: drawing(awaitingRepair) };

/** Waiting on a person to attest to what the work did. */
export const WaitingForAttestation: Story = {
  name: "Waiting for attestation",
  render: drawing(awaitingAttestation),
};

/** A person is driving the Drone directly. */
export const Piloted: Story = { name: "Piloted", render: drawing(piloted) };

/** The Drone reached for something the policy refuses, and the refusal is on the record. */
export const BlockedByPolicy: Story = {
  name: "Blocked by policy",
  render: drawing(escalatedBlockedByPolicy),
};

/** Fleet stopped under a running step, and the Drone was gone when it came back. */
export const Interrupted: Story = { name: "Interrupted", render: drawing(escalatedInterrupted) };

/** The Drone stopped writing for longer than a step allows. */
export const WentSilent: Story = { name: "Went silent", render: drawing(escalatedSilent) };

/** The step went round more times than its cap. */
export const HitTheLoopCap: Story = { name: "Hit the loop cap", render: drawing(escalatedLoopCap) };

/** The Drone ended without submitting a report. */
export const EndedWithoutAReport: Story = {
  name: "Ended without a report",
  render: drawing(escalatedNoReport),
};

/** Landed. */
export const Landed: Story = { name: "Landed", render: drawing(completedSuccess) };

/** Over, and a system failure ended it. */
export const Failed: Story = { name: "Failed", render: drawing(completedFailed) };

/** A person declined the work. A decision, not a failure, and drawn as one. */
export const Rejected: Story = { name: "Rejected", render: drawing(rejected) };

/** A person stopped it. A decision, not a failure, and drawn as one. */
export const Killed: Story = { name: "Killed", render: drawing(killed) };

/** Replaced by a redispatch, which carries the work on under a new Job. */
export const Superseded: Story = { name: "Superseded", render: drawing(superseded) };
