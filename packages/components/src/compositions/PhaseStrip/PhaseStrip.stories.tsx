import type { Meta, StoryObj } from "@storybook/react-vite";
import { PhaseStrip } from "./PhaseStrip";

/**
 * One story per gate shape the drawing names, plus the four states a step can
 * be standing in. Press a stage: what it is, what it is waiting on and where it
 * stands open beneath the strip.
 */
const meta: Meta<typeof PhaseStrip> = {
  title: "Compositions/Phase strip",
  component: PhaseStrip,
};
export default meta;

type Story = StoryObj<typeof PhaseStrip>;

/**
 * `auto_if_judge_passes` — the commands decide, then a model reads it. The
 * Judge tier says how many criteria it is answering, because that is what it
 * will report against.
 */
export const Working: Story = {
  args: {
    note: "The Drone is working. Nothing has been submitted, so no gate has been asked anything yet.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "current" },
      { id: "submitted", label: "Submitted", state: "ahead" },
      {
        id: "checks",
        label: "build, test",
        kind: "checks",
        state: "ahead",
        stands: "not run",
        rows: [
          { label: "cargo build --workspace --locked", mono: true, result: "not run" },
          { label: "cargo nextest run --workspace", mono: true, result: "not run" },
        ],
      },
      {
        id: "judge",
        label: "Judge · 2 criteria",
        kind: "judge",
        state: "ahead",
        stands: "not reached",
        rows: [
          { label: "Selectors import without the store", result: "not reached" },
          { label: "No behaviour change in the reducer", result: "not reached" },
        ],
      },
      { id: "you", label: "You", kind: "human", state: "ahead" },
    ],
  },
};

/**
 * A Check running. Open `build, test` and it says what a Check is — commands
 * the repository declares, which Fleet runs and the Drone never does.
 */
export const AChecksRunning: Story = {
  args: {
    openId: "checks",
    note: "The suite is running. Nothing has been asked of the Judge yet.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "cleared" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      {
        id: "checks",
        label: "build, test",
        kind: "checks",
        state: "current",
        stands: "1 of 2 · running",
        rows: [
          { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
          { label: "cargo nextest run --workspace", mono: true, result: "running · 1m 04s" },
        ],
      },
      { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead" },
      { id: "you", label: "You", kind: "human", state: "ahead" },
    ],
  },
};

/**
 * **The escalation shape worth designing for**: green commands, a refused
 * criterion, and a tier behind it that was never reached. The Judge can only
 * refuse — it never turns a failed Check into a pass.
 */
export const AJudgeRefused: Story = {
  args: {
    openId: "judge",
    note: "The suite passed and the Judge refused one criterion. The human tier behind it was never reached.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "cleared" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      { id: "checks", label: "build, test", kind: "checks", state: "cleared", stands: "2 of 2 passed" },
      {
        id: "judge",
        label: "Judge · 1 of 2 refused",
        kind: "judge",
        state: "failed",
        stands: "1 of 2 refused",
        rows: [
          { label: "Selectors import without the store", result: "met", named: "met" },
          { label: "No behaviour change in the reducer", result: "not met", named: "not_met" },
        ],
      },
      { id: "you", label: "You", kind: "human", state: "ahead" },
    ],
  },
};

/**
 * `human_always` — and then you. **Amber, not red.** Everything mechanical has
 * cleared, so a step sitting here is stopped with nothing wrong, which is the
 * one shape that must not read as a failure.
 */
export const WaitingOnYou: Story = {
  args: {
    openId: "you",
    note: "The suite passed and the Judge met both criteria. Nothing is wrong; the workflow asks for a person here.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "cleared" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      { id: "checks", label: "3 Checks", kind: "checks", state: "cleared", stands: "3 of 3 passed" },
      {
        id: "judge",
        label: "Judge · 2 of 2 met",
        kind: "judge",
        state: "cleared",
        stands: "2 of 2 met",
      },
      { id: "you", label: "You", kind: "human", state: "waiting", stands: "waiting · 2m 04s" },
    ],
  },
};

/**
 * A Check failed and the work went back to the Drone. **The tiers behind it are
 * still ahead, not cancelled** — a failing test is work, and the Drone that
 * wrote the code is what should fix it.
 */
export const ACheckFailed: Story = {
  args: {
    openId: "checks",
    note: "The Check went back to the Drone with its output. The tiers behind it are still ahead, not cancelled.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "current" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      {
        id: "checks",
        label: "test failed · fixing",
        kind: "checks",
        state: "failed",
        stands: "exit 101 · attempt 2 of 3",
        rows: [
          { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
          { label: "cargo nextest run --workspace", mono: true, result: "exit 101 · 3 failures", named: "failed" },
        ],
      },
      { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead" },
      { id: "you", label: "You", kind: "human", state: "ahead" },
    ],
  },
};

/**
 * **No gate at all — and an absent tier is not a failed tier.** A step
 * declaring no Check and no Judge draws what does advance it rather than an
 * empty gate greyed out, which reads as a gate that failed to render.
 */
export const NoGateAtAll: Story = {
  args: {
    note: "This step declares no Check and asks no Judge. Its evidence advances it, and nothing else.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "current" },
      { id: "submitted", label: "Submitted", state: "ahead" },
    ],
  },
};

/**
 * `manifest_rule: key` — the tier resolves at dispatch to a person or a machine
 * depending on the Manifest's policy. **It draws as whatever it resolved to,
 * and names the key that decided**, so two Jobs on the same workflow showing
 * different gates say why. Recorded as open in the journey; the row says which
 * key rather than leaving the difference unexplained.
 */
export const AManifestRuleGate: Story = {
  args: {
    note: "This step's gate resolved at dispatch from the Manifest's own policy.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "cleared" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      {
        id: "you",
        label: "You",
        kind: "human",
        state: "waiting",
        stands: "waiting · 41s",
        detail: "Resolved from review_policy on this Manifest when the Job was dispatched.",
      },
    ],
  },
};
