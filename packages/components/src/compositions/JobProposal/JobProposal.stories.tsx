import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { JobProposal } from "./JobProposal";
import type { ProposalGateRow } from "./ProposalGates";

/**
 * Two states, and they are one screen a press apart: the proposal while it is
 * yours to change, and the same values frozen at the moment you approved.
 * Armada still reading the request is the third and is drawn on the dispatch
 * form, where the request is.
 */
const meta: Meta<typeof JobProposal> = {
  title: "Compositions/Job proposal",
  component: JobProposal,
};
export default meta;

type Story = StoryObj<typeof JobProposal>;

const STEPS: ProposalGateRow[] = [
  {
    id: "plan",
    label: "Plan the change",
    checks: false,
    judge: true,
    you: false,
    advanceGate: "auto_if_judge_passes",
    does: "It advances unless the Judge refuses it.",
  },
  {
    id: "implement",
    label: "Implement",
    checks: true,
    judge: true,
    you: false,
    advanceGate: "auto_if_judge_passes",
    does: "Its Checks have to pass and the Judge has to decline to refuse them.",
  },
  {
    id: "tests",
    label: "Write tests",
    checks: true,
    judge: true,
    you: false,
    advanceGate: "auto_if_judge_passes",
    does: "Its Checks have to pass and the Judge has to decline to refuse them.",
    // A tick moves the gate and never what the step declares, which is the one
    // reading on this screen a person cannot get from the boxes alone.
    unmeant:
      "This step declares nothing for a Judge to read, so asking for a Judge asks for a verdict on no criteria.",
  },
  {
    id: "handoff",
    label: "Review the change",
    checks: false,
    judge: false,
    you: false,
    repositoryDecides: "review_gate",
    overridden: false,
    advanceGate: "manifest_rule:review_gate",
    does: "The repository's review_gate policy decides whether a person signs off.",
  },
];

const ALWAYS_LOOKS =
  "Whatever is ticked, Fleet checks that the work stayed inside what the plan declared, and " +
  "looks for a Check that was gamed. No tick turns that off.";

const CRITERIA = [
  {
    id: "a1",
    text: "The rail's Drones stat reads one running beside the machine's most",
    origin: "from armada/1162",
    verifiedBy: "check",
  },
  {
    id: "a2",
    text: "Pressing the stat lists the Drone's Job and step",
    origin: "from armada/1162",
    verifiedBy: "judge",
  },
];

const COMPLETE = [
  { value: "pr_merged", label: "Its pull request merges", served: false },
  { value: "delivered", label: "The step that delivers has delivered", served: true },
];

const COMMON = {
  title: "Show what is running in the Drones stat",
  workflow: "feature",
  steps: STEPS,
  alwaysLooks: ALWAYS_LOOKS,
  tiers: { difficult: "opus", medium: "sonnet", easy: null },
  models: ["haiku", "sonnet", "opus"],
  droneCap: 2,
  machineCap: 4,
  landing: {
    target: "main",
    from: "main",
    branching: "job" as const,
    completeWhen: "pr_merged",
    prMode: "ready" as const,
  },
  completeChoices: COMPLETE,
  criteria: CRITERIA,
};

/** Nothing frozen: every gate, every tier, both refs and every criterion is a person's. */
export const YoursToChange: Story = {
  args: {
    ...COMMON,
    onTitle: () => {},
    onGate: () => {},
    onOverride: () => {},
    onTiers: () => {},
    onDroneCap: () => {},
    onLanding: () => {},
    onCriterion: () => {},
  },
};

/**
 * Approved, and every value on it is what the Job runs on.
 *
 * The first criterion's issue has been edited since — the Job keeps the words
 * it froze and says so, and nothing re-reads the issue.
 */
export const ApprovedAndFrozen: Story = {
  args: {
    ...COMMON,
    steps: STEPS.map((step) =>
      step.id !== "handoff"
        ? step
        : {
            ...step,
            you: true,
            overridden: true,
            advanceGate: "human_always",
            does: "It holds at awaiting_review for you to answer, with nothing run before you read it.",
          },
    ),
    criteria: [{ ...CRITERIA[0]!, movedSince: "22 Sep 2026 at 10:02" }, CRITERIA[1]!],
    frozenAt: "22 Sep 2026 at 09:14",
  },
  // **A rule about what does not happen.** Frozen is the absence of every
  // handler, so the claim is that no control on the screen can take an answer
  // — which a rendering of it cannot show.
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    expect(canvas.queryAllByRole("checkbox")).toHaveLength(0);
    expect(canvas.queryAllByRole("combobox")).toHaveLength(0);
    expect(canvas.queryAllByRole("textbox")).toHaveLength(0);
    expect(canvas.queryAllByRole("spinbutton")).toHaveLength(0);
    expect(canvas.getByText(/This Job decides this step for itself/)).toBeVisible();
  },
};
