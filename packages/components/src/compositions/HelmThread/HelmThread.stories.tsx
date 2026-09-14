import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { HelmThread, type HelmThreadRow } from "./HelmThread";

/** Helm's own conversation, drawn at the dock's own width. */
const meta: Meta<typeof HelmThread> = {
  title: "Compositions/Helm thread",
  component: HelmThread,
  render: (args) => (
    <div style={{ width: "var(--w-dock)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
      <HelmThread {...args} />
    </div>
  ),
};
export default meta;

type Story = StoryObj<typeof HelmThread>;

const asked: HelmThreadRow = { id: "1", at: "14:29:40", actor: "you", message: "Why did job 12 stall?" };
const replied: HelmThreadRow = {
  id: "2",
  at: "14:29:52",
  actor: "helm",
  message: "Job 12 is waiting on a command it was not given: cargo nextest run -p store.",
};
const cost: HelmThreadRow = { ...replied, id: "3", message: "It needs an answer in the dock above.", meta: "~$0.01 · 4 turns" };

/** A conversation with a reply already in, nothing in flight. */
export const AtRest: Story = { args: { rows: [asked, cost] } };

/** Nothing asked yet. */
export const Empty: Story = { args: { rows: [] } };

/** A message just went out and Helm's own rows are still arriving. */
export const MidReply: Story = {
  args: { rows: [asked], replying: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Helm is replying…");
  },
};

/** Start fresh answered and the old thread is gone; the new one has not opened yet. */
export const Cleared: Story = {
  args: { rows: [], emptyNote: "The conversation is cleared. Ask Helm something to start again." },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("note")).toHaveTextContent("cleared");
  },
};

/** Fleet is unreachable. What already arrived stays on screen. */
export const FleetUnreachable: Story = {
  args: { rows: [asked, cost], notice: "Fleet is not connected. What Helm already said is still here." },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent("Fleet is not connected");
  },
};
