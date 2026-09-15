import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { WorkflowDiagram, type WorkflowDiagramStep } from "./WorkflowDiagram";

/**
 * One story per shape the design session named: a straight line ending in a
 * stop for a person, and a loop back to an earlier step.
 */
const meta: Meta<typeof WorkflowDiagram> = {
  title: "Compositions/Workflow diagram",
  component: WorkflowDiagram,
};
export default meta;

type Story = StoryObj<typeof WorkflowDiagram>;

/** `bug.json`, as M1 ships it: a straight line, two Judge gates, and a stop
 * for a person at the step that reviews the change. */
const bug: WorkflowDiagramStep[] = [
  {
    id: "plan",
    label: "Plan the change",
    checks: [{ command: "plan_recorded" }],
    declarations: [{ label: "judge · 2 criteria" }],
    gate: "auto",
    gateLabel: "the checks decide, unless the Judge objects",
  },
  {
    id: "implement",
    label: "Implement",
    checks: [{ command: "every_manifest_check" }, { command: "diff_nonempty" }],
    declarations: [{ label: "judge · 3 criteria · gaming check" }],
    gate: "auto",
    gateLabel: "the checks decide, unless the Judge objects",
  },
  {
    id: "handoff",
    label: "Review the change",
    gate: "person",
    gateLabel: "a person answers",
  },
];

export const Bug: Story = {
  args: { steps: bug },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Plan the change")).toBeVisible();
    await expect(canvas.getByText("Implement")).toBeVisible();
    await expect(canvas.getByText("Review the change")).toBeVisible();
    await expect(canvas.getAllByText("the checks decide, unless the Judge objects")).toHaveLength(2);
    await expect(canvas.getByText("a person answers")).toBeVisible();
  },
};

/** `epic.json`: a loop back to the first step, capped at 5 passes, with a
 * person stopping the Job both before the split is dispatched and after the
 * wave rolls up. */
const epic: WorkflowDiagramStep[] = [
  {
    id: "plan",
    label: "Plan the wave",
    checks: [{ command: "artifact_exists" }, { command: "plan_recorded" }],
    declarations: [{ label: "judge · 2 criteria" }],
    gate: "person",
    gateLabel: "a person answers",
  },
  {
    id: "dispatch",
    label: "Dispatch the wave",
    checks: [{ command: "artifact_exists" }],
    gate: "auto",
  },
  {
    id: "roll_up",
    label: "Roll up the wave",
    checks: [{ command: "artifact_exists" }],
    gate: "person",
    gateLabel: "a person answers",
    loop: { to: "plan", label: "up to 5 passes" },
  },
];

export const Epic: Story = {
  args: { steps: epic },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Plan the wave")).toBeVisible();
    await expect(canvas.getByText("Roll up the wave")).toBeVisible();
    await expect(canvas.getByText("up to 5 passes")).toBeVisible();
  },
};
