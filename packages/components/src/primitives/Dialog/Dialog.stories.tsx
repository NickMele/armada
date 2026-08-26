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
