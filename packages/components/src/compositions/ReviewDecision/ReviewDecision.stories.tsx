import type { Meta, StoryObj } from "@storybook/react-vite";
import { ReviewDecision } from "./ReviewDecision";

/**
 * The answers to a job waiting at a human gate, and the note one of them
 * carries.
 *
 * The reply field is on the surface rather than behind a control, because
 * reviewing and replying is one loop: a design that puts the reply in a
 * separate route, tab or modal from the diff is the thing `bridge.md` says to
 * push back on before it is built.
 *
 * **Reject sits below a rule and never in the group.** The others are
 * recoverable and this one ends both the job and the drone, and a person has to
 * be able to tell which before pressing.
 *
 * **Merge is drawn only where there is a pull request**, and it takes the
 * primary fill when it is. A job holding one has a single ordinary ending, and
 * it is not "record this done and leave the branch on the forge".
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
 * A job whose branch went out, so there is a pull request to merge. **Merge is
 * the primary act and Approve steps back to secondary**: approving here records
 * the job done and leaves the branch open on the forge, which is the ending the
 * merge control exists to stop being the easy one.
 *
 * Approving is still on the surface, because a person may want the job closed
 * without landing the branch — a change somebody else will carry, or one that
 * is merging by another route.
 *
 * **The press asks; it does not merge.** This is the confirmation's closed
 * state — the caller opens a dialog on `onMerge`, the way it does on
 * `onReject`, because merging writes into a repository Fleet did not make and
 * nothing in Bridge takes it back. The words are `Primitives/Dialog → Merge the
 * pull request`, and what the screen wires is proven in
 * `packages/screens/src/Decide.test.tsx`, where a screen can be mounted.
 */
export const APullRequestToMerge: Story = {
  args: {
    note: "",
    onMerge: () => {},
  },
};

/**
 * `#663`: the branch clashes with main, so Fleet would refuse the merge.
 * **Merge stays drawn and takes no fill** — a disabled control has no variant
 * left to express — **with the reason beside it**, never folded into
 * `disabledNote`, which belongs to the whole group and is not in play here:
 * Approve, Request changes and Reject all still work. The caller's own
 * conflict control is what fixes the branch; this one only says why it
 * cannot be pressed yet.
 */
export const TheBranchConflicts: Story = {
  args: {
    note: "",
    onMerge: () => {},
    mergeBlockedReason: "Resolve the conflicts first.",
  },
};

/**
 * The same, with a decision already in flight. Every control is off, including
 * the merge — the one act here that writes into a repository Armada does not
 * own, and the last one that should be pressable twice.
 */
export const AMergeAlreadySent: Story = {
  args: {
    note: "",
    onMerge: () => {},
    disabled: true,
    disabledNote: "A decision on this job is already in flight. It was not sent twice.",
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
