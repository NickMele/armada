import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, waitFor } from "storybook/test";
import { ReviewDecision, type DecisionAct } from "./ReviewDecision";

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
  // What each act does is each button's own tooltip.
  play: async ({ canvas, userEvent }) => {
    const approve = canvas.getByRole("button", { name: "Approve the work" });
    await userEvent.hover(approve);
    await waitFor(() =>
      expect(canvas.getByText("Takes the work as the drone left it.")).toBeVisible(),
    );
    await userEvent.unhover(approve);

    const reject = canvas.getByRole("button", { name: "Reject the work" });
    await userEvent.hover(reject);
    await waitFor(() =>
      expect(
        canvas.getByText("A verdict on the work, and the job ends there.", { exact: false }),
      ).toBeVisible(),
    );
  },
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

/** What should change, listed from the review and View. A listed change is not a blank note. #907. */
export const ChangesListed: Story = {
  args: {
    note: "",
    onRemoveChange: fn(),
    onRequestChanges: fn(),
    changes: [
      {
        id: "small-fix-0",
        from: "Small fix",
        text: "The store module is over 500 lines: move the migration into its own file.",
      },
      {
        id: "view-0",
        from: "From View: A busy CPU no longer delays a Job",
        text: "Keep the CPU reading in Doctor, and say in the status bar that CPU never holds a Job.",
      },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    // No count beside the label — it read as part of the sentence.
    await expect(canvas.getByText("What should change")).toBeVisible();

    const send = canvas.getByRole("button", { name: "Request changes" });
    await expect(send).toBeEnabled();
    await userEvent.click(send);
    await expect(args.onRequestChanges).toHaveBeenCalled();

    await userEvent.click(canvas.getByRole("button", { name: /^Remove The store module/ }));
    await expect(args.onRemoveChange).toHaveBeenCalledWith("small-fix-0");
    await expect(canvas.getByLabelText("Anything else the drone should know")).toBeVisible();
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

const NOTE = "Add the arm in config's loader and a test that loads one.";

/**
 * Request changes pressed, and Fleet has not answered. The pressed control
 * waits and says so; the others are off, with no sentence about a second press
 * because the control already shows the first. #1117.
 */
export const WaitingOnFleet: Story = {
  args: { note: NOTE, pending: "changes" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Requesting changes…" })).toHaveAttribute(
      "aria-busy",
      "true",
    );
    await expect(canvas.getByRole("button", { name: "Approve the work" })).toBeDisabled();
    await expect(canvas.getByRole("button", { name: "Reject the work" })).toBeDisabled();
    await expect(canvas.queryByRole("status")).toBeNull();
  },
};

/** Five seconds on and Fleet still has not answered, so the group says so. */
export const StillWaitingOnFleet: Story = {
  args: { note: NOTE, pending: "changes" },
  play: async ({ canvas }) => {
    const said = await canvas.findByRole("status", {}, { timeout: 7000 });
    await expect(said).toHaveTextContent("Still waiting on Fleet.");
  },
};

/** Stands in for the app: a press goes out, and Fleet answers or refuses it. */
function Pressing({ answer }: { answer: "answered" | "refused" }) {
  const [note, setNote] = useState(NOTE);
  const [pending, setPending] = useState<DecisionAct | undefined>(undefined);
  const [moved, setMoved] = useState(false);
  if (moved) return <p>Changes requested. The job is running again.</p>;
  return (
    <ReviewDecision
      note={note}
      onNote={setNote}
      onApprove={() => {}}
      onReject={() => {}}
      onRequestChanges={() => {
        setPending("changes");
        setTimeout(() => {
          setPending(undefined);
          setMoved(answer === "answered");
        }, 600);
      }}
      {...(pending === undefined ? {} : { pending })}
    />
  );
}

/** Fleet takes it. The Job moves on Fleet's word, so the decision goes with it. */
export const FleetAnswered: Story = {
  render: () => <Pressing answer="answered" />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Request changes" }));
    await expect(canvas.getByRole("button", { name: "Requesting changes…" })).toHaveAttribute(
      "aria-busy",
      "true",
    );
    await expect(await canvas.findByText("Changes requested. The job is running again.")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /Request/ })).toBeNull();
  },
};

/**
 * Fleet refuses it. Nothing moved, so nothing snaps back: the controls are
 * live again and the note is still there. The refusal itself is the app's
 * failure notice, not this block's.
 */
export const FleetRefused: Story = {
  render: () => <Pressing answer="refused" />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Request changes" }));
    await expect(canvas.getByRole("button", { name: "Requesting changes…" })).toBeVisible();
    const again = await canvas.findByRole("button", { name: "Request changes" });
    await expect(again).toBeEnabled();
    await expect(again).not.toHaveAttribute("aria-busy");
    await expect(canvas.getByRole("textbox")).toHaveValue(NOTE);
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
