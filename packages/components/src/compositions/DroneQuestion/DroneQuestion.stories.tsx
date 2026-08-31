import type { Meta, StoryObj } from "@storybook/react-vite";

import { DroneQuestion } from "./DroneQuestion";

/**
 * A drone that does not know, asking rather than guessing.
 *
 * The two moves it had before were escalate — which stops the job, freezes the
 * step and holds the worktree until somebody moves it — and guess, whose output
 * on a job that dispatches other jobs is drones spending on work nobody asked
 * for. This is the third.
 *
 * **Every story is a closed set.** There is no text field on this surface and
 * no prop that could add one; free text goes through Redirect, which is what
 * the sentence under the control says. That is the line between this and the
 * orchestrator-with-sub-agents shape `docs/scope.md` records as abandoned: not
 * whether a person is involved, but whether a conversation is the medium.
 *
 * **No glyph.** `packages/icons/icons.toml` has no mark for a drone asking and
 * `file-question-mark` is reserved to the evidence family, so this draws none
 * rather than borrowing one.
 */
const meta: Meta<typeof DroneQuestion> = {
  title: "Compositions/Drone question",
  component: DroneQuestion,
};
export default meta;

type Story = StoryObj<typeof DroneQuestion>;

/**
 * The two-answer case, which is most of them. The question comes from building
 * the Focus milestone by hand on 30–31 Aug, where a guess would have been wrong
 * rather than merely presumptuous.
 */
export const TwoAnswers: Story = {
  args: {
    question:
      "The store schema needs a column before three of these jobs can run. Should that be its own job?",
    options: [
      {
        label: "Its own job",
        consequence:
          "Dispatch a migration job first and make the other three depend on it. Nothing else starts until it lands.",
      },
      {
        label: "Fold it in",
        consequence:
          "The first job that needs the column adds it. The other two may race it and one of them will have to wait anyway.",
      },
    ],
    waiting: "12m",
    onAnswer: () => {},
  },
};

/**
 * Four, which is the most a question may offer. Fleet refuses a fifth: the
 * whole value of asking rather than escalating is that a person answers in one
 * glance, and a list long enough to scroll is a list read badly at 11pm.
 */
export const FourAnswers: Story = {
  args: {
    question: "How should this milestone's work be split?",
    options: [
      {
        label: "By crate",
        consequence: "One job per crate that changes — six jobs, each with its own scope.",
      },
      {
        label: "By milestone step",
        consequence:
          "One job per step of the milestone as written — four jobs, two of which cross three crates.",
      },
      {
        label: "By side of the seam",
        consequence: "Two jobs: everything in Rust, and everything in Bridge.",
      },
      {
        label: "One job",
        consequence: "Do not split it. One drone works the whole milestone in one worktree.",
      },
    ],
    waiting: "2h",
    onAnswer: () => {},
  },
};

/**
 * Just asked. **No elapsed at all rather than `0m`** — a zero is a measurement
 * and this is the absence of one, and the two read differently to somebody
 * scanning for how long something has been sitting.
 */
export const JustAsked: Story = {
  args: {
    question: "Two issues in this milestone contradict each other on the gate. Which one holds?",
    options: [
      {
        label: "The Judge decides",
        consequence: "Gate the step on the Judge, and drop the human gate from it.",
      },
      {
        label: "A person decides",
        consequence: "Keep the human gate, and the step stops for somebody every time.",
      },
    ],
    onAnswer: () => {},
  },
};

/**
 * An answer already in flight. **The reason is on the surface**, because a
 * disabled control with no sentence beside it reads as an app that is broken
 * rather than one that is busy.
 */
export const Sending: Story = {
  args: {
    ...TwoAnswers.args,
    disabled: true,
    disabledNote: "That answer is already on its way to the drone.",
  } as Story["args"],
};

/**
 * Nothing live to send over. The same controls, a different sentence — and the
 * question is still legible, because reading what was asked does not need a
 * connection.
 */
export const NotConnected: Story = {
  args: {
    ...TwoAnswers.args,
    disabled: true,
    disabledNote: "Fleet is not connected, so nothing can be sent. The drone is still waiting.",
  } as Story["args"],
};
