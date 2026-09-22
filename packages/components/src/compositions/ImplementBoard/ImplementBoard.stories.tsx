import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { ImplementBoard, type ImplementGroup } from "./ImplementBoard";

const meta: Meta<typeof ImplementBoard> = {
  title: "Compositions/Implement board",
  component: ImplementBoard,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof ImplementBoard>;

const NAMES = ["typecheck", "format", "screens_test", "components_test", "bridge_build", "storybook", "acceptance"];
const NO_CASES = "Fleet does not serve the cases a boundary owes yet, so none is drawn here.";

const ORDER =
  "One group at a time. No task of the next group starts while this one is being checked, " +
  "and a task reads done only once its group has gone green.";

const passed = (ordinal: number, commit: string, tasks: ImplementGroup["tasks"]): ImplementGroup => ({
  id: `g${ordinal}`,
  ordinal,
  state: "passed",
  says: "passed",
  shapeSays: `${tasks.length} tasks, one after another`,
  commit,
  tasks,
  boundary: {
    says: "7 checks ran at this boundary",
    checks: NAMES.map((name) => ({ name, reads: "passed" as const })),
    verdictSays: "all 7 passed",
    verdictNamed: "passed",
    commit,
    testsAbsent: NO_CASES,
  },
});

const GROUP_ONE = passed(1, "4c1b9d2", [
  {
    id: "T1",
    title: "Serve one read of everything running",
    mark: "done",
    says: "difficult · opus · its own agent",
    spentSays: "34 turns · ~$2.40",
  },
  {
    id: "T2",
    title: "Send an event when what is running changes",
    mark: "done",
    says: "medium · sonnet · its own agent",
    spentSays: "12 turns · ~$0.64",
  },
]);

const CONCURRENT: ImplementGroup = {
  id: "g3",
  ordinal: 3,
  state: "joining",
  says: "joining its work",
  shapeSays: "2 tasks, at the same time",
  tasks: [
    {
      id: "T5",
      title: "Draw what is running, in four lists",
      mark: "done",
      says: "difficult · opus · its own agent",
      spentSays: "27 turns · ~$1.90",
      besideSays: "runs beside T6",
    },
    {
      id: "T6",
      title: "Open a Drone's Job from its row",
      mark: "done",
      says: "medium · sonnet · its own agent",
      spentSays: "15 turns · ~$0.72",
      besideSays: "runs beside T5",
      touchedSays: "touched later · T7",
    },
  ],
  boundary: {
    says: "7 checks will run at this boundary",
    checks: NAMES.map((name) => ({ name, reads: "not run" as const })),
    testsAbsent: NO_CASES,
  },
};

const FAILED: ImplementGroup = {
  ...CONCURRENT,
  state: "retrying",
  says: "failed at its checks, running again",
  tasks: [
    CONCURRENT.tasks[0]!,
    {
      ...CONCURRENT.tasks[1]!,
      mark: "failed",
      failedReason: "The row's press opened the Board rather than the Job",
    },
  ],
  boundary: {
    says: "7 checks ran at this boundary",
    checks: NAMES.map((name) => ({ name, reads: name === "screens_test" ? "failed" : "passed" })),
    verdictSays: "screens_test failed",
    verdictNamed: "failed",
    retrySays: "second run",
    stopsSays: "No task of group 4 starts until this boundary passes.",
    toldNext: "1 of 1384 failed: the Drones row opened the Board",
    testsAbsent: NO_CASES,
  },
};

const PENDING: ImplementGroup = {
  id: "g4",
  ordinal: 4,
  state: "pending",
  says: "not started",
  shapeSays: "2 tasks, one after another",
  tasks: [
    { id: "T7", title: "Say what holds the next Drone back", mark: "open", says: "medium · sonnet · its own agent" },
    { id: "T8", title: "Four stories: nothing out, one Drone, three at once", mark: "open", says: "medium · sonnet · its own agent" },
  ],
  boundary: {
    says: "7 checks will run at this boundary",
    checks: NAMES.map((name) => ({ name, reads: "not run" as const })),
    testsAbsent: NO_CASES,
  },
};

/** Fan out, then join: two tasks at once, with the group's work being brought together. */
export const FanOutThenJoin: Story = {
  args: {
    stepName: "Implement",
    orderSays: ORDER,
    groups: [GROUP_ONE, CONCURRENT, PENDING],
    openGroups: ["g3"],
    onOpenGroup: fn(),
    onOpenTask: fn(),
  },
};

/** A boundary that broke, with the group behind it held. */
export const AGroupFailed: Story = {
  args: {
    stepName: "Implement",
    orderSays: ORDER,
    groups: [GROUP_ONE, FAILED, PENDING],
    openGroups: ["g3"],
    onOpenGroup: fn(),
    onOpenTask: fn(),
  },
};

/**
 * A group that passed folds to one line, and opening it is a press.
 *
 * **A `play`, because the rule is about what is not drawn.** A folded group
 * still says what it got through — its state and the commit it left — so
 * reading a run is not opening every group in turn.
 */
export const AFoldedGroupStillSaysWhatItGotThrough: Story = {
  args: {
    stepName: "Implement",
    orderSays: ORDER,
    groups: [GROUP_ONE, CONCURRENT, PENDING],
    openGroups: ["g3"],
    onOpenGroup: fn(),
    onOpenTask: fn(),
  },
  play: async ({ args, canvas }) => {
    const head = canvas.getByRole("button", { name: /^Group 1/ });
    await expect(head).toHaveAttribute("aria-expanded", "false");
    await expect(head).toHaveTextContent("passed");
    await expect(head).toHaveTextContent("4c1b9d2");
    await userEvent.click(head);
    await expect(args.onOpenGroup).toHaveBeenCalledWith("g1");
  },
};
