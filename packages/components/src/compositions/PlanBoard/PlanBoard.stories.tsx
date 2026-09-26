import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { PlanBoard, type PlanBoardGroup } from "./PlanBoard";

/**
 * One story per state the plan can be read in: before anything runs, mid-run
 * with a group that failed its boundary, and a finished task a later one
 * edited. The words are the caller's, as they are in the app.
 */
const meta: Meta<typeof PlanBoard> = {
  title: "Compositions/Plan board",
  component: PlanBoard,
};
export default meta;

type Story = StoryObj<typeof PlanBoard>;

const APPROACH =
  "One read of everything running, then the stat's words, then the panel, then what holds " +
  "the next Drone back.";

const RUST = ["build", "test", "acceptance", "format"];
const BRIDGE = [
  "test",
  "typecheck",
  "bridge_build",
  "storybook",
  "desktop_test",
  "screens_test",
  "components_test",
];

function planned(): PlanBoardGroup[] {
  return [
    {
      id: "g1",
      ordinal: 1,
      state: "pending",
      says: "not started",
      scope: ["crates/api/src/running.rs", "crates/fleet/src/running.rs"],
      tasks: [
        {
          id: "T1",
          title: "Serve one read of everything running",
          mark: "open",
          tier: "difficult",
          model: "opus",
        },
        {
          id: "T2",
          title: "Send an event when what is running changes",
          mark: "open",
          tier: "medium",
          model: "sonnet",
        },
      ],
      boundarySays: "4 checks will run at this boundary",
      checks: RUST,
      testsSay: "1 test runs at this boundary",
      tests: [{ id: "c-api", spec: "crates/api/src/tests/running.rs", reads: "owed" }],
    },
    {
      id: "g3",
      ordinal: 2,
      state: "pending",
      says: "not started",
      scope: ["packages/screens/src/Running.tsx", "packages/screens/src/running-rows.tsx"],
      concurrentSays: "both tasks run at the same time",
      tasks: [
        {
          id: "T5",
          title: "Draw what is running, in four lists",
          mark: "open",
          tier: "difficult",
          model: "opus",
          besideSays: "runs beside T6",
        },
        {
          id: "T6",
          title: "Open a Drone's Job from its row",
          mark: "open",
          tier: "medium",
          model: "sonnet",
          besideSays: "runs beside T5",
        },
      ],
      boundarySays: "7 checks will run at this boundary",
      checks: BRIDGE,
      testsSay: "1 test runs at this boundary",
      tests: [
        { id: "c-board", spec: "packages/screens/src/Board.test.tsx", reads: "not covered" },
      ],
    },
  ];
}

export const Planned: Story = {
  args: { approach: APPROACH, groups: planned(), onOpenTask: fn() },
};

export const GroupFailed: Story = {
  args: {
    approach: APPROACH,
    groups: [
      {
        ...planned()[1]!,
        state: "retrying",
        says: "failed at its checks",
        retrySays: "second run",
        tasks: [
          { ...planned()[1]!.tasks[0]!, mark: "done", spentSays: "27 turns · $1.90" },
          {
            ...planned()[1]!.tasks[1]!,
            mark: "failed",
            spentSays: "15 turns · $0.72",
            failedReason: "The row's press opened the Board rather than the Job",
          },
        ],
        boundarySays: "7 checks ran at this boundary",
        checksFailed: ["screens_test"],
      },
    ],
  },
};

export const DoneTouchedLater: Story = {
  args: {
    approach: APPROACH,
    groups: [
      {
        ...planned()[1]!,
        state: "passed",
        says: "passed",
        commit: "b81c3e4",
        tasks: [
          { ...planned()[1]!.tasks[0]!, mark: "done", spentSays: "27 turns · $1.90" },
          {
            ...planned()[1]!.tasks[1]!,
            mark: "done",
            spentSays: "15 turns · $0.72",
            touchedSays: "touched later · T7",
          },
        ],
        boundarySays: "7 checks ran at this boundary",
      },
    ],
  },
};

export const OrderContradictsScope: Story = {
  args: {
    approach: APPROACH,
    groups: planned(),
    clashes: [
      {
        path: "packages/screens/src/running-rows.tsx",
        says: "group 2 (T6) and group 4 (T7)",
      },
    ],
  },
};

/**
 * The row is one control and the task's id is part of its name, so a person
 * reaching it by keyboard hears which task they are opening. Verified against
 * a board rendered with `onOpenTask` absent, where nothing is pressable.
 */
export const OpeningATask: Story = {
  args: { ...Planned.args, openTaskId: "T5" } as Story["args"],
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: /T1/ }));
    await expect(args.onOpenTask).toHaveBeenCalledWith("T1");
    await expect(canvas.getByRole("button", { name: /T5/ })).toHaveAttribute(
      "aria-current",
      "true",
    );
    await expect(canvas.getByRole("button", { name: /T1/ })).not.toHaveAttribute("aria-current");
  },
};

/** What the board offers while the plan is still a question. `#1552`. */
const ASKS = [
  { id: "move_up", label: "Move up" },
  { id: "move_down", label: "Move down" },
  { id: "remove", label: "Remove" },
];

/**
 * The plan at its gate, where it may still be argued with. Every group offers
 * the same three controls, and the first group's Move up is drawn off rather
 * than left out — a card whose controls change place as it moves is a card a
 * person has to read again.
 */
export const WaitingAtItsGate: Story = {
  args: {
    approach: APPROACH,
    askable: true,
    groups: planned().map((group, at) => ({
      ...group,
      asks: ASKS.map((ask) =>
        ask.id === "move_up" && at === 0 ? { ...ask, disabled: true } : ask,
      ),
    })),
    onOpenTask: fn(),
    onAsk: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const first = canvas.getByRole("group", { name: "Ask about group 1" });
    await expect(within(first).getByRole("button", { name: "Move up" })).toBeDisabled();
    await userEvent.click(within(first).getByRole("button", { name: "Remove" }));
    await expect(args.onAsk).toHaveBeenCalledWith("g1", "remove");
  },
};

/**
 * One ask is out, so every one of them is off — a second press while the first
 * is unanswered would be two asks about one plan with no order between them.
 */
export const AnAskIsOut: Story = {
  args: { ...WaitingAtItsGate.args, askPending: true } as Story["args"],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const second = canvas.getByRole("group", { name: "Ask about group 2" });
    await expect(within(second).getByRole("button", { name: "Move down" })).toBeDisabled();
  },
};
