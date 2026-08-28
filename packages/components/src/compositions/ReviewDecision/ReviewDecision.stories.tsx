import type { Meta, StoryObj } from "@storybook/react-vite";
import { ReviewDecision } from "./ReviewDecision";

/**
 * The three answers to a job waiting at a human gate, and the note one of them
 * carries.
 *
 * The reply field is on the surface rather than behind a control, because
 * reviewing and replying is one loop: a design that puts the reply in a
 * separate route, tab or modal from the diff is the thing `bridge.md` says to
 * push back on before it is built.
 *
 * **Reject sits below a rule and never in the group.** Two of the three are
 * recoverable and one ends both the job and the drone, and a person has to be
 * able to tell which before pressing.
 */
const meta: Meta<typeof ReviewDecision> = {
  title: "Compositions/Review decision",
  component: ReviewDecision,
  args: {
    note: "",
    onNote: () => {},
    onApprove: () => {},
    onRequestChanges: () => {},
    onReject: () => {},
  },
};
export default meta;

type Story = StoryObj<typeof ReviewDecision>;

/**
 * At rest. `Request changes` is off because the note is blank — refused before
 * the press, matching the 422 Fleet would give it rather than making a person
 * read a refusal to learn a field was empty.
 */
export const NothingWrittenYet: Story = {
  args: { note: "" },
};

/** A note written, so the reply is live and all three answers are available. */
export const ANoteWritten: Story = {
  args: {
    note:
      "The gate change is right, but AdvanceGate::HumanAlways is handled in gate.rs and not in " +
      "config's loader, so a workflow declaring it is still refused at load. Add the arm there " +
      "and a test that loads one.",
  },
};

/**
 * A decision already in flight. Every control is off, and the sentence says
 * why — a disabled group with no reason is a surface that looks broken.
 */
export const ADecisionAlreadySent: Story = {
  args: {
    note: "Add the arm in config's loader and a test that loads one.",
    disabled: true,
    disabledNote: "A decision on this job is already in flight. It was not sent twice.",
  },
};

/**
 * Fleet is not connected. The same disabled treatment and a different sentence,
 * because "already sent" and "nothing to send it over" are different things to
 * do about it.
 */
export const NotConnectedToFleet: Story = {
  args: {
    note: "",
    disabled: true,
    disabledNote: "Fleet is not connected, so nothing here can be sent.",
  },
};
