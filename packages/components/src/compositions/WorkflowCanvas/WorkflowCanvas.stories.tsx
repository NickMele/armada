import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, waitFor } from "storybook/test";

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

// The four steps `feature.json` declares, with one Plan node under the step
// that recorded the plan. The placement is the caller's, and the caller here
// writes the numbers `workflow-canvas.ts` computes.
const STEP_X = 320;
const PLAN_Y = 150;

const steps: WorkflowCanvasNode[] = [
  {
    id: "step:plan",
    position: { x: 0, y: 0 },
    card: { kind: "step", name: "Plan the change", activity: "advanced", said: "advanced", ordinal: 1, facts: [{ value: "4 groups" }, { value: "2 criteria" }], onOpen: fn() },
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
];

/** The plan, as the one node the run draws of it. */
const planNode: WorkflowCanvasNode = {
  id: "plan",
  position: { x: 16, y: PLAN_Y },
  card: {
    kind: "plan",
    name: "Plan",
    activity: "running",
    said: "running",
    facts: [{ value: "4 groups" }, { value: "8 tasks" }],
    onOpen: fn(),
  },
};

const spine: WorkflowCanvasEdge[] = [
  { id: "plan>implement", source: "step:plan", target: "step:implement", kind: "leads" },
  { id: "implement>tests", source: "step:implement", target: "step:tests", kind: "leads" },
  { id: "tests>handoff", source: "step:tests", target: "step:handoff", kind: "leads" },
];

const nodes: WorkflowCanvasNode[] = [...steps, planNode];
const edges: WorkflowCanvasEdge[] = [
  ...spine,
  { id: "step:plan>plan", source: "step:plan", target: "plan", kind: "made" },
];

/**
 * A feature Job mid-implement: the four steps its workflow file declares, and
 * the one Plan node hanging off the step that recorded the plan.
 *
 * **A `play`, because a still cannot say which line is which.** An edge's
 * accessible name is where the graph says what it joins, and it is the only
 * place a reader who cannot see it is told at all.
 */
export const AFeatureRun: Story = {
  args: { nodes, edges, label: "The run", running: "step:implement", onFollowing: fn() },
  play: async ({ canvas }) => {
    await waitFor(() => expect(canvas.getByLabelText("Plan the change made Plan")).toBeInTheDocument());
    // One node for the plan, and nothing of what is inside it on this canvas.
    expect(canvas.getByRole("button", { name: "Plan, running" })).toHaveTextContent("4 groups");
    expect(canvas.queryByRole("button", { name: /^Group 1, / })).toBeNull();
  },
};

/** One group of a plan, with its two tasks beside it. */
const GROUPS: { id: string; ordinal: number; said: string; activity: "advanced" | "running" | "not_started"; done: boolean; tasks: string[] }[] = [
  { id: "g1", ordinal: 1, said: "passed", activity: "advanced", done: true, tasks: ["Serve one read of everything running", "Send an event when what is running changes"] },
  { id: "g2", ordinal: 2, said: "passed", activity: "advanced", done: true, tasks: ["Reword the stat to one running beside the most", "Say the same words on the Board and in Settings"] },
  { id: "g3", ordinal: 3, said: "checking", activity: "running", done: true, tasks: ["Draw what is running, in four lists", "Open a Drone's Job from its row"] },
  { id: "g4", ordinal: 4, said: "pending", activity: "not_started", done: false, tasks: ["Say what holds the next Drone back", "Four stories: nothing out, one Drone, three at once, and the most"] },
];

const GROUP_APART = 232;
const TASK_X = 256;
const TASK_APART = 104;

const planNodes: WorkflowCanvasNode[] = GROUPS.flatMap((group, at) => [
  {
    id: `group:${group.id}`,
    position: { x: 0, y: at * GROUP_APART },
    card: {
      kind: "group" as const,
      name: `Group ${group.ordinal}`,
      activity: group.activity,
      said: group.said,
      facts: [{ value: "2 tasks" }, { value: "7 checks" }],
    },
  },
  ...group.tasks.map((title, k) => ({
    id: `task:${group.id}-${k}`,
    position: { x: TASK_X, y: at * GROUP_APART + k * TASK_APART },
    card: {
      kind: "task" as const,
      name: title,
      activity: group.done ? ("advanced" as const) : ("not_started" as const),
      said: group.done ? "done" : "open",
      facts: [{ value: `T${at * 2 + k + 1}` }, { value: "1 file" }],
      onOpen: fn(),
    },
  })),
]);

const planEdges: WorkflowCanvasEdge[] = GROUPS.flatMap((group) =>
  group.tasks.map((_, k) => ({
    id: `${group.id}>t${k}`,
    source: `group:${group.id}`,
    target: `task:${group.id}-${k}`,
    kind: "holds" as const,
  })),
);

/**
 * The same canvas drawing a plan rather than a run — the Plan tab's Graph view
 * (owner, 25 Sep 2026). **A group is a root here**: there are no step nodes on
 * that tab, so the plan is as many small trees as it has groups.
 */
export const APlan: Story = {
  args: { nodes: planNodes, edges: planEdges, label: "The plan" },
  play: async ({ canvas }) => {
    await waitFor(() =>
      expect(canvas.getByLabelText("Group 1 holds Serve one read of everything running")).toBeInTheDocument(),
    );
    // A task is what a press opens; a group is a card and not a control.
    expect(canvas.getByRole("button", { name: "Draw what is running, in four lists, done" })).toBeInTheDocument();
    expect(canvas.queryByRole("button", { name: "Group 4, pending" })).toBeNull();
  },
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
    nodes: steps,
    edges: [
      ...spine,
      { id: "tests>plan", source: "step:tests", target: "step:plan", kind: "returns", label: "up to 5 passes" },
    ],
    label: "The run",
  },
};

/**
 * Narrow opens on the step a person is on, its neighbours and the plan those
 * steps hold, rather than fitting a whole run no one can read (`#1539`,
 * revision of 22 Sep).
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
    opensOn: [["step:plan", "step:implement", "step:tests", "plan"], ["step:implement", "plan"]],
  },
  play: async ({ canvas }) => {
    await waitFor(() => {
      const step = canvas.getByRole("button", { name: "Implement, running" });
      expect(step.getBoundingClientRect().width).toBeGreaterThan(160);
    });
  },
};
