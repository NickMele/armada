import type { Meta, StoryObj } from "@storybook/react-vite";
import { PhaseCard, type PhaseCardProps } from "./PhaseCard";

const meta: Meta<typeof PhaseCard> = {
  title: "Compositions/Phase card",
  component: PhaseCard,
};
export default meta;

type Story = StoryObj<typeof PhaseCard>;

const CHECKS: PhaseCardProps = {
  kind: "checks",
  name: "Checks",
  state: "current",
  stands: "1 of 2 · running",
  rows: [
    {
      label: "cargo build --workspace --locked",
      mono: true,
      state: "cleared",
      result: "exit 0 · 47s",
    },
    {
      label: "cargo nextest run --workspace",
      mono: true,
      state: "current",
      result: "running · 1m 04s",
    },
  ],
};

const CHECKS_FAILED: PhaseCardProps = {
  kind: "checks",
  name: "Checks",
  state: "failed",
  stands: "1 of 2 failed",
  rows: [
    { label: "cargo nextest run --workspace", mono: true, state: "cleared", result: "exit 0" },
    { label: "cargo build --workspace", mono: true, state: "failed", result: "exit 101" },
  ],
};

const JUDGE: PhaseCardProps = {
  kind: "judge",
  name: "Judge",
  state: "current",
  stands: "2 criteria",
  rows: [
    { label: "Selectors import without the store", state: "cleared" },
    { label: "No behaviour change in the reducer", state: "failed" },
  ],
};

const YOU: PhaseCardProps = {
  kind: "human",
  name: "You",
  state: "waiting",
  stands: "waiting 2m 04s",
  note: "Approve, or send it back with a reason. Both are recorded on the Job.",
};

/**
 * **Checks.** Commands the repository declares and Fleet runs — which is the
 * point of them, because a Drone reporting its own tests is a claim and not a
 * result.
 *
 * Each command is mono with its exit code beside it. A running command carries
 * `shield-minus`, the nearest declared member of the shield family: the
 * drawing gives it a bare shield outline and the icon registry has none.
 * Reported.
 */
export const Checks: Story = { args: CHECKS };

/**
 * **A Check that failed.** The same card, one row red — and the closing line is
 * why that is enough: a command and an exit code, nothing to interpret, and
 * the same answer every time it is run.
 */
export const AChecksFailed: Story = { args: CHECKS_FAILED };

/**
 * **A Check the step's paths never reached**, and the row that overflowed the
 * card.
 *
 * A skipped Check names the whole path set it covers, which is a value with no
 * ceiling — this one is the real Manifest's, and there is no shorter true way
 * to say it. The card is a reading measure and cannot widen, so the result
 * wraps onto its own line under its command rather than painting across the
 * card's right edge.
 *
 * **The long value is not the reported one.** A story that only proved the
 * screenshot would pass again the next time a Manifest declared a longer set.
 */
export const AChecksSkipped: Story = {
  args: {
    kind: "checks",
    name: "Checks",
    state: "cleared",
    stands: "1 of 2 passed",
    rows: [
      {
        label: "cargo nextest run --workspace",
        mono: true,
        state: "cleared",
        result: "exit 0 · 47s",
      },
      {
        label: "pnpm -C packages/components build-storybook",
        mono: true,
        state: "ahead",
        result:
          "not run · no changed file is under packages/**, package.json, pnpm-lock.yaml, " +
          "pnpm-workspace.yaml, apps/desktop/**, tsconfig.base.json",
      },
    ],
  },
};

/**
 * **The Judge.** A model reading the work against this step's acceptance
 * criteria — and never the Drone's transcript, so it cannot be argued at by
 * the thing it is judging.
 *
 * The criteria are sentences, so they are sans and they wrap. A criterion id
 * and `not_met` is a machine key and a verdict; what a person needs is the
 * sentence they were judged against.
 */
export const Judge: Story = { args: JUDGE };

/**
 * **A refusal, with its citation.** The citation is the whole persuasive
 * content of a refusal: one that cites nothing is unactionable, and one
 * quoting words that appear nowhere in the material is a failed call rather
 * than a verdict.
 *
 * The override dialog has read citations in full since it was built. The panel
 * you reach it from should not be weaker than the dialog it opens.
 */
export const JudgeRefusedAndWhy: Story = {
  args: {
    kind: "judge",
    name: "Judge",
    state: "failed",
    stands: "1 of 2 refused",
    rows: [
      { label: "Selectors import without the store", named: "met", result: "met" },
      {
        label: "No behaviour change in the reducer",
        named: "not_met",
        result: "not met",
        cited:
          "packages/settings/src/reducer.ts:88 — the SETTINGS_RESET branch now clears " +
          "manifests as well as columns, which it did not before this step.",
      },
    ],
  },
};

/**
 * **You.** The human gate, where the workflow asks for one.
 *
 * Everything mechanical has already cleared by the time this tier is lit, so a
 * step sitting here is stopped with nothing wrong — the one shape that must
 * not read as a failure. Amber, not red.
 */
export const You: Story = { args: YOU };

/**
 * The card as it opens off the strip — `--bg-overlay`, the strong edge, and
 * the arrow pointing back at the stage that opened it.
 *
 * A stage in the trailing half of the strip opens an `end`-aligned card, so it
 * stays inside the panel and still points at itself.
 */
export const FloatingOffTheStrip: Story = {
  args: { ...CHECKS, floating: true },
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-4) 0" }}>
        <Story />
      </div>
    ),
  ],
};

/**
 * The three side by side, the way the drawing sets them out. Flat on the panel
 * rather than floating, because the point of this rendering is comparing what
 * each tier says, not where it opens.
 */
export const TheThree: Story = {
  render: () => (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        gap: "var(--space-6)",
        alignItems: "start",
      }}
    >
      <PhaseCard {...CHECKS_FAILED} />
      <PhaseCard {...JUDGE} />
      <PhaseCard {...YOU} />
    </div>
  ),
};
