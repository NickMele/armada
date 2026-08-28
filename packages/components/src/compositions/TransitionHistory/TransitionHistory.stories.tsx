import type { Meta, StoryObj } from "@storybook/react-vite";
import { TransitionHistory } from "./TransitionHistory";

/**
 * Every move a Job made, in order, so a Job that ended somewhere surprising can
 * be read rather than guessed at.
 *
 * The log is already ordered, already append-only and already validated — the
 * fold refuses a history the machine would not admit. So this draws what
 * arrived and recomputes none of it.
 */
const meta: Meta<typeof TransitionHistory> = {
  title: "Compositions/Transition history",
  component: TransitionHistory,
};
export default meta;

type Story = StoryObj<typeof TransitionHistory>;

const NOTE = "What Armada did. What the drone said is in its turns.";

const NOTHING_YET =
  "This job has not moved yet. Creation is not a transition, so no row describes it.";

/** A Job that ran clean: approved, dispatched, three steps, done. */
export const AJobThatRanClean: Story = {
  args: {
    note: NOTE,
    emptyNote: NOTHING_YET,
    moves: [
      { seq: 1, at: "09:14:02", kind: "status", moved: "awaiting_approval → queued", actor: "human" },
      { seq: 2, at: "09:14:02", kind: "status", moved: "queued → running", actor: "fleet" },
      { seq: 3, at: "09:14:03", kind: "drone", subject: "drn_01M13", moved: "drone_spawned", actor: "fleet" },
      { seq: 4, at: "09:14:03", kind: "step", subject: "plan", moved: "not_started → running", actor: "fleet" },
      { seq: 5, at: "09:21:41", kind: "step", subject: "plan", moved: "running → advanced", actor: "fleet" },
      { seq: 6, at: "09:21:41", kind: "step", subject: "implement", moved: "not_started → running", actor: "fleet" },
      { seq: 7, at: "10:02:18", kind: "step", subject: "implement", moved: "running → advanced", actor: "fleet" },
      { seq: 8, at: "10:02:19", kind: "drone", subject: "drn_01M13", moved: "drone_exited", actor: "fleet" },
      { seq: 9, at: "10:02:19", kind: "status", moved: "running → completed_success", actor: "fleet" },
    ],
  },
};

/**
 * A Job that ended somewhere surprising — the case this exists for. Two Drones,
 * a step retried, a refusal, and a kill. The sequence is what says which of
 * those caused which.
 */
export const AJobThatEndedSomewhereSurprising: Story = {
  args: {
    note: NOTE,
    emptyNote: NOTHING_YET,
    moves: [
      { seq: 1, at: "13:40:11", kind: "status", moved: "awaiting_approval → queued", actor: "human" },
      { seq: 2, at: "13:40:11", kind: "status", moved: "queued → running", actor: "fleet" },
      { seq: 3, at: "13:40:12", kind: "drone", subject: "drn_01M2A", moved: "drone_spawned", actor: "fleet" },
      { seq: 4, at: "13:40:12", kind: "step", subject: "implement", moved: "not_started → running", actor: "fleet" },
      {
        seq: 5,
        at: "13:58:04",
        kind: "step",
        subject: "implement",
        moved: "running → stopped",
        why: "refused by the judge",
        actor: "fleet",
      },
      {
        seq: 6,
        at: "13:58:04",
        kind: "status",
        moved: "running → escalated",
        why: "refused by the judge · owes c-2, c-4",
        actor: "fleet",
      },
      { seq: 7, at: "14:06:33", kind: "drone", subject: "drn_01M2A", moved: "drone_exited", actor: "human" },
      { seq: 8, at: "14:11:50", kind: "step", subject: "implement", moved: "stopped → retrying", actor: "human" },
      { seq: 9, at: "14:11:51", kind: "drone", subject: "drn_01M2F", moved: "drone_spawned", actor: "fleet" },
      { seq: 10, at: "14:11:51", kind: "status", moved: "escalated → running", actor: "human" },
      {
        seq: 11,
        at: "14:44:09",
        kind: "status",
        moved: "running → escalated",
        why: "hit the iteration cap",
        actor: "fleet",
      },
      { seq: 12, at: "14:52:00", kind: "drone", subject: "drn_01M2F", moved: "drone_exited", actor: "fleet" },
      { seq: 13, at: "14:52:00", kind: "status", moved: "escalated → killed", actor: "human" },
    ],
  },
};

/**
 * Two moves inside one millisecond, carrying the same instant. **This is why
 * `seq` orders the list and `at` never does** — sorted on the instant, these
 * two could come back either way round, and which of them caused the other is
 * the whole question.
 */
export const TwoMovesInOneInstant: Story = {
  args: {
    note: NOTE,
    emptyNote: NOTHING_YET,
    moves: [
      { seq: 41, at: "16:02:57", kind: "step", subject: "verify", moved: "running → advanced", actor: "fleet" },
      { seq: 42, at: "16:02:57", kind: "status", moved: "running → awaiting_review", actor: "fleet" },
    ],
  },
};

/** A Job created and never moved. Ordinary, and never an error. */
export const NothingRecordedYet: Story = {
  args: { moves: [], emptyNote: NOTHING_YET },
};
