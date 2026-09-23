import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { WorkflowStepCard } from "./WorkflowStepCard";

const meta: Meta<typeof WorkflowStepCard> = {
  title: "Compositions/Workflow step card",
  component: WorkflowStepCard,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WorkflowStepCard>;

/** A step nothing has entered. Its number stands in for a glyph. */
export const NotStarted: Story = {
  args: {
    kind: "step",
    name: "Write tests",
    activity: "not_started",
    said: "not started",
    ordinal: 3,
    facts: [{ value: "7 checks" }, { value: "1 criterion" }],
    onOpen: fn(),
  },
};

/** The step the Job is on. One mark pulses per screen and this is it. */
export const Running: Story = {
  args: {
    kind: "step",
    name: "Implement",
    activity: "running",
    said: "running",
    ordinal: 2,
    current: true,
    facts: [{ value: "4 groups" }, { value: "11 checks" }, { value: "attempt 2" }],
    onOpen: fn(),
  },
};

/** A step waiting on a person says what it is waiting for. */
export const WaitingOnYou: Story = {
  args: {
    kind: "step",
    name: "Review the change",
    activity: "awaiting_human",
    said: "awaiting review",
    ordinal: 4,
    gate: "a person answers",
    facts: [{ value: "delivers" }],
    onOpen: fn(),
  },
};

/** A step that advanced, with what its Checks came to. */
export const Advanced: Story = {
  args: {
    kind: "step",
    name: "Plan the change",
    activity: "advanced",
    said: "advanced",
    ordinal: 1,
    facts: [{ value: "2 of 2 passed", named: "passed" }],
    onOpen: fn(),
  },
};

/** A step that stopped, and one whose verdict refused it — three kinds of stopped, never alike. */
export const Stopped: Story = {
  args: {
    kind: "step",
    name: "Implement",
    activity: "stopped",
    said: "stopped",
    ordinal: 2,
    facts: [{ value: "typecheck failed", named: "failed" }],
    onOpen: fn(),
  },
};

/** A group inside the step, at its boundary's Checks. */
export const Group: Story = {
  args: {
    kind: "group",
    name: "Group 3",
    activity: "running",
    said: "checking",
    facts: [{ value: "2 tasks" }, { value: "7 checks" }],
    onOpen: fn(),
  },
};

/** A task inside the group, narrower again. Its id is the first fact. */
export const Task: Story = {
  args: {
    kind: "task",
    name: "Draw what is running, in four lists",
    activity: "running",
    said: "working",
    facts: [{ value: "T5" }, { value: "1 file" }, { value: "14 turns" }],
    onOpen: fn(),
  },
};

/** The one being read. */
export const Open: Story = {
  args: { ...Group.args, selected: true } as Story["args"],
};

/**
 * A card with nowhere to open is not a control.
 *
 * **A `play`, because a still cannot tell a button from a span** — and the
 * stacked run draws cards a caller has given no inspector to, where a focus
 * stop that does nothing is worse than no focus stop.
 */
export const NotAControl: Story = {
  args: {
    kind: "step",
    name: "Summarise",
    activity: "not_started",
    said: "not started",
    ordinal: 2,
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button")).toBeNull();
    await expect(canvas.getByRole("group", { name: "Summarise, not started" })).toBeVisible();
  },
};
