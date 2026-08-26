import type { Meta, StoryObj } from "@storybook/react-vite";
import { Toast } from "./Toast";

const meta: Meta<typeof Toast> = {
  title: "Primitives/Toast",
  component: Toast,
};
export default meta;

type Story = StoryObj<typeof Toast>;

/**
 * The one use the contract names: a machine value copies on click and a toast
 * confirms, because a clipboard write is silent and a failed one is otherwise
 * indistinguishable from a dead element.
 *
 * No dot. A clipboard write is not a Job state, and status colour is never
 * chosen; see the report.
 */
export const Copied: Story = {
  args: {
    children: "Copied job_8f2a1c.",
  },
};

/**
 * The action keeps its name through the flow. The button said Kill, so this
 * says Killed. The dot carries `killed`, which is a deliberate human decision
 * and must not read as an error.
 */
export const Killed: Story = {
  args: {
    status: "killed",
    children: "Killed job_8f2a1c. The worktree is left in place.",
  },
};

/** The trailing link the component sheet draws. One at most, never a second decision. */
export const Landed: Story = {
  args: {
    status: "completed-success",
    children: "Convoy landed as one PR.",
    actionLabel: "View",
  },
};
