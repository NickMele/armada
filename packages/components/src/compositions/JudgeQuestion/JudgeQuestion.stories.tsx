import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { JudgeQuestion } from "./JudgeQuestion";

/**
 * A Judge criterion refused and a person is being asked about it, rather than
 * the step stopping over it — `docs/concepts/judge.md`'s asking design.
 *
 * The measured example the design closes: `declared_plan_drift` refused
 * because a test file had been reformatted, no behaviour changed, and the
 * owner's answer took ten seconds once somebody finally read it.
 */
const meta: Meta<typeof JudgeQuestion> = {
  title: "Compositions/Judge question",
  component: JudgeQuestion,
};
export default meta;

type Story = StoryObj<typeof JudgeQuestion>;

/** The drift look, refusing on a reformatted test file. */
export const DriftRefusal: Story = {
  args: {
    question:
      "This step declared where its work would be, and these paths changed outside that declaration: xtask/src/rules_stories/tests.rs. Is every one of them a change this step's own task required?",
    expected: "the step touches only what it declared",
    produced: "it also reformatted xtask/src/rules_stories/tests.rs — line breaks only, no behaviour",
    consequence: "a person reviewing this step's diff sees a file it never claimed to touch",
    onAnswer: fn(),
  },
  /** One press is the whole answer — no confirm, no second step. */
  play: async ({ args, canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Disagree, just this step" }));
    await expect(args.onAnswer).toHaveBeenCalledWith("disagree_once", undefined);
  },
};

/** A criterion on the step's own workflow, refusing on the actual fix. */
export const ACriterionTheStepDeclared: Story = {
  args: {
    question: "Does the fix address the cause the note names?",
    expected: "expired tokens refresh once rather than per request",
    produced: "the token still refreshes on every request behind the flag",
    consequence: "the rate limit this was meant to fix trips again within a day",
    onAnswer: () => {},
  },
};

/** A note travels with the answer, and never gates it. */
export const WithANote: Story = {
  args: {
    ...DriftRefusal.args,
    onAnswer: fn(),
  } as Story["args"],
  play: async ({ args, canvas }) => {
    await userEvent.type(
      canvas.getByLabelText("Note (optional)"),
      "reformatting only, no behaviour change",
    );
    await userEvent.click(canvas.getByRole("button", { name: "Always disagree" }));
    await expect(args.onAnswer).toHaveBeenCalledWith(
      "disagree_always",
      "reformatting only, no behaviour change",
    );
  },
};

/** An answer already on its way. */
export const Sending: Story = {
  args: {
    ...DriftRefusal.args,
    disabled: true,
    disabledNote: "That answer is already on its way to Fleet.",
  } as Story["args"],
};
