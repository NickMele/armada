import type { Meta, StoryObj } from "@storybook/react-vite";
import { FigureList } from "./FigureList";

const meta: Meta<typeof FigureList> = {
  title: "Compositions/FigureList",
  component: FigureList,
};
export default meta;

type Story = StoryObj<typeof FigureList>;

/** Pulse's figures, on the column *Where things are* draws. */
export const Wide: Story = {
  args: {
    figures: [
      { label: "Processes", value: "None", wrong: true },
      { label: "Worktree", value: "gone", wrong: true },
    ],
  },
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-run-column)" }}>
        <Story />
      </div>
    ),
  ],
};

/** The Fleet panel's rows, the column as wide as `protocol` and no wider. */
export const Fit: Story = {
  args: {
    column: "fit",
    figures: [
      { label: "pid", value: "4242" },
      { label: "port", value: "7878" },
      { label: "protocol", value: "14.5" },
      { label: "up", value: "171h 00m" },
    ],
  },
  decorators: [
    (Story) => (
      <div style={{ width: "var(--sidebar-min)" }}>
        <Story />
      </div>
    ),
  ],
};
