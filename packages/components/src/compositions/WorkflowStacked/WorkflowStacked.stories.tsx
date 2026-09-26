import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { WorkflowStacked, type WorkflowStackedRow } from "./WorkflowStacked";

const meta: Meta<typeof WorkflowStacked> = {
  title: "Compositions/Workflow stacked",
  component: WorkflowStacked,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-run-column)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WorkflowStacked>;

/** The one Plan row, indented under the step that recorded it. */
const plan: WorkflowStackedRow = {
  id: "plan",
  under: "step:plan",
  depth: 1,
  card: {
    kind: "plan",
    name: "Plan",
    activity: "running",
    said: "running",
    facts: [{ value: "4 groups" }, { value: "8 tasks" }],
    onOpen: fn(),
  },
};

const rows: WorkflowStackedRow[] = [
  { id: "step:plan", card: { kind: "step", name: "Plan the change", activity: "advanced", said: "advanced", ordinal: 1, facts: [{ value: "4 groups" }, { value: "2 criteria" }], onOpen: fn() } },
  plan,
  { id: "step:implement", card: { kind: "step", name: "Implement", activity: "running", said: "running", ordinal: 2, current: true, facts: [{ value: "4 groups" }, { value: "11 checks" }], onOpen: fn() } },
  { id: "step:tests", card: { kind: "step", name: "Write tests", activity: "not_started", said: "not started", ordinal: 3, facts: [{ value: "7 checks" }], onOpen: fn() } },
  { id: "step:handoff", card: { kind: "step", name: "Review the change", activity: "not_started", said: "not started", ordinal: 4, gate: "a person answers", onOpen: fn() } },
];

/** The same feature run the canvas draws, as a column. */
export const AFeatureRun: Story = { args: { label: "The run", rows } };

/** A loop has nothing to arc over in a column, so the step says where it goes back to. */
export const WithALoop: Story = {
  args: {
    label: "The run",
    rows: rows.map((row) =>
      row.id === "step:tests"
        ? { ...row, returns: { toName: "Plan the change", label: "up to 5 passes" } }
        : row,
    ),
  },
};

/**
 * The claim the toggle rests on: the two arrangements say the same thing.
 *
 * **A `play`, because a still cannot say it.** The column carries the same
 * cards the canvas places, the Plan node among them — a column that dropped
 * one would make the toggle a change of subject.
 */
export const SaysWhatTheCanvasSays: Story = {
  args: { label: "The run", rows },
  play: async ({ canvas }) => {
    const run = canvas.getByRole("list", { name: "The run" });
    await expect(run).toBeVisible();
    for (const name of ["Plan the change, advanced", "Implement, running", "Write tests, not started"]) {
      await expect(canvas.getByRole("button", { name })).toBeVisible();
    }
    // The plan is one row, under the step that recorded it, saying what it holds.
    const node = canvas.getByRole("button", { name: "Plan, running" });
    await expect(node).toHaveTextContent("4 groups");
    await expect(node).toHaveTextContent("8 tasks");
  },
};
