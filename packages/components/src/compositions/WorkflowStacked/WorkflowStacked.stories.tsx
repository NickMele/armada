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

const rows: WorkflowStackedRow[] = [
  { id: "step:plan", card: { kind: "step", name: "Plan the change", activity: "advanced", said: "advanced", ordinal: 1, facts: [{ value: "2 of 2 passed", named: "passed" }], onOpen: fn() } },
  { id: "step:implement", card: { kind: "step", name: "Implement", activity: "running", said: "running", ordinal: 2, current: true, facts: [{ value: "4 groups" }, { value: "11 checks" }], onOpen: fn() } },
  { id: "group:g1", under: "step:implement", card: { kind: "group", name: "Group 1", activity: "advanced", said: "passed", facts: [{ value: "2 tasks" }], onOpen: fn() } },
  { id: "group:g2", under: "step:implement", card: { kind: "group", name: "Group 2", activity: "advanced", said: "passed", facts: [{ value: "2 tasks" }], onOpen: fn() } },
  { id: "group:g3", under: "step:implement", card: { kind: "group", name: "Group 3", activity: "running", said: "checking", facts: [{ value: "2 tasks" }, { value: "7 checks" }], onOpen: fn() } },
  { id: "group:g4", under: "step:implement", card: { kind: "group", name: "Group 4", activity: "not_started", said: "pending", facts: [{ value: "2 tasks" }], onOpen: fn() } },
  { id: "step:tests", card: { kind: "step", name: "Write tests", activity: "not_started", said: "not started", ordinal: 3, facts: [{ value: "7 checks" }], onOpen: fn() } },
  { id: "step:handoff", card: { kind: "step", name: "Review the change", activity: "not_started", said: "not started", ordinal: 4, gate: "a person answers", onOpen: fn() } },
];

/** The same feature run the canvas draws, as a column. */
export const AFeatureRun: Story = { args: { label: "The run", rows } };

/** A loop has nothing to arc over in a column, so the step says where it goes back to. */
export const WithALoop: Story = {
  args: {
    label: "The run",
    rows: [
      rows[0]!,
      rows[1]!,
      { ...rows[6]!, returns: { toName: "Plan the change", label: "up to 5 passes" } },
      rows[7]!,
    ],
  },
};

/**
 * The claim the toggle rests on: the two arrangements say the same thing.
 *
 * **A `play`, because a still cannot say it.** The group cards are still here,
 * still under their step, and still carry what they carry on the canvas — a
 * stacked mode that dropped them would make the toggle a change of subject.
 */
export const SaysWhatTheCanvasSays: Story = {
  args: { label: "The run", rows },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("list", { name: "The run" })).toBeVisible();
    for (const name of ["Group 1, passed", "Group 2, passed", "Group 3, checking", "Group 4, pending"]) {
      await expect(canvas.getByRole("button", { name })).toBeVisible();
    }
    await expect(canvas.getByRole("button", { name: "Group 3, checking" })).toHaveTextContent("7 checks");
  },
};
