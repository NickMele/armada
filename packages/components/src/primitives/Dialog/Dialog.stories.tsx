import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn } from "storybook/test";
import { Textarea } from "../Textarea/Textarea";
import { Dialog } from "./Dialog";

const meta: Meta<typeof Dialog> = {
  title: "Primitives/Dialog",
  component: Dialog,
};
export default meta;

type Story = StoryObj<typeof Dialog>;

/**
 * The state the contract names: every destructive action confirms, and the
 * confirmation states what happens and what survives. Cancel holds initial
 * focus. The action keeps its name — this button says Kill and produces
 * "Killed".
 */
export const Confirmation: Story = {
  args: {
    open: true,
    tone: "destructive",
    title: "Kill the drone on job 12?",
    confirmLabel: "Kill job",
    cancelLabel: "Cancel",
    children:
      "Step 3 of 5, 18 minutes in. The worktree is left in place and evidence carries forward if you redispatch.",
    onConfirm: fn(),
    onCancel: fn(),
  },
  /**
   * **The keyboard contract, run rather than described**, and it is the half
   * the contract used to leave to an implementation. Cancel holds initial
   * focus; `Enter` fires the focused control; so `Enter` cancels, and nothing
   * destructive is ever one keystroke from a focused row.
   *
   * The regression this catches is the one that was here: a `window` handler
   * that confirmed past focused Cancel, which passes any assertion written as
   * "Enter did something" and ends a job the person was declining to end. So
   * both calls are asserted, not just the one that should have happened.
   */
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "Cancel" })).toHaveFocus();

    await userEvent.keyboard("{Enter}");
    await expect(args.onCancel).toHaveBeenCalled();
    await expect(args.onConfirm).not.toHaveBeenCalled();
  },
};

/**
 * `Esc` cancels too, from anywhere in the layer. Its own story rather than a
 * third line on the one above: that one is about which control `Enter` fires,
 * and this is about the way out working whatever holds focus.
 */
export const EscapeCancels: Story = {
  args: {
    open: true,
    tone: "destructive",
    title: "Kill the drone on job 12?",
    confirmLabel: "Kill job",
    children: "Step 3 of 5, 18 minutes in.",
    onConfirm: fn(),
    onCancel: fn(),
  },
  play: async ({ args, userEvent }) => {
    await userEvent.keyboard("{Escape}");
    await expect(args.onCancel).toHaveBeenCalled();
    await expect(args.onConfirm).not.toHaveBeenCalled();
  },
};

/**
 * The second dialog the component sheet draws. Not a state the contract names
 * — it is read off the sheet's Kill & Redispatch drawing, which puts the
 * escalated glyph on the head and an accent fill on the confirm even though
 * the action is destructive twice over. The disagreement is in the report.
 */
export const NeutralConfirm: Story = {
  args: {
    open: true,
    tone: "neutral",
    title: "Kill drone 4 and dispatch a replacement",
    confirmLabel: "Kill and redispatch",
    cancelLabel: "Cancel",
    children:
      "Ends job 12 and opens a new job on the same workspace and branch. The worktree is kept.",
  },
};

/**
 * The confirmations Bridge actually shows, in the words it uses.
 *
 * Each states **what happens and what survives**, which is the two halves the
 * copy contract asks for — and it is why the two kills cannot share one dialog:
 * they survive differently. Neither names a step or an elapsed the way the
 * contract's sample line does, because neither is a fact the dialog holds; the
 * detail behind it carries both and stays on screen.
 *
 * Redirect is not among them. Its dialog carries the instruction itself and is
 * its own confirmation — see `Screens/A failed job` for that shape.
 */
export const KillTheDrone: Story = {
  args: {
    open: true,
    tone: "destructive",
    title: "Kill the drone on this job?",
    confirmLabel: "Kill drone",
    children:
      "The process stops and the job stays open. Its worktree is held as the drone left it, " +
      "so the job can be redispatched from where it got to.",
  },
};

/** The other kill. Terminal, and it says so rather than implying a pause. */
export const KillTheJob: Story = {
  args: {
    open: true,
    tone: "destructive",
    title: "Kill this job?",
    confirmLabel: "Kill job",
    children:
      "The job ends at killed. That is terminal and carries no verdict — nothing resumes it, " +
      "and anything the drone wrote stays on its branch.",
  },
};

/**
 * Redispatch. **Two acts in one press**, and the copy says so.
 *
 * It disagrees with `Neutral confirm` above, which is read off the sheet's Kill
 * & Redispatch drawing: that copy says the replacement opens "on the same
 * workspace and branch". `crates/ipc/src/job.rs` says otherwise — the
 * replacement is minted at the approval gate carrying `redispatched_from`, and
 * the failed Job's worktree and branch are left as its drone left them, with
 * nothing stated about the new Job reusing either. The wire wins; the
 * disagreement is in the report.
 *
 * Destructive rather than neutral for the same reason: the sheet fills the
 * confirm with the accent on an act that ends a Job, and a redispatch ends one.
 */
export const RedispatchAsANewJob: Story = {
  args: {
    open: true,
    tone: "destructive",
    title: "Redispatch this job as a new one?",
    confirmLabel: "Redispatch as a new job",
    children:
      "This job is killed and a replacement is created carrying a reference back to it. " +
      "Nothing resumes: the new job starts at the approval gate and needs releasing. " +
      "The failed job's worktree and branch are left as its drone left them.",
  },
};

/**
 * Restart the step. **Neutral, not destructive** — nothing ends, a fresh
 * drone resumes the step that stopped on the same worktree. Offered only
 * where the escalated job's drone is gone; where one is alive, redirect is
 * the act, and it confirms through its own dialog rather than this one.
 */
