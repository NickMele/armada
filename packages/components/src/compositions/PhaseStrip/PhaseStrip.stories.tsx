import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
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
  /**
   * **Hover opens, click pins, `Escape` unpins** — three behaviours the drawing
   * asks for as one, and a run is the only place they can be seen to disagree.
   *
   * The pointer is moved off the chip before every assertion, which is not
   * ceremony. Hover and pin are two reasons the same card is open, and a press
   * leaves the cursor sitting on the chip it pressed — assert without moving
   * away and a strip that had stopped pinning altogether would pass every line
   * here.
   */
  play: async ({ canvas, userEvent }) => {
    const checks = canvas.getByRole("button", { name: /build, test/ });
    const away = canvas.getByText(/The Drone is working/);

    await userEvent.hover(checks);
    await expect(canvas.getByRole("dialog")).toBeVisible();

    // Nothing pinned it, so leaving closes it.
    await userEvent.hover(away);
    await expect(canvas.queryByRole("dialog")).toBeNull();

    // Pinned, and now it survives the pointer leaving.
    await userEvent.click(checks);
    await userEvent.hover(away);
    await expect(canvas.getByRole("dialog")).toBeVisible();

    // A card held open covers the strip it explains, and the way out should not
    // be finding the same chip again.
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog")).toBeNull();
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
    onPin: fn(),
  },
  /**
   * **The pin does not move, and that is the assertion.** A controlled strip
   * with nobody holding the other end reports and changes nothing. The failure
   * mode is a component that keeps its own copy as well — which behaves
   * correctly in every story where the caller writes back, and disagrees with
   * the caller silently everywhere else.
   *
   * `Escape` is the same rule where it is easiest to miss: it reads as a
   * dismissal rather than as a change, so it is the press most likely to reach
   * past the caller and unpin locally.
   */
  play: async ({ args, canvas, userEvent }) => {
    const judge = canvas.getByRole("button", { name: /Judge/ });

    await userEvent.click(canvas.getByRole("button", { name: "You" }));
    await expect(args.onPin).toHaveBeenCalledWith("you");
    await expect(judge).toHaveAttribute("aria-expanded", "true");

    await userEvent.keyboard("{Escape}");
    await expect(args.onPin).toHaveBeenCalledWith(null);
    await expect(judge).toHaveAttribute("aria-expanded", "true");
  },
};

/**
 * **The two tiers that used to be one chip.** Both steps are
 * `.armada/workflows/bug.json`'s: `Plan the change` declares
 * `advance_gate: auto_if_judge_passes` and can never stop for a person, and
 * `Summarise` declares `advance_gate: human_always` and has not been reached.
 * Two different facts — *this can never wait for you* and *this has not got to
 * you yet* — and they drew as one chip, `You` sitting `ahead`, with the
 * difference only in the card a hover opens. Nobody hovers a step that passed.
 *
 * **Told apart by label and glyph, never by hue.** Amber is spent on waiting on
 * you and hue below Job level exists only where `tokens/status.css` declares
 * it, so the never-asks tier reads `No one` and carries no `user-check` — the
 * one human silhouette in the set is reserved to *human required*, and this is
 * a step that requires none. Both keep the same neutral ground, because neither
 * is a state a reader should be pulled towards.
 *
 * The `play` function is the claim: one `getByRole` per name. Before the fix
 * both chips were named `You`, and an ambiguous match is what fails.
 */
export const TheHumanTierThatCanNeverAsk: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-6)" }}>
      <PhaseStrip
        label="Plan the change · advance_gate auto_if_judge_passes · advanced"
        stages={[
          { id: "instructed", label: "Instructed", state: "cleared" },
          { id: "working", label: "Working", state: "cleared" },
          { id: "submitted", label: "Submitted", state: "cleared" },
          {
            id: "checks",
            label: "artifact_exists",
            kind: "checks",
            state: "cleared",
            stands: "1 of 1 passed",
            rows: [
              {
                label: "artifact_exists · .armada/artifacts/plan.md",
                mono: true,
                state: "cleared",
                result: "exit 0",
              },
            ],
          },
          {
            id: "judge",
            label: "Judge · 2 of 2 met",
            kind: "judge",
            state: "cleared",
            stands: "2 of 2 met",
            rows: [
              {
                label: "Does this plan address the problem that was reported?",
                state: "cleared",
                result: "no objection",
              },
              {
                label: "Does this plan name a specific root cause?",
                state: "cleared",
                result: "no objection",
              },
            ],
          },
          {
            id: "you",
            label: "No one",
            kind: "human",
            state: "never",
            stands: "this step advances without a person",
            said: "The human gate, which this step's workflow does not use.",
            detail: "Nothing at this step waits for a person. Its advance gate never asks for one.",
          },
        ]}
      />

      <PhaseStrip
        label="Summarise · advance_gate human_always · not started"
        note="Nothing has reached this step yet."
        stages={[
          { id: "instructed", label: "Instructed", state: "ahead", stands: "not reached" },
          { id: "working", label: "Working", state: "ahead" },
          { id: "submitted", label: "Submitted", state: "ahead", stands: "nothing submitted" },
          { id: "you", label: "You", kind: "human", state: "ahead", stands: "not reached" },
        ]}
      />
    </div>
  ),
  play: async ({ canvas }) => {
    // Two names, two queries. `getByRole` refuses an ambiguous match, so this
    // pair failing is the regression: before the fix both chips read `You`.
    const never = canvas.getByRole("button", { name: "No one" });
    const notReached = canvas.getByRole("button", { name: "You" });

    // Said outright rather than left implied by the queries above, because
    // "these two do not render identically" is what the story is for.
    await expect(never.textContent).not.toEqual(notReached.textContent);
  },
};
