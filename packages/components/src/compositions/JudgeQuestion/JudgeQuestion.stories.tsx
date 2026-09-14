import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent } from "storybook/test";

import { JudgeQuestion, type JudgeQuestionProps } from "./JudgeQuestion";

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

/** An answer already on its way, from somewhere this block cannot name. */
export const Sending: Story = {
  args: {
    ...DriftRefusal.args,
    disabled: true,
    disabledNote: "That answer is already on its way to Fleet.",
  } as Story["args"],
};

/** Stands in for the app: a press goes out, and Fleet answers or refuses it. */
function Pressing({ answer }: { answer: "answered" | "refused" }) {
  const [pending, setPending] = useState(false);
  const [moved, setMoved] = useState(false);
  if (moved) return <p>The step advances. Nothing else is asked about this criterion.</p>;
  const args = DriftRefusal.args as JudgeQuestionProps;
  return (
    <JudgeQuestion
      {...args}
      pending={pending}
      onAnswer={(sent) => {
        setPending(true);
        setTimeout(() => {
          setPending(false);
          setMoved(answer === "answered" && sent === "disagree_once");
        }, 600);
      }}
    />
  );
}

/**
 * The pressed answer waits and says so; the other two go off with no mark of
 * their own. #1117.
 */
export const WaitingOnFleet: Story = {
  render: () => <Pressing answer="answered" />,
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Disagree, just this step" }));
    await expect(canvas.getByRole("button", { name: "Disagreeing, just this step…" })).toHaveAttribute(
      "aria-busy",
      "true",
    );
    await expect(canvas.getByRole("button", { name: "Agree with the refusal" })).toBeDisabled();
    await expect(await canvas.findByText(/The step advances/)).toBeVisible();
  },
};

/**
 * Fleet refuses it. Nothing moved, so the controls are live again and the
 * note is still there.
 */
export const FleetRefused: Story = {
  render: () => <Pressing answer="refused" />,
  play: async ({ canvas }) => {
    await userEvent.type(canvas.getByLabelText("Note (optional)"), "reformatting only");
    await userEvent.click(canvas.getByRole("button", { name: "Agree with the refusal" }));
    await expect(canvas.getByRole("button", { name: "Agreeing…" })).toBeVisible();
    const again = await canvas.findByRole("button", { name: "Agree with the refusal" });
    await expect(again).toBeEnabled();
    await expect(again).not.toHaveAttribute("aria-busy");
    await expect(canvas.getByLabelText("Note (optional)")).toHaveValue("reformatting only");
  },
};

/** Five seconds on and Fleet still has not answered, so the block says so. */
export const StillWaitingOnFleet: Story = {
  args: { ...DriftRefusal.args, onAnswer: () => {}, pending: true } as Story["args"],
  play: async ({ canvas }) => {
    const said = await canvas.findByRole("status", {}, { timeout: 7000 });
    await expect(said).toHaveTextContent("Still waiting on Fleet.");
  },
};
