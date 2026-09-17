import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { FootprintInstrument, type FootprintColumn } from "./FootprintInstrument";

/**
 * A finished Job's footprint: each file a column, width by lines changed,
 * added over deleted. Columns arrive widest first from
 * `packages/screens/src/instruments.ts`, which also words the descriptions.
 */
const meta: Meta<typeof FootprintInstrument> = {
  title: "Compositions/Footprint instrument",
  component: FootprintInstrument,
  args: { label: "Lines changed per file", uncounted: 0 },
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-dialog-wide)", maxWidth: "100%" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof FootprintInstrument>;

/** Nothing counted, and nothing left uncounted: no columns, and the key says so. */
export const Empty: Story = {
  args: { columns: [], description: "No changed lines were counted" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Lines changed per file" })).toHaveAccessibleDescription(
      "No changed lines were counted",
    );
    await expect(canvas.getByText("no lines counted")).toBeVisible();
  },
};

/** Two files, one of them most of the work. */
export const Sparse: Story = {
  args: {
    columns: [
      { path: "crates/fleet/src/silence.rs", added: 84, deleted: 12, outsidePlan: false },
      { path: "crates/fleet/src/lib.rs", added: 2, deleted: 0, outsidePlan: false },
    ],
    description: "2 files changed 98 lines, +86 −12; crates/fleet/src/silence.rs took 98% of them",
  },
};

/**
 * One file outside every declared plan, and one the record could not count.
 * The uncounted file has no column; the key counts it instead.
 */
export const OutsideThePlanAndUncounted: Story = {
  args: {
    columns: [
      { path: "packages/screens/src/JobDetail.tsx", added: 61, deleted: 40, outsidePlan: false },
      { path: "packages/tokens/src/status.css", added: 18, deleted: 9, outsidePlan: true },
      { path: "packages/screens/src/instruments.ts", added: 22, deleted: 0, outsidePlan: false },
    ],
    uncounted: 1,
    description:
      "3 files changed 150 lines, +101 −49; packages/screens/src/JobDetail.tsx took 67% of them; 1 file outside every declared plan; 1 file not counted",
  },
  play: async ({ canvas }) => {
    // The key says what the outline means, and counts what was not drawn.
    await expect(canvas.getByText("outside every declared plan")).toBeVisible();
    await expect(canvas.getByText("1 file not counted")).toBeVisible();
  },
};

/** Only uncounted files: no drawing, a count in the key. Never a guessed width. */
export const OnlyUncounted: Story = {
  args: {
    columns: [],
    uncounted: 3,
    description: "No changed lines were counted; 3 files not counted",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("3 files not counted")).toBeVisible();
    await expect(canvas.queryByText("added")).toBeNull();
  },
};

const DENSE: FootprintColumn[] = Array.from({ length: 40 }, (_, index) => {
  const lines = Math.max(2, Math.round(420 / (index + 1)));
  return {
    path: `crates/api/src/routes/r${index}.rs`,
    added: Math.round(lines * 0.7),
    deleted: lines - Math.round(lines * 0.7),
    outsidePlan: index === 3 || index === 17,
  };
});

/** Forty files, falling off from the widest. Narrow columns keep their hairline. */
export const Dense: Story = {
  args: {
    columns: DENSE,
    description: "40 files changed 1798 lines, +1261 −537; crates/api/src/routes/r0.rs took 23% of them; 2 files outside every declared plan",
  },
};
