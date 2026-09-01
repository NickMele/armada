import type { Meta, StoryObj } from "@storybook/react-vite";
import { PhaseStrip } from "./PhaseStrip";

const meta: Meta<typeof PhaseStrip> = {
  title: "Compositions/Phase strip",
  component: PhaseStrip,
  decorators: [
    // The panel the strip lives in. Every tint on it is picked against
    // --bg-raised, and the card that opens off it lands on the same ground.
    (Story) => (
      <div
        style={{
          width: "calc(var(--space-12) * 16)",
          padding: "var(--space-4) var(--space-6)",
          borderRadius: "var(--radius-md)",
          border: "var(--border-width) solid var(--border-default)",
          background: "var(--bg-raised)",
          // The card opens beneath the strip and has to be visible in the
          // story, not clipped by the frame around it.
          minHeight: "calc(var(--space-12) * 8)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof PhaseStrip>;

const CHECKS = [
  {
    label: "cargo build --workspace --locked",
    mono: true,
    state: "cleared" as const,
    result: "exit 0 · 47s",
  },
  {
    label: "cargo nextest run --workspace",
    mono: true,
    state: "current" as const,
    result: "running · 1m 04s",
  },
];

const CRITERIA = [
  { label: "Selectors import without the store", state: "cleared" as const },
  { label: "No behaviour change in the reducer", state: "cleared" as const },
];

/**
 * A step the Drone is working. Instructed cleared, Working live, and
 * everything past it ahead — no hue at all, because a stage still ahead is a
 * position and not a state.
 *
 * The connectors are what make this an order rather than a set, and `You`
 * closes it. The build stopped at the Judge, which said a step could only ever
 * be waiting on a machine.
 */
export const AllFourStates: Story = {
  args: {
    note: "The Drone is working. Nothing has been submitted, so no gate has been asked anything yet.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "current" },
      { id: "submitted", label: "Submitted", state: "ahead" },
      { id: "checks", label: "build, test", kind: "checks", state: "ahead", rows: CHECKS },
      {
        id: "judge",
        label: "Judge · 2 criteria",
        kind: "judge",
        state: "ahead",
        rows: CRITERIA,
      },
      {
        id: "you",
        label: "You",
        kind: "human",
        state: "ahead",
        cardNote: "Approve, or send it back with a reason. Both are recorded on the Job.",
      },
    ],
  },
};

/**
 * **`You` present and lit.** Everything mechanical cleared and the step is
 * stopped anyway, because this workflow asks for a person whatever the gates
 * came to.
 *
 * Amber, never red. A step sitting here is stopped with nothing wrong, which
 * is the one shape that must not read as a failure.
 */
export const WithYouPresent: Story = {
  args: {
    note: "Every Check passed and both criteria were met. This step is waiting on you.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "cleared" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      {
        id: "checks",
        label: "build, test",
        kind: "checks",
        state: "cleared",
        stands: "2 of 2 passed",
        rows: [
          { label: "cargo build --workspace --locked", mono: true, state: "cleared", result: "exit 0 · 47s" },
          { label: "cargo nextest run --workspace", mono: true, state: "cleared", result: "exit 0 · 1m 21s" },
        ],
      },
      {
        id: "judge",
        label: "Judge · 2 of 2 met",
        kind: "judge",
        state: "cleared",
        stands: "2 of 2 met",
        rows: CRITERIA,
      },
      {
        id: "you",
        label: "You",
        kind: "human",
        state: "waiting",
        stands: "waiting 2m 04s",
        cardNote: "Approve, or send it back with a reason. Both are recorded on the Job.",
      },
    ],
  },
};

/**
 * **The exact escalation worth designing for**: green commands, a refused
 * criterion, and a tier behind it that was never reached.
 *
 * The Judge stage is pinned, so the refusal and what it cites are on screen
 * without anything being pressed — a criterion id and `not_met` tells a person
 * nothing about their own Job.
 */
export const AJudgeRefused: Story = {
  args: {
    pinnedId: "judge",
    note: "The commands were fine and one criterion was refused. Nothing past it ran.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "cleared" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      {
        id: "checks",
        label: "build, test",
        kind: "checks",
        state: "cleared",
        stands: "2 of 2 passed",
        rows: CHECKS.map((row) => ({ ...row, state: "cleared" as const, result: "exit 0" })),
      },
      {
        id: "judge",
        label: "Judge · 1 of 2 refused",
        kind: "judge",
        state: "failed",
        stands: "1 of 2 refused",
        rows: [
          { label: "Selectors import without the store", state: "cleared", result: "met" },
          {
            label: "No behaviour change in the reducer",
            state: "failed",
            result: "not met",
            cited:
              "packages/settings/src/reducer.ts:88 — the SETTINGS_RESET branch now clears " +
              "manifests as well as columns, which it did not before this step.",
          },
        ],
      },
      { id: "you", label: "You", kind: "human", state: "ahead" },
    ],
  },
};

/**
 * A Check that failed. The stage is red and everything behind it is ahead:
 * nothing past a failed Check ran, and the strip shows that by leaving it
 * unlit rather than by greying it out.
 */
export const ACheckFailed: Story = {
  args: {
    pinnedId: "checks",
    note: "A command exited non-zero. The Drone is repairing it; no model has been asked anything.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "cleared" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      {
        id: "checks",
        label: "test failed · fixing",
        kind: "checks",
        state: "failed",
        stands: "1 of 2 failed",
        rows: [
          { label: "cargo build --workspace", mono: true, state: "cleared", result: "exit 0" },
          { label: "cargo nextest run --workspace", mono: true, state: "failed", result: "exit 101" },
        ],
      },
      { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", rows: CRITERIA },
      { id: "you", label: "You", kind: "human", state: "ahead" },
    ],
  },
};

/**
 * A stage in the trailing half of the strip, pinned. The card hangs off the
 * trailing edge instead of the leading one, so it stays inside the panel and
 * still points at itself.
 *
 * Decided by position rather than by measurement — there is nothing to
 * recompute when the window resizes, which is the failure this app exists to
 * escape.
 */
export const OpenedNearTheTrailingEdge: Story = {
  args: {
    ...WithYouPresent.args,
    pinnedId: "you",
  },
};

/**
 * A step that declares no command and asks no model. **An absent tier is not a
 * failed tier**, so the row says what does advance it rather than drawing an
 * empty gate.
 */
export const NoGateAtAll: Story = {
  args: {
    note: "This step declares no command and asks no model. Its evidence is what advances it.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "current" },
      { id: "submitted", label: "Submitted", state: "ahead" },
      { id: "you", label: "You", kind: "human", state: "ahead" },
    ],
  },
};

/**
 * Held by the caller. `pinnedStage` is the whole of what is pinned and clicking
 * a chip only reports — **the pin in this story does not move**, because that
 * is what a controlled component does when nobody holds the other end. Hover
 * still opens a card: hovering is a reading of the pointer's position, not a
 * held decision, and a caller holding it would be told about every crossing.
 *
 * It exists for a keyboard map that has to open a stage by id. The alternative
 * it replaces is a caller reaching into the DOM for
 * `button.armada-phases__control`, which works until this component renames a
 * class.
 */
export const HeldByTheCaller: Story = {
  args: {
    ...WithYouPresent.args,
    pinnedStage: "judge",
  },
};
