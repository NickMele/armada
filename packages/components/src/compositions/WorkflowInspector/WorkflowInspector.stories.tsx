import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { WorkflowInspector } from "./WorkflowInspector";

const meta: Meta<typeof WorkflowInspector> = {
  title: "Compositions/Workflow inspector",
  component: WorkflowInspector,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-step-panel-min)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WorkflowInspector>;

const checks = [
  { name: "test", outcome: "passed", named: "passed" as const },
  { name: "typecheck", outcome: "passed", named: "passed" as const },
  { name: "screens_test", outcome: "failed", named: "failed" as const },
  { name: "components_test" },
];

const tests = [
  { id: "c1", title: "A stat with one Drone out reads one running", outcome: "passed", named: "passed" as const },
  { id: "c2", title: "Pressing the stat lists the Drone's Job", outcome: "not covered" },
];

/** A group mid-flight: two tasks, the Checks at its boundary, and the cases beside them. */
export const AGroupRunning: Story = {
  args: {
    name: "Group 3",
    kind: "group",
    doing: "Two tasks running at once, then seven Checks at this group's end.",
    tasks: [
      { id: "T5", title: "Read what every Drone is holding", said: "working", facts: ["4 files", "12 turns"] },
      { id: "T6", title: "Say it in the stat's own words", said: "done", facts: ["2 files", "$0.31"] },
    ],
    checks,
    tests,
    redirect: {
      value: "",
      onChange: fn(),
      onSend: fn(),
      drones: [
        { id: "d-5", label: "Drone on T5" },
        { id: "d-6", label: "Drone on T6" },
      ],
      reaches: "d-5",
      onReaches: fn(),
    },
    stop: {
      children: "Hold to stop this group",
      askLabel: "Stop this group",
      description: "Stops the agents on this group once held until it fills. Letting go sooner stops nothing.",
      onCommit: fn(),
      onAsk: fn(),
    },
  },
};

/** A step nothing has entered: no tasks, no runs, and nothing to stop. */
export const AStepNotStarted: Story = {
  args: {
    name: "Write tests",
    kind: "step",
    doing: "Nothing has entered this step.",
    tasksAbsent: "No task is planned under this step.",
    checks: checks.map((check) => ({ name: check.name })),
    testsAbsent: "No case is owed here until the work declares one.",
  },
};

/**
 * A step with one Drone on it names it in a sentence rather than a picker, and
 * a task a later task edited keeps its flag.
 *
 * **A `play`, because the rule is about what is *not* drawn.** One Drone and a
 * picker would ask a person to choose between one thing; the two lists must
 * also stay two — a case folded in among the Checks reads as a Check that
 * passed, which is the one confusion `#1530` named.
 */
export const OneDroneAndTwoLists: Story = {
  args: {
    name: "Implement",
    kind: "step",
    doing: "One Drone, on task T7.",
    tasks: [
      {
        id: "T6",
        title: "Say it in the stat's own words",
        said: "done",
        facts: ["$0.31"],
        flag: "T7 edited a file this task had finished. It stays done.",
      },
      { id: "T7", title: "Hold the next Drone back", said: "working", facts: ["9 turns"] },
    ],
    checks,
    tests,
    redirect: { value: "", onChange: fn(), onSend: fn(), drones: [{ id: "d-7", label: "Drone on T7" }] },
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("combobox")).toBeNull();
    await expect(canvas.getByText("Reaches Drone on T7")).toBeVisible();
    const boundaryChecks = canvas.getByRole("region", { name: "Checks at this boundary" });
    const boundaryTests = canvas.getByRole("region", { name: "Tests at this boundary" });
    await expect(boundaryChecks).toHaveTextContent("screens_test");
    await expect(boundaryChecks).not.toHaveTextContent("not covered");
    await expect(boundaryTests).toHaveTextContent("not covered");
  },
};

/**
 * One task, read whole — what it is doing, what it was told, what it may touch,
 * what it last wrote and what it has said. `#1536`.
 *
 * **A `play`, because the rule is about what a task's panel does *not* draw.**
 * A task has no boundary of its own: the Checks run at its group's end, so the
 * two boundary regions are absent here rather than drawn empty, which would
 * read as a task whose Checks all passed.
 */
export const OneTask: Story = {
  args: {
    name: "T5 · Draw what is running, in four lists",
    kind: "task",
    doing: "Its agent is working — 14 turns so far. What it cost reads once that agent stops.",
    brief: "Proves it: The panel lists Drones, Checks, Judge calls and proposer calls",
    scope: ["packages/screens/src/Running.tsx", "packages/screens/src/running-rows.tsx"],
    beside: ["T6"],
    lastEdit: { path: "packages/screens/src/Running.tsx", says: "+61 −4" },
    log: [
      { id: "1", at: "10:14:02", said: "Reading the four lists the panel owes" },
      { id: "2", at: "10:16:40", said: "Edit  Running.tsx  +61 −4" },
    ],
    redirect: { value: "", onChange: fn(), onSend: fn(), drones: [{ id: "d-5", label: "Drone on T5" }] },
    stop: {
      children: "Hold to stop this task",
      askLabel: "Stop this task",
      description: "Stops the agent on this task once held until it fills. Letting go sooner stops nothing.",
      onCommit: fn(),
      onAsk: fn(),
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("region", { name: "Checks at this boundary" })).toBeNull();
    await expect(canvas.queryByRole("region", { name: "Tests at this boundary" })).toBeNull();
    await expect(canvas.getByRole("region", { name: "What it runs beside" })).toHaveTextContent("T6");
    await expect(canvas.getByRole("region", { name: "Its last edit" })).toHaveTextContent("Running.tsx");
  },
};

/** A task nothing is watching says which absence it is, never a blank region. */
export const OneTaskWithNothingRead: Story = {
  args: {
    name: "T8 · Four stories: nothing out, one Drone, three at once",
    kind: "task",
    doing: "Nothing has been dispatched at this task yet.",
    briefAbsent: "The planner recorded no words of its own for this task.",
    scope: ["packages/screens/src/Running.stories.tsx"],
    beside: [],
    lastEditAbsent: "Nothing this task wrote has been read on this step yet.",
    log: [],
    logAbsent: "Nothing is watching this Job's turns, so this task's own lines are not being read.",
  },
};
