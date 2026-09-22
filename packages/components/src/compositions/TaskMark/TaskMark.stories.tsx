import type { Meta, StoryObj } from "@storybook/react-vite";
import { TaskMark } from "./TaskMark";

/** One story per task state — the whole of the mark's vocabulary. */
const meta: Meta<typeof TaskMark> = {
  title: "Compositions/Task mark",
  component: TaskMark,
};
export default meta;

type Story = StoryObj<typeof TaskMark>;

export const Open: Story = { args: { state: "open" } };
export const Working: Story = { args: { state: "working" } };
export const Done: Story = { args: { state: "done" } };
export const Failed: Story = { args: { state: "failed" } };
export const Dropped: Story = { args: { state: "dropped" } };
