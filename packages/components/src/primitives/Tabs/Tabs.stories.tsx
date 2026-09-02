import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
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
  /**
   * The arrows, and the wrap at the end of the strip. `Tabs` computes the next
   * index itself rather than taking one from a library, so the wrap is
   * arithmetic somebody wrote — and arithmetic on an index is where an
   * off-by-one lives.
   *
   * Left twice from `Diff` is the case worth pressing: once to `Log` through
   * the wrap, and the strip is three long, so a modulo that lost its `+
   * items.length` would land on nothing instead.
   */
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("tab", { name: "Diff" }));
    await expect(canvas.getByRole("tab", { selected: true })).toHaveAccessibleName("Diff");

    await userEvent.keyboard("{ArrowRight}");
    await expect(canvas.getByRole("tab", { selected: true })).toHaveAccessibleName("Evidence");

    await userEvent.keyboard("{ArrowLeft}{ArrowLeft}");
    await expect(canvas.getByRole("tab", { selected: true })).toHaveAccessibleName("Log");
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
