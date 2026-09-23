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

/** One group's rows: the group under the step that wrote it, then its tasks. */
function group(
  ordinal: number,
  said: string,
  activity: "advanced" | "running" | "not_started",
  worked: boolean,
  tasks: string[],
): WorkflowStackedRow[] {
  return [
    {
      id: `group:g${ordinal}`,
      under: "step:plan",
      depth: 1,
      ...(worked ? { worked: "Implement" } : {}),
      card: {
        kind: "group",
        name: `Group ${ordinal}`,
        activity,
        said,
        facts: [{ value: "2 tasks" }, { value: "7 checks" }],
        onOpen: fn(),
      },
    },
    ...tasks.map((title, k) => ({
      id: `task:g${ordinal}-${k}`,
      under: `group:g${ordinal}`,
      depth: 2 as const,
      card: {
        kind: "task" as const,
        name: title,
        activity: worked ? ("advanced" as const) : ("not_started" as const),
        said: worked ? "done" : "open",
        facts: [{ value: `T${(ordinal - 1) * 2 + k + 1}` }, { value: "1 file" }],
        onOpen: fn(),
      },
    })),
  ];
}

const rows: WorkflowStackedRow[] = [
  { id: "step:plan", card: { kind: "step", name: "Plan the change", activity: "advanced", said: "advanced", ordinal: 1, facts: [{ value: "4 groups" }, { value: "2 criteria" }], onOpen: fn() } },
  ...group(1, "passed", "advanced", true, ["Serve one read of everything running", "Send an event when what is running changes"]),
  ...group(2, "passed", "advanced", true, ["Reword the stat to one running beside the most", "Say the same words on the Board and in Settings"]),
  ...group(3, "checking", "running", true, ["Draw what is running, in four lists", "Open a Drone's Job from its row"]),
  ...group(4, "pending", "not_started", false, ["Say what holds the next Drone back", "Four stories: nothing out, one Drone, three at once, and the most"]),
  { id: "step:implement", card: { kind: "step", name: "Implement", activity: "running", said: "running", ordinal: 2, current: true, facts: [{ value: "11 checks" }], onOpen: fn() } },
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
      { ...rows[rows.length - 3]!, returns: { toName: "Plan the change", label: "up to 5 passes" } },
      rows[rows.length - 2]!,
      rows[rows.length - 1]!,
    ],
  },
};

/**
 * The claim the toggle rests on: the two arrangements say the same thing.
 *
 * **A `play`, because a still cannot say it.** The graph's second edge into a
 * group is a line of words here, and only the words can carry it — a column
 * that dropped them would make the toggle a change of subject.
 */
export const SaysWhatTheCanvasSays: Story = {
  args: { label: "The run", rows },
  play: async ({ canvas }) => {
    const run = canvas.getByRole("list", { name: "The run" });
    await expect(run).toBeVisible();
    // Every group is here, under the step that wrote it, with its tasks.
    for (const name of ["Group 1, passed", "Group 2, passed", "Group 3, checking", "Group 4, pending"]) {
      await expect(canvas.getByRole("button", { name })).toBeVisible();
    }
    await expect(canvas.getByRole("button", { name: "Draw what is running, in four lists, done" })).toBeVisible();
    // A worked group says which step worked it; an unworked one says nothing.
    const worked = canvas.getAllByText(/^worked at Implement$/);
    expect(worked).toHaveLength(3);
    await expect(canvas.getByRole("button", { name: "Group 3, checking" })).toHaveTextContent("7 checks");
  },
};
