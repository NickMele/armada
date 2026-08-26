import type { Meta, StoryObj } from "@storybook/react-vite";
import { Card, CardTitle } from "../Card/Card";
import { Skeleton, SkeletonText } from "./Skeleton";

/**
 * The contract names one skeleton rendering and no others: bars in
 * `--bg-hover`, drawn at rest. There is no loaded-in transition, because no
 * entrance animation is permitted on data.
 */
const meta: Meta<typeof Skeleton> = {
  title: "Primitives/Skeleton",
  component: Skeleton,
  decorators: [
    (Story) => (
      <div style={{ maxWidth: "56ch" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof Skeleton>;

export const Single: Story = {
  render: () => <Skeleton width="60%" />,
};

/**
 * Three bars at unequal widths — the thing that stops a block reading as a
 * table of empty cells.
 */
export const Text: Story = {
  render: () => <SkeletonText />,
};

/** In place, on the surface the data will land on. */
export const InACard: Story = {
  render: () => (
    <Card>
      <CardTitle>Evidence</CardTitle>
      <SkeletonText label="Loading evidence" />
    </Card>
  ),
};
