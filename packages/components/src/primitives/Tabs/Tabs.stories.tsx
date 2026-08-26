import type { Meta, StoryObj } from "@storybook/react-vite";
import { Tabs } from "./Tabs";

const meta: Meta<typeof Tabs> = {
  title: "Primitives/Tabs",
  component: Tabs,
};
export default meta;

type Story = StoryObj<typeof Tabs>;

/**
 * The state the component sheet draws: sections of one object, with no count,
 * because these are views of one job and there is nothing to tally.
 */
export const SectionsOfOneObject: Story = {
  args: {
    defaultValue: "diff",
    items: [
      { id: "diff", label: "Diff" },
      { id: "evidence", label: "Evidence" },
      { id: "log", label: "Log" },
    ],
  },
};

/** The last section active, so the underline can be read away from the left edge. */
export const LastActive: Story = {
  args: {
    defaultValue: "log",
    items: [
      { id: "diff", label: "Diff" },
      { id: "evidence", label: "Evidence" },
      { id: "log", label: "Log" },
    ],
  },
};