export const RestartTheStep: Story = {
  args: {
    open: true,
    tone: "neutral",
    title: "Restart this step?",
    confirmLabel: "Restart step",
    children:
      "A fresh drone takes over on the same worktree, at the step the last one stopped at. " +
      "The toolset, model and environment are resolved again from scratch, so a widened scope " +
      "can only narrow — and where the worktree itself is gone, Fleet refuses this and names a " +
      "redispatch instead.",
  },
};

/**
 * **More than fits, and the controls still reachable.** The body is the one
 * region that gives: the title is fixed above it, the field and the two
 * buttons are fixed below it, and only the prose in between scrolls.
 *
 * This is the state #197 was raised for. The dialog was a single column that
 * grew with its content, so an explanation this long ran off the top and the
 * bottom of the window with no way to reach either end — and the field the
 * confirm control is waiting on, plus the confirm control itself, were the
 * first things off the bottom. Resize the preview to a short window: Cancel,
 * the field and Overrule stay where they are.
 *
 * `wide`, because the body carries findings rather than sentences. Read it at
 * `default` to see why the second measure exists.
 */
export const MoreThanFitsWithAFieldToReach: Story = {
  args: {
    open: true,
    tone: "neutral",
    width: "wide",
    title: "Overrule the gaming flag on this step?",
    confirmLabel: "Overrule the flag",
    field: <Textarea label="Why the flag is wrong" rows={3} />,
    children: (
      <>
        <p>
          The gaming check flagged the evidence for Regression check. It did not refuse the work —
          it says the evidence for it is not to be trusted. Overruling says a person has read that
          evidence and takes responsibility for it; the step advances still recorded as failed
          against the flag.
        </p>
        <p>
          It is not the last step, so the job carries on at the next one. Your reason is written to
          this job&apos;s log and stays there — the log is append-only, and nothing takes an
          override back. It is not sent to the drone, which did nothing wrong and is told only that
          the step was accepted.
        </p>
        <p>
          The check reads the diff and the evidence together and infers intent, which is why it is
          its own escalation rather than a gate failure: resubmitting under the same instructions
          would likely reproduce whatever it found, so the job stops and asks rather than retrying.
        </p>
        <p>
          A person overruling it is on the record as having read the finding. That is the whole
          reason this is a dialog and not a button.
        </p>
      </>
    ),
  },
};

/**
 * **Waiting on the field, and refusing twice.** `confirmDisabled` turns the
 * confirm control off without hiding it, because a control that vanished would
 * leave a person looking for the act rather than for the reason it is off.
 *
 * Three screens set this — `Overrule`, `Redirect` and `Report` in
 * `packages/screens` — all on `trim() === ""`, and all three are acts that go
 * on a record nothing takes back.
 */
export const WaitingOnAField: Story = {
  args: {
    open: true,
    tone: "neutral",
    width: "wide",
    title: "Overrule the gaming flag on this step?",
    confirmLabel: "Overrule the flag",
    confirmDisabled: true,
    field: <Textarea label="Why the flag is wrong" rows={3} />,
    children:
      "Overruling says a person has read the evidence and takes responsibility for it. Your reason " +
      "is written to this job's log and stays there — the log is append-only, and nothing takes an " +
      "override back.",
    onConfirm: fn(),
    onCancel: fn(),
  },
  /**
   * **Two refusals, and only one of them can be seen.** The button carries
   * `disabled`; the `Enter` handler bound on `window` has its own
   * `if (!confirmDisabled)`. They are independent, so a regression in the
   * second leaves a dialog whose confirm reads as refused and confirms anyway
   * from the keyboard — on the one surface where the act is a person putting
   * their name to something.
   *
   * `Esc` is asserted beside them because a dialog that refused every key
   * would pass the line above and be worse than the bug: the way out has to
   * work while the way through does not.
   */
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "Overrule the flag" })).toBeDisabled();

    await userEvent.keyboard("{Enter}");
    await expect(args.onConfirm).not.toHaveBeenCalled();

    await userEvent.keyboard("{Escape}");
    await expect(args.onCancel).toHaveBeenCalled();
  },
};

/**
 * The same dialog with the field wired, which is the only way the transition
 * can be read: `Waiting on a field` above holds `confirmDisabled` as a literal,
 * so nothing there can be satisfied.
 *
 * The state is the caller's in the app too — `Overrule.tsx` holds the reason
 * and hands this component the answer to *is it blank*. This wrapper is that
 * caller, reduced to the one rule.
 */
export const TheReasonSatisfiesIt: Story = {
  render: () => {
    const [reason, setReason] = useState("");
    return (
      <Dialog
        open
        tone="neutral"
        width="wide"
        title="Overrule the gaming flag on this step?"
        confirmLabel="Overrule the flag"
        confirmDisabled={reason.trim() === ""}
        field={
          <Textarea
            label="Why the flag is wrong"
            rows={3}
            value={reason}
            onChange={(event) => setReason(event.target.value)}
          />
        }
      >
        Overruling says a person has read the evidence and takes responsibility for it.
      </Dialog>
    );
  },
  /**
   * Blank, then whitespace, then a reason. **The whitespace step is the one
   * worth the line**: `trim()` is what every caller of this prop uses, and a
   * check that had lost it would enable the confirm on a field holding spaces —
   * which reads as filled and records nothing.
   */
  play: async ({ canvas, userEvent }) => {
    const confirm = canvas.getByRole("button", { name: "Overrule the flag" });
    const field = canvas.getByRole("textbox", { name: "Why the flag is wrong" });
    await expect(confirm).toBeDisabled();

    await userEvent.type(field, "   ");
    await expect(confirm).toBeDisabled();

    await userEvent.type(field, "it read the wrong diff");
    await expect(confirm).toBeEnabled();
  },
};
