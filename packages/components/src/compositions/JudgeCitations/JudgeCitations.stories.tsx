import type { Meta, StoryObj } from "@storybook/react-vite";
import { JudgeCitations, type JudgeCitation } from "./JudgeCitations";

/**
 * One story per state: a panel that cited across both verdicts, a lone dissent
 * where the overlap is the whole question, and a panel that recorded nothing.
 */
const meta: Meta<typeof JudgeCitations> = {
  title: "Compositions/Judge citations",
  component: JudgeCitations,
};
export default meta;

type Story = StoryObj<typeof JudgeCitations>;

const BEHAVIOUR = "Behaviour is unchanged for every input the old code accepted";
const ENTRY = "Exactly one parsing entry point remains";

const rows: JudgeCitation[] = [
  {
    id: "chk-suite",
    who: "j1 · 02",
    criterion: BEHAVIOUR,
    where: "check:test_suite lines 2007–2008",
    named: "not_met",
    verdict: "refusal",
  },
  {
    id: "chk-suite",
    who: "j3 · 02",
    criterion: BEHAVIOUR,
    where: "check:test_suite lines 2007–2008",
    named: "not_met",
    verdict: "refusal",
  },
  {
    id: "diff-loose",
    who: "j3 · 02",
    criterion: BEHAVIOUR,
    where: "tests/loose.rs diff −14",
    named: "not_met",
    verdict: "refusal",
  },
  {
    id: "diff-mod",
    who: "j1 · 04",
    criterion: ENTRY,
    where: "src/parse/mod.rs diff +94",
    named: "met",
    verdict: "met",
  },
  {
    id: "diff-mod",
    who: "j2 · 04",
    criterion: ENTRY,
    where: "src/parse/mod.rs diff +94",
    named: "met",
    verdict: "met",
  },
];

/**
 * Every citation the panel made.
 *
 * **The met rows are not filler.** A judge that meets a criterion writes no
 * prose but still records what it read, and those rows are half of the
 * comparison that tells a lone dissent apart from an ambiguous criterion.
 */
export const EveryPointerThePanelMade: Story = {
  args: { rows, label: "Citations" },
};

/**
 * A lone dissent where every judge read the same lines. **Three rows, one
 * location** — which is what says the disagreement is about the criterion
 * rather than the work.
 */
export const OneDissentOnSharedLines: Story = {
  args: {
    label: "Citations",
    rows: [
      {
        id: "guard",
        who: "j1 · 03",
        criterion: "No additional database round trip on the rejection path",
        where: "src/dispatch/guard.rs lines 29–33",
        named: "met",
        verdict: "met",
      },
      {
        id: "guard",
        who: "j2 · 03",
        criterion: "No additional database round trip on the rejection path",
        where: "src/dispatch/guard.rs lines 29–33",
        named: "not_met",
        verdict: "refusal",
      },
      {
        id: "guard",
        who: "j3 · 03",
        criterion: "No additional database round trip on the rejection path",
        where: "src/dispatch/guard.rs lines 29–33",
        named: "met",
        verdict: "met",
      },
    ],
  },
};

/**
 * A panel that recorded no citations. **A named absence**, because a verdict
 * with nothing behind it is the one a person should distrust — and an empty
 * list would look like a list that had not loaded.
 */
export const NothingRecorded: Story = {
  args: {
    label: "Citations",
    rows: [],
    emptyNote:
      "This panel recorded no citations. A verdict with nothing behind it cannot be checked against what it read.",
  },
};
