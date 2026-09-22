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
