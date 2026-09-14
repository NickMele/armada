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

/** `#1041`. Asked to approve Job 9, and the card is waiting on a press. */
export const ApprovalReady: Story = {
  args: {
    rows: [
      { id: "1", at: "14:29:40", actor: "you", message: "Approve job 9" },
      {
        id: "2",
        at: "14:29:52",
        actor: "helm",
        message: "Here it is.",
        cards: [
          { id: "c1", jobHandle: "9-fix-801-unanswered-permission-ask", workflow: "bug", stepCount: 4, state: "ready" },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Approve" })).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Not now" })).toBeInTheDocument();
  },
};

/**
 * Approve pressed, and Fleet has not answered — the Job is still
 * `awaiting_approval`. Approve waits and says so; `Not now` is off, because
 * this card has already committed to one answer. #1117.
 */
export const ApprovalPending: Story = {
  args: {
    rows: [
      { id: "1", at: "14:29:40", actor: "you", message: "Approve job 9" },
      {
        id: "2",
        at: "14:29:52",
        actor: "helm",
        message: "Here it is.",
        cards: [
          { id: "c1", jobHandle: "9-fix-801-unanswered-permission-ask", workflow: "bug", stepCount: 4, state: "pending" },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    const approving = canvas.getByRole("button", { name: "Approving…" });
    await expect(approving).toHaveAttribute("aria-busy", "true");
    await expect(canvas.getByRole("button", { name: "Not now" })).toBeDisabled();
  },
};

/** The press landed: the card says so and offers nothing further. */
export const ApprovalApproved: Story = {
  args: {
    rows: [
      { id: "1", at: "14:29:40", actor: "you", message: "Approve job 9" },
      {
        id: "2",
        at: "14:29:52",
        actor: "helm",
        message: "Here it is.",
        cards: [
          { id: "c1", jobHandle: "9-fix-801-unanswered-permission-ask", workflow: "bug", stepCount: 4, state: "approved" },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Approved.")).toBeInTheDocument();
  },
};

/** Not now, pressed. Nothing about the Job moved. */
export const ApprovalDismissed: Story = {
  args: {
    rows: [
      { id: "1", at: "14:29:40", actor: "you", message: "Approve job 9" },
      {
        id: "2",
        at: "14:29:52",
        actor: "helm",
        message: "Here it is.",
        cards: [
          { id: "c1", jobHandle: "9-fix-801-unanswered-permission-ask", workflow: "bug", stepCount: 4, state: "dismissed" },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Dismissed.")).toBeInTheDocument();
  },
};

/** The Job left the gate by another road — Bridge, or a second card — while this one sat unread. */
export const ApprovalAlreadyMoved: Story = {
  args: {
    rows: [
      { id: "1", at: "14:29:40", actor: "you", message: "Approve job 9" },
      {
        id: "2",
        at: "14:29:52",
        actor: "helm",
        message: "Here it is.",
        cards: [
          { id: "c1", jobHandle: "9-fix-801-unanswered-permission-ask", workflow: "bug", stepCount: 4, state: "elsewhere" },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("No longer awaiting approval.")).toBeInTheDocument();
  },
};
