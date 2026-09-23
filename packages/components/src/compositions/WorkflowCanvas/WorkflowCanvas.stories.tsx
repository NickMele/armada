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

// The four steps `feature.json` declares, with the arc's four groups hanging
// off the step that wrote them and their tasks off the groups. The placement
// is the caller's, and the caller here writes the numbers
// `workflow-canvas.ts` computes.
const STEP_X = 320;
const GROUP_Y = 150;
const GROUP_APART = 184;
const TASK_X = 272;
const TASK_APART = 80;

const steps: WorkflowCanvasNode[] = [
  {
    id: "step:plan",
    position: { x: 0, y: 0 },
    card: { kind: "step", name: "Plan the change", activity: "advanced", said: "advanced", ordinal: 1, facts: [{ value: "4 groups" }, { value: "2 criteria" }], onOpen: fn() },
  },
  {
    id: "step:implement",
    position: { x: STEP_X, y: 0 },
    card: { kind: "step", name: "Implement", activity: "running", said: "running", ordinal: 2, current: true, facts: [{ value: "11 checks" }], onOpen: fn() },
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

/** One group of the arc's plan, with its two tasks beside it. */
const GROUPS: { id: string; ordinal: number; said: string; activity: "advanced" | "running" | "not_started"; worked: boolean; tasks: string[] }[] = [
  { id: "g1", ordinal: 1, said: "passed", activity: "advanced", worked: true, tasks: ["Serve one read of everything running", "Send an event when what is running changes"] },
  { id: "g2", ordinal: 2, said: "passed", activity: "advanced", worked: true, tasks: ["Reword the stat to one running beside the most", "Say the same words on the Board and in Settings"] },
  { id: "g3", ordinal: 3, said: "checking", activity: "running", worked: true, tasks: ["Draw what is running, in four lists", "Open a Drone's Job from its row"] },
  { id: "g4", ordinal: 4, said: "pending", activity: "not_started", worked: false, tasks: ["Say what holds the next Drone back", "Four stories: nothing out, one Drone, three at once, and the most"] },
];

const under: WorkflowCanvasNode[] = GROUPS.flatMap((group, at) => [
  {
    id: `group:${group.id}`,
    position: { x: 16, y: GROUP_Y + at * GROUP_APART },
    card: {
      kind: "group" as const,
      name: `Group ${group.ordinal}`,
      activity: group.activity,
      said: group.said,
      facts: [{ value: "2 tasks" }, { value: "7 checks" }],
      onOpen: fn(),
    },
  },
  ...group.tasks.map((title, k) => ({
    id: `task:${group.id}-${k}`,
    position: { x: TASK_X, y: GROUP_Y + at * GROUP_APART + k * TASK_APART },
    card: {
      kind: "task" as const,
      name: title,
      activity: group.worked ? ("advanced" as const) : ("not_started" as const),
      said: group.worked ? "done" : "open",
      facts: [{ value: `T${at * 2 + k + 1}` }, { value: "1 file" }],
      onOpen: fn(),
    },
  })),
]);

const nodes: WorkflowCanvasNode[] = [...steps, ...under];

const spine: WorkflowCanvasEdge[] = [
  { id: "plan>implement", source: "step:plan", target: "step:implement", kind: "leads" },
  { id: "implement>tests", source: "step:implement", target: "step:tests", kind: "leads" },
  { id: "tests>handoff", source: "step:tests", target: "step:handoff", kind: "leads" },
];

const plan: WorkflowCanvasEdge[] = GROUPS.flatMap((group) => [
  { id: `plan>${group.id}`, source: "step:plan", target: `group:${group.id}`, kind: "made" as const },
  ...(group.worked
    ? [{ id: `implement>${group.id}`, source: "step:implement", target: `group:${group.id}`, kind: "worked" as const }]
    : []),
  ...group.tasks.map((_, k) => ({
    id: `${group.id}>t${k}`,
    source: `group:${group.id}`,
    target: `task:${group.id}-${k}`,
    kind: "holds" as const,
  })),
]);

const edges: WorkflowCanvasEdge[] = [...spine, ...plan];

/**
 * A feature Job mid-implement: the plan step's four groups, their tasks, and
 * a second edge from the step working each group that has been worked.
 *
 * **A `play`, because a still cannot say which line is which.** Two lines meet
 * group one and a picture cannot tell you what either claims; the edges'
 * accessible names are where the graph says it, and they are the only place a
 * reader who cannot see it is told at all.
 */
export const AFeatureRun: Story = {
  args: { nodes, edges, label: "The run", running: "step:implement", onFollowing: fn() },
  play: async ({ canvas }) => {
    // A group hangs off the step that wrote it, and its tasks off it.
    await waitFor(() => expect(canvas.getByLabelText("Plan the change made Group 1")).toBeInTheDocument());
    expect(canvas.getByLabelText("Group 1 holds Serve one read of everything running")).toBeInTheDocument();
    // A worked group carries a second edge into the same node; an unworked
    // group carries the first alone.
    expect(canvas.getByLabelText("Implement worked Group 1")).toBeInTheDocument();
    expect(canvas.getByLabelText("Plan the change made Group 4")).toBeInTheDocument();
    expect(canvas.queryByLabelText("Implement worked Group 4")).toBeNull();
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
 * Narrow opens on the step a person is on, its neighbours and the groups those
 * steps made, rather than fitting a whole plan no one can read (`#1539`,
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
    opensOn: [
      ["step:plan", "step:implement", "step:tests", "group:g1", "group:g2", "group:g3", "group:g4"],
      ["step:implement", "group:g1", "group:g2", "group:g3"],
    ],
  },
  play: async ({ canvas }) => {
    await waitFor(() => {
      const step = canvas.getByRole("button", { name: "Implement, running" });
      expect(step.getBoundingClientRect().width).toBeGreaterThan(160);
    });
  },
};
