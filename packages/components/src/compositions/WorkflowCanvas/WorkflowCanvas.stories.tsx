import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, waitFor } from "storybook/test";

import { Button } from "../../primitives/Button/Button";
import { WorkflowCanvas, type WorkflowCanvasEdge, type WorkflowCanvasNode } from "./WorkflowCanvas";

const meta: Meta<typeof WorkflowCanvas> = {
  title: "Compositions/Workflow canvas",
  component: WorkflowCanvas,
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

type Story = StoryObj<typeof WorkflowCanvas>;

// The four steps `feature.json` declares, with `implement` opened into the
// four groups the arc's plan has under it. The placement is the caller's, and
// the caller here writes the same numbers `workflow-canvas.ts` computes.
const STEP_X = 320;
const GROUP_Y = 150;

const nodes: WorkflowCanvasNode[] = [
  {
    id: "step:plan",
    position: { x: 0, y: 0 },
    card: { kind: "step", name: "Plan the change", activity: "advanced", said: "advanced", ordinal: 1, facts: [{ value: "2 of 2 passed", named: "passed" }], onOpen: fn() },
  },
  {
    id: "step:implement",
    position: { x: STEP_X, y: 0 },
    card: { kind: "step", name: "Implement", activity: "running", said: "running", ordinal: 2, current: true, facts: [{ value: "4 groups" }, { value: "11 checks" }], onOpen: fn() },
  },
  {
    id: "step:tests",
    position: { x: STEP_X * 2, y: 0 },
    card: { kind: "step", name: "Write tests", activity: "not_started", said: "not started", ordinal: 3, facts: [{ value: "7 checks" }], onOpen: fn() },
  },
  {
    id: "step:handoff",
    position: { x: STEP_X * 3, y: 0 },
    card: { kind: "step", name: "Review the change", activity: "not_started", said: "not started", ordinal: 4, gate: "a person answers", onOpen: fn() },
  },
  {
    id: "group:g1",
    position: { x: STEP_X, y: GROUP_Y },
    card: { kind: "group", name: "Group 1", activity: "advanced", said: "passed", facts: [{ value: "2 tasks" }, { value: "4 checks" }], onOpen: fn() },
  },
  {
    id: "group:g2",
    position: { x: STEP_X, y: GROUP_Y + 110 },
    card: { kind: "group", name: "Group 2", activity: "advanced", said: "passed", facts: [{ value: "2 tasks" }], onOpen: fn() },
  },
  {
    id: "group:g3",
    position: { x: STEP_X, y: GROUP_Y + 220 },
    card: { kind: "group", name: "Group 3", activity: "running", said: "checking", facts: [{ value: "2 tasks" }, { value: "7 checks" }], onOpen: fn() },
  },
  {
    id: "group:g4",
    position: { x: STEP_X, y: GROUP_Y + 330 },
    card: { kind: "group", name: "Group 4", activity: "not_started", said: "pending", facts: [{ value: "2 tasks" }], onOpen: fn() },
  },
];

const edges: WorkflowCanvasEdge[] = [
  { id: "plan>implement", source: "step:plan", target: "step:implement" },
  { id: "implement>tests", source: "step:implement", target: "step:tests" },
  { id: "tests>handoff", source: "step:tests", target: "step:handoff" },
  { id: "implement>g1", source: "step:implement", target: "group:g1" },
  { id: "g1>g2", source: "group:g1", target: "group:g2" },
  { id: "g2>g3", source: "group:g2", target: "group:g3" },
  { id: "g3>g4", source: "group:g3", target: "group:g4" },
];

/** A feature Job mid-implement, with the step opened into its groups. */
export const AFeatureRun: Story = {
  args: { nodes, edges, label: "The run", running: "step:implement", following: true, onFollowing: fn() },
};

/**
 * A step that sends the run back to an earlier one — dashed, and arcing above
 * the spine.
 *
 * **No shipped workflow declares one today.** `verdict_routing_target` is on
 * the wire and none of the eight files under `.armada/workflows/` sets it, so
 * this is the state drawn ahead of the data rather than a reading of it.
 */
export const WithALoop: Story = {
  args: {
    nodes: nodes.slice(0, 4),
    edges: [
      ...edges.slice(0, 3),
      { id: "tests>plan", source: "step:tests", target: "step:plan", returning: true, label: "up to 5 passes" },
    ],
    label: "The run",
  },
};

/**
 * Narrow opens on the step a person is on and its neighbours, rather than
 * fitting nine cards no one can read (`#1539`, revision of 22 Sep).
 *
 * **A `play`, because a still cannot say what the canvas was fitted to.** What
 * has to hold is that the step it opened on is legible — which is the whole
 * claim — and a card drawn at a tenth of its width still renders.
 */
export const OpensOnWhereYouAre: Story = {
  args: {
    nodes,
    edges,
    label: "The run",
    running: "step:implement",
    opensOn: ["step:plan", "step:implement", "step:tests"],
  },
  play: async ({ canvas }) => {
    const step = canvas.getByRole("button", { name: "Implement, running" });
    await waitFor(() => expect(step.getBoundingClientRect().width).toBeGreaterThan(200));
  },
};

/** The toggle back to the stacked run, drawn over the canvas by its surface. */
export const WithTheToggle: Story = {
  args: {
    nodes,
    edges,
    label: "The run",
    running: "step:implement",
    aside: <Button size="sm">Stacked</Button>,
  },
};
