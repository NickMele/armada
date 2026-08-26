import type { Meta, StoryObj } from "@storybook/react-vite";
import { StepBar } from "./StepBar";

/**
 * One story per state a Job's progress can be in on a list row. The bar is
 * drawn at its 72px track width; in a Job row it fills the track the field run
 * gives it.
 */
const meta: Meta<typeof StepBar> = {
  title: "Compositions/Step bar",
  component: StepBar,
};
export default meta;

type Story = StoryObj<typeof StepBar>;

/** A Job that has not run. Every segment is remaining; none takes a hue. */
export const NotStarted: Story = {
  args: { total: 4, current: 0, label: "Not started, 4 steps" },
};

export const Running: Story = {
  args: { total: 4, current: 2, activity: "running", label: "Step 2 of 4" },
};

/**
 * Segment width says how long the workflow is, so the same bar at seven steps
 * reads differently from the same bar at four. That is the reading a fraction
 * cannot give down a column.
 */
export const RunningLongWorkflow: Story = {
  args: { total: 7, current: 5, activity: "running", label: "Step 5 of 7" },
};

export const AwaitingHuman: Story = {
  args: { total: 4, current: 3, activity: "awaiting_human", label: "Step 3 of 4" },
};

/**
 * A failed segment is loud, because at M1 a failed Check ends the Job and that
 * row is the reason a person opened the screen.
 */
export const Failed: Story = {
  args: { total: 4, current: 3, activity: "failed", label: "Step 3 of 4" },
};

/**
 * A killed segment is not. It keeps `--fg-default` and no hue: a human
 * decision rather than a system failure, and it must not read as an error.
 */
export const Killed: Story = {
  args: { total: 4, current: 2, activity: "killed", label: "Step 2 of 4" },
};

/**
 * A finished Job: every segment past, none hued. There is no current step left
 * for the hue to mark, which is what a completed workflow looks like.
 */
export const AllAdvanced: Story = {
  args: { total: 4, current: 5, activity: "advanced", label: "All 4 of 4 steps advanced" },
};

/**
 * The bar never pulses, and this is the story that says so: a running bar
 * beside a running bar, both static. Motion on a list row belongs to the
 * badge, in its fixed column.
 */
export const RunningNeverPulses: StoryObj = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)", width: "var(--sidebar-rail)" }}>
      <StepBar total={4} current={2} activity="running" label="Step 2 of 4" />
      <StepBar total={4} current={3} activity="running" label="Step 3 of 4" />
    </div>
  ),
};
