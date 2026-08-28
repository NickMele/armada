import type { Meta, StoryObj } from "@storybook/react-vite";
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
