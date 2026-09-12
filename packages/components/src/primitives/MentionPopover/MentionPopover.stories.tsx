import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { MentionPopover } from "./MentionPopover";

/**
 * The `@` mention popup's own two states: results to pick from, and the
 * honest answer to a miss — `CommandPalette`'s own empty state, on a smaller
 * list. `useMention` in this directory is what decides when the popup opens
 * and what it holds; nothing about that logic is a rendering, so it is not
 * asserted here — see `DispatchRequest`'s own stories for the field it
 * drives.
 */
const meta: Meta<typeof MentionPopover> = {
  title: "Primitives/MentionPopover",
  component: MentionPopover,
  args: {
    query: "com",
    active: 0,
    onHover: fn(),
    onChoose: fn(),
  },
};
export default meta;

type Story = StoryObj<typeof MentionPopover>;

export const Results: Story = {
  args: {
    results: ["packages/components/src/index.ts", "packages/components/package.json"],
  },
  /** A press on a row is a choice, and it carries the row's own path. */
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("option", { name: "packages/components/package.json" }));
    await expect(args.onChoose).toHaveBeenCalledWith("packages/components/package.json");
  },
};

/** The row `Enter` would choose, drawn active without a press. */
export const SecondRowActive: Story = {
  args: {
    results: ["packages/components/src/index.ts", "packages/components/package.json"],
    active: 1,
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("option", { name: "packages/components/package.json" }),
    ).toHaveAttribute("aria-selected", "true");
    await expect(
      canvas.getByRole("option", { name: "packages/components/src/index.ts" }),
    ).toHaveAttribute("aria-selected", "false");
  },
};

/**
 * Nothing matched. **Names the query**, the same honesty `CommandPalette`'s
 * own empty state carries — there is no suggestion and no did-you-mean.
 */
export const NoMatch: Story = {
  args: { query: "zzz", results: [] },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("No file matches “zzz”.")).toBeVisible();
  },
};
