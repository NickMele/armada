import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, waitFor } from "storybook/test";

import { WaveCanvas, type WaveCanvasEdge, type WaveCanvasNode } from "./WaveCanvas";

const meta: Meta<typeof WaveCanvas> = {
  title: "Compositions/Wave canvas",
  component: WaveCanvas,
  parameters: { layout: "fullscreen" },
  decorators: [
    (Story) => (
      <div style={{ height: "100vh", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WaveCanvas>;

// The placement is the caller's, and the caller here writes the same numbers
// `wave.ts` computes from the waiting order.
const APART = 320;
const DOWN = 132;

const nodes: WaveCanvasNode[] = [
  {
    id: "job:a",
    position: { x: 0, y: 0 },
    card: { job: "a", title: "Refuse an unknown code at the seam", status: "completed_success", handle: "32-refuse-an-unknown-code", facts: ["merged"], onOpen: fn() },
  },
  {
    id: "job:b",
    position: { x: APART, y: 0 },
    card: { job: "b", title: "Name the fault in the toast", status: "completed_success", handle: "33-name-the-fault-in-the-toast", facts: ["merged", "waits on 1"], onOpen: fn() },
  },
  {
    id: "job:c",
    position: { x: APART, y: DOWN },
    card: { job: "c", title: "Carry the code into the journal", status: "awaiting_review", handle: "34-carry-the-code-into-the-log", facts: ["waits on 1"], onOpen: fn() },
  },
  {
    id: "job:d",
    position: { x: APART * 2, y: 0 },
    card: { job: "d", title: "Say which half refused", status: "escalated", handle: "35-say-which-half-refused", facts: ["waits on 1"], onOpen: fn() },
  },
  {
    id: "job:e",
    position: { x: APART * 3, y: 0 },
    card: { job: "e", title: "Drop the second error shape", status: "running", handle: "36-drop-the-second-error-shape", facts: ["waits on 2"], onOpen: fn() },
  },
];

// Source is the Job waited on, target the one waiting — so the one that waits
// is drawn behind it.
const edges: WaveCanvasEdge[] = [
  { id: "a>b", source: "job:a", target: "job:b" },
  { id: "a>c", source: "job:a", target: "job:c" },
  { id: "b>d", source: "job:b", target: "job:d" },
  { id: "c>e", source: "job:c", target: "job:e" },
  { id: "d>e", source: "job:d", target: "job:e" },
];

/** Five Jobs under one plan: two merged, one at a gate, one asking, one running. */
export const AWaveMidFlight: Story = {
  args: { nodes, edges, label: "The wave" },
};

/**
 * A wave nothing waits on anything in — every Job may start at once, which is
 * what the derivation from the Board's rows produces until Fleet serves the
 * order.
 */
export const NothingWaitsOnAnything: Story = {
  args: {
    nodes: nodes.map((node, at) => ({ ...node, position: { x: 0, y: at * DOWN }, card: { ...node.card, facts: node.card.facts?.filter((fact) => !fact.startsWith("waits")) } })),
    edges: [],
    label: "The wave",
  },
};

/**
 * A status no registry row carries. **It shows the wire value rather than
 * drawing nothing**, because a missing verb is a fact about the registry and
 * a card with no state reads as one nobody has read.
 */
export const AStatusTheRegistryHasNoWordFor: Story = {
  args: {
    nodes: [{ ...nodes[0]!, card: { ...nodes[0]!.card, status: "reticulating", facts: [] } }],
    edges: [],
    label: "The wave",
  },
};

/**
 * A wave too wide for its frame opens on the Jobs still out rather than
 * fitting every card below the size anyone can read — `bridge.md`'s v1
 * complaint.
 *
 * **A `play`, because a still cannot say what the canvas was fitted to.**
 */
export const OpensOnWhatIsStillOut: Story = {
  args: { nodes, edges, label: "The wave", opensOn: ["job:c", "job:d", "job:e"] },
  play: async ({ canvas }) => {
    await waitFor(() => {
      const card = canvas.getByRole("button", { name: "Say which half refused, needs you" });
      expect(card.getBoundingClientRect().width).toBeGreaterThan(200);
    });
  },
};
