import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { WhatElseIsRunning } from "./WhatElseIsRunning";

/**
 * One story per state the panel has: nobody looked, a comparison that found
 * nobody, and the reading itself. The first two are the pair this component
 * exists to keep apart.
 */
const meta: Meta<typeof WhatElseIsRunning> = {
  title: "Compositions/What else is running",
  component: WhatElseIsRunning,
};
export default meta;

type Story = StoryObj<typeof WhatElseIsRunning>;

const PEERS = [
  {
    id: "01M2D3P0QW001CAPACITYREAD",
    title: "Fold the capacity read into one query",
    status: "running",
    sharedPaths: ["crates/api/src/"],
  },
  {
    id: "01M2D3P0QW001RAILSCROLL0",
    title: "Give the rail its own scroll",
    status: "awaiting_review",
    sharedPaths: ["packages/screens/src/overview.ts"],
  },
];

/** Two Jobs already writing where this work would, each with the path they share. */
export const TwoJobsWritingThere: Story = {
  args: {
    peers: PEERS,
    pathsAsked: ["crates/api/src/", "crates/fleet/src/", "packages/screens/src/"],
    onOpen: () => {},
  },
};

/** A comparison that ran and found nobody. Not the same sentence as the one below. */
export const NobodyFound: Story = {
  args: { peers: [], pathsAsked: ["crates/api/src/"] },
};

/**
 * Nobody looked — every request before its plan claims a path, which is every
 * request at dispatch today.
 *
 * The play is the rule the two states exist for: this one must not say that no
 * other Job is writing there, because nothing has read whether one is.
 */
export const NobodyLooked: Story = {
  args: { peers: null },
  play: async ({ canvasElement }) => {
    const panel = within(canvasElement);
    await expect(panel.getByText(/Nothing has been compared/)).toBeVisible();
    await expect(panel.queryByText(/No other Job is writing/)).toBeNull();
  },
};
