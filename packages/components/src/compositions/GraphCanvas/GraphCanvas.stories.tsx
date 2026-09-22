import type { Meta, StoryObj } from "@storybook/react-vite";
import { Handle, type Edge, type Node, type NodeProps } from "@xyflow/react";
import { expect, waitFor } from "storybook/test";

import { Card, CardContent, CardTitle } from "../../primitives/Card/Card";
import { GRAPH_CANVAS_SIDES, GraphCanvas } from "./GraphCanvas";

/**
 * The graph surface on its own, with three plain cards on it — what is left
 * when neither a Studio's node nor a workflow's step is drawn. The two real
 * surfaces are *Studio whiteboard* and *Workflow canvas*.
 */
const meta: Meta<typeof GraphCanvas> = {
  title: "Compositions/Graph canvas",
  component: GraphCanvas,
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

type Card = Node<{ title: string }, "card">;
type Line = Edge<Record<string, never>, "default">;

function CardView({ data, selected }: NodeProps<Card>) {
  return (
    <>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <Card role="group" aria-label={data.title} aria-current={selected || undefined}>
        <CardContent>
          <CardTitle>{data.title}</CardTitle>
        </CardContent>
      </Card>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`s-${side}`} id={`s-${side}`} type="source" position={side} isConnectable={false} />
      ))}
    </>
  );
}

const NODE_TYPES = { card: CardView };
const EDGE_TYPES = {};

const nodes: Card[] = [
  { id: "a", type: "card", position: { x: 0, y: 0 }, data: { title: "One" } },
  { id: "b", type: "card", position: { x: 260, y: 0 }, data: { title: "Two" } },
  { id: "c", type: "card", position: { x: 520, y: 0 }, data: { title: "Three" } },
];

const edges: Line[] = [
  { id: "a-b", source: "a", target: "b", sourceHandle: "s-right", targetHandle: "t-left" },
  { id: "b-c", source: "b", target: "c", sourceHandle: "s-right", targetHandle: "t-left" },
];

const args = { surface: "armada-graph-canvas-demo", label: "Three cards", nodes, edges, nodeTypes: NODE_TYPES, edgeTypes: EDGE_TYPES };

type Story = StoryObj<typeof GraphCanvas<Card, Line>>;

/** A surface a person works in names its controls. */
export const NamedControls: Story = { args };

/**
 * A surface drawn over a run draws the signs instead.
 *
 * **A `play`, because the rendering is the half that changed and the half that
 * must not is what a screen reader hears.** `−` and `+` carry no accessible
 * name of their own, so without the labels below the two controls are read out
 * as punctuation.
 */
export const SignedControls: Story = {
  args: { ...args, controls: "signs" },
  play: async ({ canvas }) => {
    await waitFor(() => expect(canvas.getByRole("button", { name: "Zoom in" })).toBeVisible());
    await expect(canvas.getByRole("button", { name: "Zoom in" })).toHaveTextContent("+");
    await expect(canvas.getByRole("button", { name: "Zoom out" })).toHaveTextContent("−");
    await expect(canvas.getByRole("button", { name: "Fit" })).toBeVisible();
  },
};
