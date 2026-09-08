import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
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
 * to say it. **A wider card does not fix this and the card is wider now**: the
 * ceiling went from 460px to 880 on 2026-09-08 so a command sits on one line
 * with its result, and a value with no ceiling still overruns any ceiling. The
 * result wraps onto its own line under its command rather than painting across
 * the card's right edge.
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
 * **Every row that produced something opens it.** A person opens this card to
 * find out why a tier went red; making them shut it again and hunt for the same
 * Check further down the screen is the navigation the surface exists to remove.
 *
 * **The whole line is the target, and only where there is something to open.**
 * The skipped Check below has no output and stays a line of text — a control
 * that goes nowhere is worse than a value that never offered one.
 */
export const RowsThatOpenWhatTheyProduced: Story = {
  args: {
    kind: "checks",
    name: "Checks",
    state: "failed",
    stands: "1 of 3 failed",
    onOpenArtifact: fn(),
    rows: [
      {
        label: "cargo nextest run --workspace",
        mono: true,
        state: "failed",
        result: "exit 101 · 3 failures",
        opens: "chk-suite",
      },
      {
        label: "cargo build --workspace --locked",
        mono: true,
        state: "cleared",
        result: "exit 0 · 47s",
        opens: "chk-build",
      },
      {
        label: "pnpm -C packages/components build-storybook",
        mono: true,
        state: "ahead",
        result: "not run · no changed file is under packages/**",
      },
    ],
  },
  play: async ({ canvas, userEvent, args }) => {
    // Only the two Checks that ran are controls. A row with no output must not
    // be a target that refuses.
    const opens = canvas.getAllByRole("button");
    await expect(opens).toHaveLength(2);

    await userEvent.click(opens[0]);
    await expect(args.onOpenArtifact).toHaveBeenCalledWith("chk-suite");
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
 * **A panel of three, on the two criteria it answered.** A panel runs
 * independent judges against one brief, none seeing another's answer, and
 * **any single refusal refuses the criterion** — unanimity, never a vote, so
 * that a panel is a stricter veto rather than a way to grant by consensus.
 *
 * **One row per criterion, and the row says how the panel went.** The wire
 * sends one row per *call*: two criteria at `panel_size: 3` is six. Drawn
 * straight that is each criterion three times, and counted straight it reports
 * `1 of 6 refused` for a step where one criterion of two was refused.
 *
 * **`1 of 3` and `3 of 3` are different readings, which is why the number is
 * carried at all.** A criterion two judges cleared and one refused is a close
 * call worth opening; one all three refused is not.
 *
 * A step that asks a single judge says nothing about a panel and reads exactly
 * as `Judge` above does.
 */
export const JudgeAsAPanel: Story = {
  args: {
    kind: "judge",
    name: "Judge",
    state: "failed",
    stands: "1 of 2 refused",
    rows: [
      {
        label: "Selectors import without the store",
        named: "met",
        result: "no objection · 3 judges",
      },
      {
        label: "No behaviour change in the reducer",
        named: "not_met",
        result: "refused by 1 of 3",
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
 * **A human tier that can never ask, given nothing but its kind and its
 * state.** No `said`, no `detail`, no `note` — every sentence on this card is
 * the component's own, which is the whole point of the story.
 *
 * A step whose `advance_gate` is `auto` or `auto_if_judge_passes` will never
 * stop for a person. Both of the human tier's standing sentences describe a
 * tier that *can* stop one, so on this card both were false, and the closing
 * one said *waiting on you* — the exact claim #308 was filed to stop. It never
 * reached a screen only because `phases.tsx` overrode both lines at every site,
 * which is a guard that lasts until somebody adds a caller.
 *
 * Beside `You` above, the pair is the argument: same kind, same component, and
 * neither sentence retyped by a caller.
 */
export const TheHumanTierThatCanNeverAsk: Story = {
  args: {
    kind: "human",
    name: "No one",
    state: "never",
    stands: "this step advances without a person",
  },
  play: async ({ canvas }) => {
    // The refusal first, because it is what the story is for. A word rather
    // than a sentence: any rewording of the standing closer that put *waiting*
    // back on a tier that can never wait still fails here.
    expect(canvas.queryByText(/waiting/i)).toBeNull();
    await expect(canvas.getByText(/never asks for one/)).toBeVisible();
  },
};

/**
 * **A human tier that will ask and has not been reached**, given nothing but
 * its kind and its state. The sibling of the card above, and the same defect
 * one state over.
 *
 * `ahead` is un-lit, so the chip is not amber and nothing is on you — and the
 * standing closer said *Amber, not red. It is waiting on you, not broken*,
 * which describes a state this card is not in. **The state that has not been
 * reached is not the state that is waiting.** Copy that described what the
 * card will become was the alternative and it is refused: a card describing
 * what it will become is a card that is wrong now.
 *
 * The line it carries instead says why it is un-lit, which is the fact that
 * separates it from `No one` above: this gate will ask.
 */
export const TheHumanTierNotReachedYet: Story = {
  args: {
    kind: "human",
    name: "You",
    state: "ahead",
    stands: "not reached",
  },
  play: async ({ canvas }) => {
    // The same assertion as the never tier, on the same word, for the same
    // reason. The set is why the copy avoids *waiting* rather than hedging
    // it: one word, refused on every human tier that is not waiting.
    expect(canvas.queryByText(/waiting/i)).toBeNull();
    await expect(canvas.getByText(/Not amber yet/)).toBeVisible();
  },
};

/**
 * **A human tier a person has already answered**, given nothing but its kind
 * and its state. The third of the three states that carried the standing line
 * wrongly, and **the most-seen of them** — every approved step lands here.
 *
 * *It is waiting on you* was not merely the wrong tense on this card. It asks
 * again for something already given, on the one state a person reaches by
 * having acted.
 *
 * **`stands` is omitted rather than written.** `phases.tsx` passes `not
 * reached` for this state, which is wrong in its own right and is a caller's
 * bug rather than this component's, so a story asserting the closing line
 * should not draw it. Reported, not fixed here.
 */
export const TheHumanTierAlreadyAnswered: Story = {
  args: {
    kind: "human",
    name: "You",
    state: "cleared",
  },
  play: async ({ canvas }) => {
    // Three states, one absent word. A rewording of the standing closer that
    // put the claim back fails on all three at once, which is the property the
    // copy was written for.
    expect(canvas.queryByText(/waiting/i)).toBeNull();
    await expect(canvas.getByText(/does not ask twice/)).toBeVisible();
  },
};

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
